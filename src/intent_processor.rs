//! Intent processing module
//!
//! Handles conversion of user intents to renderer operations.

use crate::bounds::RenderingBounds;
use crate::channel::{Channel, Receiver, Sender};
use crate::color::{Rgb, kelvin_to_rgb};
use crate::effect::EffectId;
use crate::filter::BrightnessRange;
use crate::operation::OperationStack;

/// Represents a user intent to change the light state.
#[derive(Debug, Clone, Default)]
pub struct LightStateIntent {
    pub power: Option<bool>,
    pub brightness: Option<u8>,
    pub color: Option<Rgb>,
    pub color_temperature: Option<u16>,
    pub effect_id: Option<EffectId>,
}

/// Intent to change light state or settings
#[derive(Debug, Clone)]
pub enum LightChangeIntent {
    /// Change the light state (power, brightness, color, effect)
    State(LightStateIntent),
    /// Change the rendering bounds
    Bounds(RenderingBounds),
    /// Change the color correction
    ColorCorrection(Rgb),
    /// Change the brightness range (min/scale)
    BrightnessRange(BrightnessRange),
}

/// Side effects from processing intents that the renderer should apply
#[derive(Debug, Clone, Default)]
pub struct IntentEffects {
    /// New bounds to apply
    pub bounds: Option<RenderingBounds>,
    /// New color correction to apply
    pub color_correction: Option<Rgb>,
    /// New brightness range to apply
    pub brightness_range: Option<BrightnessRange>,
}

impl IntentEffects {
    /// Check if any effects need to be applied
    pub const fn has_effects(&self) -> bool {
        self.bounds.is_some() || self.color_correction.is_some() || self.brightness_range.is_some()
    }
}

/// Type alias for intent sender
pub type IntentSender<'a, const SIZE: usize> = Sender<'a, LightChangeIntent, SIZE>;

/// Type alias for intent receiver
pub type IntentReceiver<'a, const SIZE: usize> = Receiver<'a, LightChangeIntent, SIZE>;

/// Type alias for the intent channel
pub type IntentChannel<const SIZE: usize> = Channel<LightChangeIntent, SIZE>;

/// Processes user intents and converts them to renderer operations
pub struct IntentProcessor<'a, const SIZE: usize> {
    intents: IntentReceiver<'a, SIZE>,
}

impl<'a, const SIZE: usize> IntentProcessor<'a, SIZE> {
    /// Create a new intent processor
    pub const fn new(intents: IntentReceiver<'a, SIZE>) -> Self {
        Self { intents }
    }

    /// Process all pending intents from the channel (non-blocking)
    ///
    /// Drains all queued intents, pushes corresponding operations onto the stack,
    /// and returns side effects (bounds/filter changes) for the renderer to apply.
    pub fn process_pending<const N: usize>(
        &mut self,
        stack: &mut OperationStack<N>,
        current_brightness: u8,
    ) -> IntentEffects {
        let mut effects = IntentEffects::default();

        while let Ok(intent) = self.intents.try_receive() {
            match intent {
                LightChangeIntent::State(state_intent) => {
                    Self::process_state_intent(stack, &state_intent, current_brightness);
                }
                LightChangeIntent::Bounds(bounds) => {
                    effects.bounds = Some(bounds);
                }
                LightChangeIntent::ColorCorrection(color_correction) => {
                    effects.color_correction = Some(color_correction);
                }
                LightChangeIntent::BrightnessRange(range) => {
                    effects.brightness_range = Some(range);
                }
            }
        }

        effects
    }

    /// Process a state change intent, pushing operations onto the stack
    fn process_state_intent<const N: usize>(
        stack: &mut OperationStack<N>,
        intent: &LightStateIntent,
        current_brightness: u8,
    ) {
        if let Some(effect_id) = intent.effect_id {
            let _ = stack.push_effect(effect_id, current_brightness);
        }

        if let Some(brightness) = intent.brightness {
            let _ = stack.push_brightness(brightness);
        }

        if let Some(color) = intent.color {
            let _ = stack.push_color(color);
        } else if let Some(temp_kelvin) = intent.color_temperature {
            let color = kelvin_to_rgb(temp_kelvin);
            let _ = stack.push_color(color);
        }

        if let Some(power) = intent.power {
            if power {
                let _ = stack.push_power_on();
            } else {
                let _ = stack.push_power_off();
            }
        }
    }
}
