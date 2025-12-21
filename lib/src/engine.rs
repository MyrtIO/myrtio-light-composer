use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Instant, Timer};

#[cfg(feature = "log")]
use esp_println::println;

use crate::bounds::{bounded, RenderingBounds};
use crate::LedDriver;
use crate::color::{Rgb, kelvin_to_rgb};
use crate::effect::{EffectProcessor, EffectProcessorConfig};
use crate::mode::{ModeId, ModeSlot};
use crate::operation::{Operation, OperationStack};

const DEFAULT_FPS: u32 = 90;
const DEFAULT_FRAME_DURATION_MS: u64 = 1000 / DEFAULT_FPS as u64;
const DEFAULT_FRAME_DURATION: Duration = Duration::from_millis(DEFAULT_FRAME_DURATION_MS);

/// Maximum drift before resetting frame timing (2 frames worth)
/// If we fall behind by more than this, we skip the backlog instead of catching up
const MAX_DRIFT: Duration = Duration::from_millis(2 * DEFAULT_FRAME_DURATION_MS);

/// Configuration for effect transitions
#[derive(Clone, Copy)]
pub struct TransitionTimings {
    /// Duration of fade-out phase
    pub fade_out: Duration,
    /// Duration of fade-in phase
    pub fade_in: Duration,
    /// Duration of color change
    pub color_change: Duration,
    /// Duration of brightness change
    pub brightness: Duration,
}

#[derive(Debug, Clone)]
struct LightState {
    color: Rgb,
    current_mode: ModeSlot,
    pending_mode: Option<ModeSlot>,
    brightness: u8,
}

/// Configuration for the light engine
#[derive(Clone)]
pub struct LightEngineConfig {
    pub mode: ModeId,
    pub bounds: RenderingBounds,
    pub effects: EffectProcessorConfig,
    pub timings: TransitionTimings,
    pub brightness: u8,
    pub color: Rgb,
}

/// Represents a user intent to change the light state.
#[derive(Debug, Clone)]
pub struct LightIntent {
    pub power: Option<bool>,
    pub brightness: Option<u8>,
    pub color: Option<Rgb>,
    pub color_temperature: Option<u16>,
    pub mode_id: Option<ModeId>,
}

const INTENT_CHANNEL_SIZE: usize = 4;

/// Type alias for intent sender
pub type IntentSender = Sender<'static, CriticalSectionRawMutex, LightIntent, INTENT_CHANNEL_SIZE>;

/// Type alias for intent receiver  
pub type IntentReceiver =
    Receiver<'static, CriticalSectionRawMutex, LightIntent, INTENT_CHANNEL_SIZE>;

/// Type alias for the intent channel
pub type IntentChannel = Channel<CriticalSectionRawMutex, LightIntent, INTENT_CHANNEL_SIZE>;

/// Light Engine - the main orchestrator
pub struct LightEngine<D: LedDriver, const N: usize> {
    // External dependencies and configuration
    driver: D,
    intents: IntentReceiver,
    timings: TransitionTimings,
    bounds: RenderingBounds,

    // Internal state
    state: LightState,
    next_frame: Instant,
    stack: OperationStack<10>,

    // Internal dependencies
    effects: EffectProcessor,
}

impl<D: LedDriver, const N: usize> LightEngine<D, N> {
    /// Create a new light engine with command channel
    ///
    /// Returns the engine and a sender for commands.
    pub fn new(driver: D, intents: IntentReceiver, config: &LightEngineConfig) -> Self {
        let now = Instant::now();
        Self {
            driver,
            intents,
            timings: config.timings,
            bounds: config.bounds,
            state: LightState {
                color: config.color,
                current_mode: config.mode.to_mode_slot(config.color),
                pending_mode: None,
                brightness: config.brightness,
            },
            next_frame: now,
            stack: OperationStack::new(),
            effects: EffectProcessor::new(&config.effects),
        }
    }

    /// Process one frame
    ///
    /// This is the main render loop step. Call this continuously.
    pub async fn tick(&mut self) {
        let now = Instant::now();

        // Drift correction: if we've fallen too far behind, reset to now
        // This prevents catch-up bursts after long stalls
        if now > self.next_frame + MAX_DRIFT {
            self.next_frame = now;
        }
        self.next_frame += DEFAULT_FRAME_DURATION;

        self.process_intents();
        self.process_operations(now);

        self.effects.tick(now);

        let mut frame = [Rgb::default(); N];
        let leds = bounded(&mut frame, self.bounds);

        self.state.current_mode.render(now, leds);
        self.effects.apply(leds);

        Timer::at(self.next_frame).await;
        self.driver.write(&frame);
    }

    /// Process pending commands from the channel (non-blocking)
    fn process_intents(&mut self) {
        while let Ok(intent) = self.intents.try_receive() {
            if let Some(mode_id) = intent.mode_id {
                let _ = self.stack.push_mode(mode_id, self.state.brightness);
            }

            if let Some(brightness) = intent.brightness {
                let _ = self.stack.push_brightness(brightness);
            }

            if let Some(color) = intent.color {
                let _ = self.stack.push_color(color);
            } else if let Some(temp_kelvin) = intent.color_temperature {
                let color = kelvin_to_rgb(temp_kelvin);
                let _ = self.stack.push_color(color);
            }

            if let Some(power) = intent.power {
                if power {
                    let _ = self.stack.push_power_on();
                } else {
                    let _ = self.stack.push_power_off();
                }
            }
        }
    }

    /// Process the next operation from the stack
    fn process_operations(&mut self, now: Instant) {
        let Some(next) = self.process_current_operation() else {
            return;
        };
        // Start the transition for the current operation
        match next {
            Operation::SetBrightness(brightness) => {
                self.effects
                    .brightness
                    .set(brightness, self.timings.brightness, now);
            }
            Operation::SetColor(color) => {
                self.state
                    .current_mode
                    .set_color(color, self.timings.color_change, now);
            }
            Operation::PowerOff => {
                self.effects
                    .brightness
                    .set_uncorrected(0, self.timings.brightness, now);
            }
            Operation::PowerOn => {
                self.effects
                    .brightness
                    .set(self.state.brightness, self.timings.brightness, now);
            }
            Operation::SwitchMode(_mode) => {
                // This command changes instantly
            }
        }
    }

    /// Process the current operation from the stack
    ///
    /// Returns the next operation to process
    fn process_current_operation(&mut self) -> Option<Operation> {
        let current = self.stack.current()?;
        let is_complete = match current {
            Operation::SetBrightness(_) | Operation::PowerOff | Operation::PowerOn => {
                !self.effects.brightness.is_transitioning()
            }
            Operation::SetColor(_) => !self.state.current_mode.is_transitioning(),
            Operation::SwitchMode(_) => true,
        };
        if !is_complete {
            return None;
        }
        // Apply the operation to the state
        match current {
            Operation::SetBrightness(brightness) => {
                self.state.brightness = brightness;
            }
            Operation::SetColor(color) => {
                self.state.color = color;
            }
            Operation::SwitchMode(mode) => {
                self.set_mode(mode);
            }
            Operation::PowerOff | Operation::PowerOn => {
                // This commands does not change the state
            }
        }

        self.stack.pop()
    }

    fn set_mode(&mut self, mode: ModeId) {
        let slot = mode.to_mode_slot(self.state.color);
        self.state.current_mode = slot;
        self.state.current_mode.reset();
        self.state.pending_mode = None;
    }
}
