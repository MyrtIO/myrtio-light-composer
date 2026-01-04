//! Effect system with compile-time known effect variants
//!
//! All effects are stored in an enum to avoid heap allocations.
//! Each effect implements the `Effect` trait.

mod flow;
mod rainbow;
mod static_color;

use embassy_time::{Duration, Instant};
pub use flow::{FlowEffect, FlowVariant};
pub use rainbow::RainbowEffect;
pub use static_color::StaticColorEffect;

use crate::{color::Rgb, effect::rainbow::RainbowVariant};

const EFFECT_NAME_STATIC: &str = "static";
const EFFECT_NAME_FANTASY: &str = "fantasy";
const EFFECT_NAME_RAINBOW_SHORT: &str = "rainbow_short";
const EFFECT_NAME_RAINBOW_LONG: &str = "rainbow_long";
const EFFECT_NAME_RAINBOW_LONG_INVERSE: &str = "rainbow_long_inverse";
const EFFECT_NAME_GARLAND: &str = "garland";
const EFFECT_NAME_NEON: &str = "neon";
const EFFECT_NAME_REST: &str = "rest";
const EFFECT_NAME_SUNSET: &str = "sunset";

const EFFECT_ID_STATIC: u8 = 0;
const EFFECT_ID_FANTASY: u8 = 1;
const EFFECT_ID_RAINBOW_LONG: u8 = 2;
const EFFECT_ID_RAINBOW_SHORT: u8 = 3;
const EFFECT_ID_RAINBOW_LONG_INVERSE: u8 = 4;
const EFFECT_ID_GARLAND: u8 = 5;
const EFFECT_ID_NEON: u8 = 6;
const EFFECT_ID_REST: u8 = 7;
const EFFECT_ID_SUNSET: u8 = 8;

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
    Fantasy(RainbowEffect),
    /// Forward rainbow cycling effect
    RainbowForward(RainbowEffect),
    /// Backward rainbow cycling effect
    RainbowBackward(RainbowEffect),
    /// Static single color effect
    Static(StaticColorEffect),
    /// Neon effect with flowing multi-layer gradients
    Neon(FlowEffect),
    /// Rest effect with warm flowing gradients
    Rest(FlowEffect),
    /// Sunset effect with flowing gradients
    Sunset(FlowEffect),
}

/// Known effect ids that can be requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EffectId {
    Static = EFFECT_ID_STATIC,
    Fantasy = EFFECT_ID_FANTASY,
    RainbowLong = EFFECT_ID_RAINBOW_LONG,
    RainbowLongInverse = EFFECT_ID_RAINBOW_LONG_INVERSE,
    RainbowShort = EFFECT_ID_RAINBOW_SHORT,
    Garland = EFFECT_ID_GARLAND,
    Neon = EFFECT_ID_NEON,
    Rest = EFFECT_ID_REST,
    Sunset = EFFECT_ID_SUNSET,
}

impl Default for EffectSlot {
    fn default() -> Self {
        Self::Fantasy(RainbowEffect::new(RainbowVariant::Mirrored))
    }
}

