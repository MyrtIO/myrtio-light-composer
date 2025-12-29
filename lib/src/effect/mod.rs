//! Mode system with compile-time known mode variants
//!
//! All modes are stored in an enum to avoid heap allocations.
//! Each mode implements the `Mode` trait.

mod rainbow;
mod static_color;
mod velvet_analog;

use crate::color::Rgb;
use embassy_time::{Duration, Instant};

pub use rainbow::RainbowEffect;
pub use static_color::StaticColorEffect;
pub use velvet_analog::VelvetAnalogEffect;

const EFFECT_NAME_STATIC: &str = "static";
const EFFECT_NAME_RAINBOW: &str = "rainbow";
const EFFECT_NAME_VELVET_ANALOG: &str = "velvet_analog";

const EFFECT_ID_STATIC: u8 = 0;
const EFFECT_ID_RAINBOW: u8 = 1;
const EFFECT_ID_VELVET_ANALOG: u8 = 2;

pub trait Effect {
    /// Render a single frame
    fn render(&mut self, now: Instant, leds: &mut [Rgb]);

    /// Reset effect state
    fn reset(&mut self) {}

    /// Check if the effect is transitioning
    fn is_transitioning(&self) -> bool {
        false
    }
}

/// Effect slot - enum containing all possible effects
#[derive(Debug, Clone)]
pub enum EffectSlot {
    /// Rainbow cycling effect
    Rainbow(RainbowEffect),
    /// Static single color effect
    Static(StaticColorEffect),
    /// Velvet analog gradient derived from selected color
    VelvetAnalog(VelvetAnalogEffect),
}

/// Known effect ids that can be requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EffectId {
    Static = EFFECT_ID_STATIC,
    Rainbow = EFFECT_ID_RAINBOW,
    VelvetAnalog = EFFECT_ID_VELVET_ANALOG,
}

impl Default for EffectSlot {
    fn default() -> Self {
        Self::Rainbow(RainbowEffect::default())
    }
}

impl EffectId {
    pub fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            EFFECT_ID_STATIC => Self::Static,
            EFFECT_ID_RAINBOW => Self::Rainbow,
            EFFECT_ID_VELVET_ANALOG => Self::VelvetAnalog,
            _ => return None,
        })
    }

    pub fn to_slot(self, color: Rgb) -> EffectSlot {
        match self {
            Self::Static => EffectSlot::Static(StaticColorEffect::new(color)),
            Self::Rainbow => EffectSlot::Rainbow(RainbowEffect::default()),
            Self::VelvetAnalog => EffectSlot::VelvetAnalog(VelvetAnalogEffect::new(color)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => EFFECT_NAME_STATIC,
            Self::Rainbow => EFFECT_NAME_RAINBOW,
            Self::VelvetAnalog => EFFECT_NAME_VELVET_ANALOG,
        }
    }

    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s {
            EFFECT_NAME_STATIC => Some(Self::Static),
            EFFECT_NAME_RAINBOW => Some(Self::Rainbow),
            EFFECT_NAME_VELVET_ANALOG => Some(Self::VelvetAnalog),
            _ => None,
        }
    }
}

impl EffectSlot {
    /// Render the current mode
    pub fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        match self {
            Self::Rainbow(effect) => effect.render(now, leds),
            Self::Static(effect) => effect.render(now, leds),
            Self::VelvetAnalog(effect) => effect.render(now, leds),
        };
    }

    /// Reset the mode state
    pub fn reset(&mut self) {
        match self {
            Self::Rainbow(effect) => Effect::reset(effect),
            Self::Static(effect) => Effect::reset(effect),
            Self::VelvetAnalog(effect) => Effect::reset(effect),
        }
    }

    /// Get the mode ID for external observation
    pub fn id(&self) -> EffectId {
        match self {
            Self::Rainbow(_) => EffectId::Rainbow,
            Self::Static(_) => EffectId::Static,
            Self::VelvetAnalog(_) => EffectId::VelvetAnalog,
        }
    }

    /// Update the color of the current mode with optional transition.
    pub fn set_color(&mut self, color: Rgb, duration: Duration, now: Instant) {
        match self {
            Self::Static(mode) => mode.set_color(color, duration, now),
            Self::VelvetAnalog(mode) => mode.set_color(color, duration, now),
            _ => {}
        }
    }

    pub fn is_transitioning(&self) -> bool {
        match self {
            Self::Static(mode) => mode.is_transitioning(),
            Self::VelvetAnalog(mode) => mode.is_transitioning(),
            _ => false,
        }
    }
}
