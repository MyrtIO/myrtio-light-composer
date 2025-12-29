use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Instant, Timer};

#[cfg(feature = "log")]
use esp_println::println;

use crate::LedDriver;
use crate::bounds::{RenderingBounds, bounded};
use crate::color::{Rgb, kelvin_to_rgb};
use crate::filter::{ColorCorrection, FilterProcessor, FilterProcessorConfig};
use crate::effect::{EffectId, EffectSlot};
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
    current_effect: EffectSlot,
    brightness: u8,
}

/// Configuration for the light engine
#[derive(Clone)]
pub struct LightEngineConfig {
    pub effect: EffectId,
    pub bounds: RenderingBounds,
    pub filters: FilterProcessorConfig,
    pub timings: TransitionTimings,
    pub brightness: u8,
    pub color: Rgb,
}

/// Represents a user intent to change the light state.
#[derive(Debug, Clone)]
pub struct LightStateIntent {
    pub power: Option<bool>,
    pub brightness: Option<u8>,
    pub color: Option<Rgb>,
    pub color_temperature: Option<u16>,
    pub effect_id: Option<EffectId>,
}

pub enum LightIntent {
    StateChange(LightStateIntent),
    BoundsChange(RenderingBounds),
    ColorCorrectionChange(Rgb),
    MinimalBrightnessChange(u8),
    BrightnessScaleChange(u8),
}

/// Type alias for intent sender
pub type IntentSender<const SIZE: usize> =
    Sender<'static, CriticalSectionRawMutex, LightIntent, SIZE>;

/// Type alias for intent receiver  
pub type IntentReceiver<const SIZE: usize> =
    Receiver<'static, CriticalSectionRawMutex, LightIntent, SIZE>;

/// Type alias for the intent channel
pub type IntentChannel<const SIZE: usize> = Channel<CriticalSectionRawMutex, LightIntent, SIZE>;

/// Light Engine - the main orchestrator
pub struct LightEngine<D: LedDriver, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize> {
    // External dependencies and configuration
    driver: D,
    intents: IntentReceiver<INTENT_CHANNEL_SIZE>,
    timings: TransitionTimings,
    bounds: RenderingBounds,

    // Internal state
    state: LightState,
    next_frame: Instant,
    stack: OperationStack<10>,

    // Internal dependencies
    filters: FilterProcessor,
}

impl<D: LedDriver, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize>
    LightEngine<D, MAX_LEDS, INTENT_CHANNEL_SIZE>
{
    /// Create a new light engine with command channel
    ///
    /// Returns the engine and a sender for commands.
    pub fn new(
        driver: D,
        intents: IntentReceiver<INTENT_CHANNEL_SIZE>,
        config: &LightEngineConfig,
    ) -> Self {
        let now = Instant::now();
        Self {
            driver,
            intents,
            timings: config.timings,
            bounds: config.bounds,
            state: LightState {
                color: config.color,
                current_effect: config.effect.to_slot(config.color),
                brightness: config.brightness,
            },
            next_frame: now,
            stack: OperationStack::new(),
            filters: FilterProcessor::new(&config.filters),
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

        self.filters.tick(now);

        let mut frame = [Rgb::default(); MAX_LEDS];
        let leds = bounded(&mut frame, self.bounds);

        self.state.current_effect.render(now, leds);
        self.filters.apply(leds);

        Timer::at(self.next_frame).await;
        self.driver.write(&frame);
    }

    /// Process pending commands from the channel (non-blocking)
    fn process_intents(&mut self) {
        while let Ok(intent) = self.intents.try_receive() {
            match intent {
                LightIntent::StateChange(intent) => {
                    if let Some(mode_id) = intent.effect_id {
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
                LightIntent::BoundsChange(bounds) => {
                    self.bounds = bounds;
                }
                LightIntent::ColorCorrectionChange(color_correction) => {
                    self.filters.color_correction = Some(ColorCorrection::new(color_correction));
                }
                LightIntent::MinimalBrightnessChange(brightness) => {
                    self.filters.brightness.set_min_brightness(brightness);
                }
                LightIntent::BrightnessScaleChange(scale) => {
                    self.filters.brightness.set_scale(scale);
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
                self.filters
                    .brightness
                    .set(brightness, self.timings.brightness, now);
            }
            Operation::SetColor(color) => {
                self.state
                    .current_effect
                    .set_color(color, self.timings.color_change, now);
            }
            Operation::PowerOff => {
                self.filters
                    .brightness
                    .set_uncorrected(0, self.timings.brightness, now);
            }
            Operation::PowerOn => {
                self.filters
                    .brightness
                    .set(self.state.brightness, self.timings.brightness, now);
            }
            Operation::SwitchEffect(_mode) => {
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
                !self.filters.brightness.is_transitioning()
            }
            Operation::SetColor(_) => !self.state.current_effect.is_transitioning(),
            Operation::SwitchEffect(_) => true,
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
            Operation::SwitchEffect(mode) => {
                self.set_effect(mode);
            }
            Operation::PowerOff | Operation::PowerOn => {
                // This commands does not change the state
            }
        }

        self.stack.pop()
    }

    fn set_effect(&mut self, effect: EffectId) {
        self.state.current_effect = effect.to_slot(self.state.color);
        self.state.current_effect.reset();
    }
}