impl EffectId {
    pub fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            EFFECT_ID_STATIC => Self::Static,
            EFFECT_ID_FANTASY => Self::Fantasy,
            EFFECT_ID_RAINBOW_LONG => Self::RainbowLong,
            EFFECT_ID_RAINBOW_SHORT => Self::RainbowShort,
            EFFECT_ID_RAINBOW_LONG_INVERSE => Self::RainbowLongInverse,
            EFFECT_ID_GARLAND => Self::Garland,
            EFFECT_ID_NEON => Self::Neon,
            EFFECT_ID_REST => Self::Rest,
            EFFECT_ID_SUNSET => Self::Sunset,
            _ => return None,
        })
    }

    pub fn to_slot(self, color: Rgb) -> EffectSlot {
        match self {
            Self::Static => EffectSlot::Static(StaticColorEffect::new(color)),
            Self::Fantasy => {
                EffectSlot::Fantasy(RainbowEffect::new(RainbowVariant::Mirrored))
            }
            Self::RainbowLong => {
                EffectSlot::RainbowForward(RainbowEffect::new(RainbowVariant::Long))
            }
            Self::RainbowShort => EffectSlot::RainbowBackward(RainbowEffect::new(
                RainbowVariant::Short,
            )),
            Self::RainbowLongInverse => EffectSlot::RainbowForward(
                RainbowEffect::new(RainbowVariant::Long).with_inverse(),
            ),
            Self::Garland => EffectSlot::RainbowBackward(
                RainbowEffect::new(RainbowVariant::Short).with_inverse(),
            ),
            Self::Neon => EffectSlot::Neon(FlowEffect::new(FlowVariant::Neon)),
            Self::Rest => EffectSlot::Rest(FlowEffect::new(FlowVariant::LavaLamp)),
            Self::Sunset => EffectSlot::Sunset(FlowEffect::new(FlowVariant::Sunset)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => EFFECT_NAME_STATIC,
            Self::Fantasy => EFFECT_NAME_FANTASY,
            Self::RainbowLong => EFFECT_NAME_RAINBOW_SHORT,
            Self::RainbowShort => EFFECT_NAME_RAINBOW_LONG,
            Self::RainbowLongInverse => EFFECT_NAME_RAINBOW_LONG_INVERSE,
            Self::Garland => EFFECT_NAME_GARLAND,
            Self::Neon => EFFECT_NAME_NEON,
            Self::Rest => EFFECT_NAME_REST,
            Self::Sunset => EFFECT_NAME_SUNSET,
        }
    }

    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s {
            EFFECT_NAME_STATIC => Some(Self::Static),
            EFFECT_NAME_FANTASY => Some(Self::Fantasy),
            EFFECT_NAME_RAINBOW_SHORT => Some(Self::RainbowLong),
            EFFECT_NAME_RAINBOW_LONG => Some(Self::RainbowShort),
            EFFECT_NAME_RAINBOW_LONG_INVERSE => Some(Self::RainbowLongInverse),
            EFFECT_NAME_GARLAND => Some(Self::Garland),
            EFFECT_NAME_NEON => Some(Self::Neon),
            EFFECT_NAME_REST => Some(Self::Rest),
            EFFECT_NAME_SUNSET => Some(Self::Sunset),
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
            Self::Fantasy(_) => RainbowEffect::PRECISE_COLORS,
            Self::RainbowForward(_) => RainbowEffect::PRECISE_COLORS,
            Self::RainbowBackward(_) => RainbowEffect::PRECISE_COLORS,
            Self::Static(_) => StaticColorEffect::PRECISE_COLORS,
            Self::Neon(_) | Self::Rest(_) | Self::Sunset(_) => {
                FlowEffect::PRECISE_COLORS
            }
        }
    }

    /// Render the current effect
    pub fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        match self {
            Self::Fantasy(effect) => effect.render(now, leds),
            Self::RainbowForward(effect) => effect.render(now, leds),
            Self::RainbowBackward(effect) => effect.render(now, leds),
            Self::Static(effect) => effect.render(now, leds),
            Self::Neon(effect) | Self::Rest(effect) | Self::Sunset(effect) => {
                effect.render(now, leds);
            }
        }
    }

    /// Reset the effect state
    pub fn reset(&mut self) {
        match self {
            Self::Fantasy(effect) => Effect::reset(effect),
            Self::RainbowForward(effect) => Effect::reset(effect),
            Self::RainbowBackward(effect) => Effect::reset(effect),
            Self::Static(effect) => Effect::reset(effect),
            Self::Neon(effect) | Self::Rest(effect) | Self::Sunset(effect) => {
                Effect::reset(effect);
            }
        }
    }

    /// Get the effect ID for external observation
    pub fn id(&self) -> EffectId {
        match self {
            Self::Fantasy(_) => EffectId::Fantasy,
            Self::RainbowForward(_) => EffectId::RainbowLong,
            Self::RainbowBackward(_) => EffectId::RainbowShort,
            Self::Static(_) => EffectId::Static,
            Self::Neon(_) => EffectId::Neon,
            Self::Rest(_) => EffectId::Rest,
            Self::Sunset(_) => EffectId::Sunset,
        }
    }

    /// Update the color of the current effect with optional transition.
    #[allow(clippy::single_match)]
    pub fn set_color(&mut self, color: Rgb, duration: Duration, now: Instant) {
        match self {
            Self::Static(effect) => effect.set_color(color, duration, now),
            _ => {}
        }
    }

    pub fn is_transitioning(&self) -> bool {
        match self {
            Self::Static(effect) => effect.is_transitioning(),
            Self::Fantasy(_)
            | Self::RainbowForward(_)
            | Self::RainbowBackward(_)
            | Self::Neon(_)
            | Self::Rest(_)
            | Self::Sunset(_) => false,
        }
    }
}
