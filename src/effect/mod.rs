//! Effect system with compile-time known effect variants
//!
//! All effects are stored in an enum to avoid heap allocations.
//! Each effect implements the `Effect` trait.

mod rainbow;
mod static_color;
mod velvet_analog;

use embassy_time::{Duration, Instant};
pub use rainbow::RainbowEffect;
pub use static_color::StaticColorEffect;
pub use velvet_analog::VelvetAnalogEffect;

use crate::{color::Rgb, effect::rainbow::RainbowDirection};

const EFFECT_NAME_STATIC: &str = "static";
const EFFECT_NAME_RAINBOW_MIRRORED: &str = "rainbow_mirrored";
const EFFECT_NAME_RAINBOW_FORWARD: &str = "rainbow_forward";
const EFFECT_NAME_RAINBOW_BACKWARD: &str = "rainbow_backward";
const EFFECT_NAME_VELVET_ANALOG: &str = "velvet_analog";

const EFFECT_ID_STATIC: u8 = 0;
const EFFECT_ID_RAINBOW_MIRRORED: u8 = 1;
const EFFECT_ID_RAINBOW_FORWARD: u8 = 2;
const EFFECT_ID_RAINBOW_BACKWARD: u8 = 3;
const EFFECT_ID_VELVET_ANALOG: u8 = 4;

pub trait Effect {
    /// Sets if effect requires precise (corrected) colors
    ///
    /// This option affects brightness, so it disabled by default
    const PRECISE_COLORS: bool = false;

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
    /// Mirrored rainbow cycling effect
    RainbowMirrored(RainbowEffect),
    /// Forward rainbow cycling effect
    RainbowForward(RainbowEffect),
    /// Backward rainbow cycling effect
    RainbowBackward(RainbowEffect),
    /// Static single color effect
    Static(StaticColorEffect),
    /// Velvet analog gradient derived from selected color
    VelvetAnalog(VelvetAnalogEffect),
}

/// Known effect ids that can be requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EffectId {
    Static          = EFFECT_ID_STATIC,
    RainbowMirrored = EFFECT_ID_RAINBOW_MIRRORED,
    RainbowForward  = EFFECT_ID_RAINBOW_FORWARD,
    RainbowBackward = EFFECT_ID_RAINBOW_BACKWARD,
    VelvetAnalog    = EFFECT_ID_VELVET_ANALOG,
}

impl Default for EffectSlot {
    fn default() -> Self {
        Self::RainbowMirrored(RainbowEffect::default())
    }
}

impl EffectId {
    pub fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            EFFECT_ID_STATIC => Self::Static,
            EFFECT_ID_RAINBOW_MIRRORED => Self::RainbowMirrored,
            EFFECT_ID_RAINBOW_FORWARD => Self::RainbowForward,
            EFFECT_ID_RAINBOW_BACKWARD => Self::RainbowBackward,
            EFFECT_ID_VELVET_ANALOG => Self::VelvetAnalog,
            _ => return None,
        })
    }

    pub fn to_slot(self, color: Rgb) -> EffectSlot {
        match self {
            Self::Static => EffectSlot::Static(StaticColorEffect::new(color)),
            Self::RainbowMirrored => EffectSlot::RainbowMirrored(
                RainbowEffect::default().with_direction(RainbowDirection::Mirrored),
            ),
            Self::RainbowForward => EffectSlot::RainbowForward(
                RainbowEffect::default().with_direction(RainbowDirection::Forward),
            ),
            Self::RainbowBackward => EffectSlot::RainbowBackward(
                RainbowEffect::default().with_direction(RainbowDirection::Backward),
            ),
            Self::VelvetAnalog => {
                EffectSlot::VelvetAnalog(VelvetAnalogEffect::new(color))
            }
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => EFFECT_NAME_STATIC,
            Self::RainbowMirrored => EFFECT_NAME_RAINBOW_MIRRORED,
            Self::RainbowForward => EFFECT_NAME_RAINBOW_FORWARD,
            Self::RainbowBackward => EFFECT_NAME_RAINBOW_BACKWARD,
            Self::VelvetAnalog => EFFECT_NAME_VELVET_ANALOG,
        }
    }

    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s {
            EFFECT_NAME_STATIC => Some(Self::Static),
            EFFECT_NAME_RAINBOW_MIRRORED => Some(Self::RainbowMirrored),
            EFFECT_NAME_RAINBOW_FORWARD => Some(Self::RainbowForward),
            EFFECT_NAME_RAINBOW_BACKWARD => Some(Self::RainbowBackward),
            EFFECT_NAME_VELVET_ANALOG => Some(Self::VelvetAnalog),
            _ => None,
        }
    }
}

impl EffectSlot {
    /// Returns if effect requires precise (corrected) colors
    ///
    /// Derived from each effect's `Effect::PRECISE_COLORS` constant.
    /// This option affects brightness, so it is disabled by default.
    pub fn requires_precise_colors(&self) -> bool {
        match self {
            Self::RainbowMirrored(_) => RainbowEffect::PRECISE_COLORS,
            Self::RainbowForward(_) => RainbowEffect::PRECISE_COLORS,
            Self::RainbowBackward(_) => RainbowEffect::PRECISE_COLORS,
            Self::Static(_) => StaticColorEffect::PRECISE_COLORS,
            Self::VelvetAnalog(_) => VelvetAnalogEffect::PRECISE_COLORS,
        }
    }

    /// Render the current effect
    pub fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        match self {
            Self::RainbowMirrored(effect) => effect.render(now, leds),
            Self::RainbowForward(effect) => effect.render(now, leds),
            Self::RainbowBackward(effect) => effect.render(now, leds),
            Self::Static(effect) => effect.render(now, leds),
            Self::VelvetAnalog(effect) => effect.render(now, leds),
        }
    }

    /// Reset the effect state
    pub fn reset(&mut self) {
        match self {
            Self::RainbowMirrored(effect) => Effect::reset(effect),
            Self::RainbowForward(effect) => Effect::reset(effect),
            Self::RainbowBackward(effect) => Effect::reset(effect),
            Self::Static(effect) => Effect::reset(effect),
            Self::VelvetAnalog(effect) => Effect::reset(effect),
        }
    }

    /// Get the effect ID for external observation
    pub fn id(&self) -> EffectId {
        match self {
            Self::RainbowMirrored(_) => EffectId::RainbowMirrored,
            Self::RainbowForward(_) => EffectId::RainbowForward,
            Self::RainbowBackward(_) => EffectId::RainbowBackward,
            Self::Static(_) => EffectId::Static,
            Self::VelvetAnalog(_) => EffectId::VelvetAnalog,
        }
    }

    /// Update the color of the current effect with optional transition.
    pub fn set_color(&mut self, color: Rgb, duration: Duration, now: Instant) {
        match self {
            Self::Static(effect) => effect.set_color(color, duration, now),
            Self::VelvetAnalog(effect) => effect.set_color(color, duration, now),
            _ => {}
        }
    }

    pub fn is_transitioning(&self) -> bool {
        match self {
            Self::Static(effect) => effect.is_transitioning(),
            Self::VelvetAnalog(effect) => effect.is_transitioning(),
            Self::RainbowMirrored(_)
            | Self::RainbowForward(_)
            | Self::RainbowBackward(_) => false,
        }
    }
}
