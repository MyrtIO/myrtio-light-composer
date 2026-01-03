use embassy_time::{Duration, Instant};

#[cfg(feature = "esp32-log")]
use esp_println::println;

use crate::bounds::{RenderingBounds, bounded};
use crate::color::Rgb;
use crate::effect::{EffectId, EffectSlot};
use crate::filter::{ColorCorrection, Filter, FilterProcessor, FilterProcessorConfig};
use crate::intent_processor::{IntentEffects, IntentProcessor, IntentReceiver};
use crate::operation::{Operation, OperationStack};

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
pub struct LightState {
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

/// Light Engine - the main orchestrator
pub struct Renderer<'a, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize> {
    // External dependencies and configuration
    intent_processor: IntentProcessor<'a, INTENT_CHANNEL_SIZE>,
    timings: TransitionTimings,
    bounds: RenderingBounds,

    // Internal state
    state: LightState,
    stack: OperationStack<10>,
    frame_buffer: [Rgb; MAX_LEDS],

    // Internal dependencies
    filters: FilterProcessor,
}

impl<'a, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize>
    Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE>
{
    /// Create a new light engine with command channel
    ///
    /// Returns the engine and a sender for commands.
    pub fn new(intents: IntentReceiver<'a, INTENT_CHANNEL_SIZE>, config: &LightEngineConfig) -> Self {
        Self {
            intent_processor: IntentProcessor::new(intents),
            frame_buffer: [Rgb::default(); MAX_LEDS],
            timings: config.timings,
            bounds: config.bounds,
            state: LightState {
                color: config.color,
                current_effect: config.effect.to_slot(config.color),
                brightness: config.brightness,
            },
            stack: OperationStack::new(),
            filters: FilterProcessor::new(&config.filters),
        }
    }

    /// Process one frame
    ///
    /// This is the main render loop step. Call this continuously.
    pub fn render(&mut self, now: Instant) -> &[Rgb] {
        self.process_intents();
        self.process_operations(now);

        self.filters.tick(now);

        let frame = bounded(&mut self.frame_buffer, self.bounds);
        self.state.current_effect.render(now, frame);

        if self.state.current_effect.requires_precise_colors() {
            self.filters.color_correction.apply(frame);
        }
        self.filters.brightness.apply(frame);

        frame
    }

    /// Process pending intents from the channel (non-blocking)
    fn process_intents(&mut self) {
        let effects = self
            .intent_processor
            .process_pending(&mut self.stack, self.state.brightness);

        self.apply_effects(&effects);
    }

    /// Apply side effects from intent processing
    fn apply_effects(&mut self, effects: &IntentEffects) {
        if let Some(bounds) = effects.bounds {
            self.bounds = bounds;
        }

        if let Some(color_correction) = effects.color_correction {
            self.filters.color_correction = ColorCorrection::new(color_correction);
        }

        if let Some(brightness_range) = effects.brightness_range {
            self.filters.brightness.set_min_brightness(brightness_range.min());
            self.filters.brightness.set_scale(brightness_range.max());
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
            Operation::SwitchEffect(_effect) => {
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
            Operation::SwitchEffect(effect) => {
                self.set_effect(effect);
            }
            Operation::PowerOff | Operation::PowerOn => {
                // This commands does not change the state
            }
        }

        self.stack.pop()
    }

    /// Set new effect by id
    fn set_effect(&mut self, effect: EffectId) {
        self.state.current_effect = effect.to_slot(self.state.color);
        self.state.current_effect.reset();
    }
}
