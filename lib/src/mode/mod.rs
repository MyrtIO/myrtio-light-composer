//! Mode system with compile-time known mode variants
//!
//! All modes are stored in an enum to avoid heap allocations.
//! Each mode implements the `Mode` trait.

mod rainbow;
mod static_color;
mod velvet_analog;

use crate::color::Rgb;
use embassy_time::{Duration, Instant};

pub use rainbow::RainbowMode;
pub use static_color::StaticColorMode;
pub use velvet_analog::VelvetAnalogMode;

const MODE_NAME_STATIC: &str = "static";
const MODE_NAME_RAINBOW: &str = "rainbow";
const MODE_NAME_VELVET_ANALOG: &str = "velvet_analog";

const MODE_ID_STATIC: u8 = 0;
const MODE_ID_RAINBOW: u8 = 1;
const MODE_ID_VELVET_ANALOG: u8 = 2;

pub trait Mode {
    /// Render a single frame
    fn render(&mut self, now: Instant, leds: &mut [Rgb]);

    /// Reset mode state
    fn reset(&mut self) {}

    /// Check if the mode is transitioning
    fn is_transitioning(&self) -> bool {
        false
    }
}

/// Mode slot - enum containing all possible modes
#[derive(Debug, Clone)]
pub enum ModeSlot {
    /// Rainbow cycling mode
    Rainbow(RainbowMode),
    /// Static single color
    Static(StaticColorMode),
    /// Velvet analog gradient derived from selected color
    VelvetAnalog(VelvetAnalogMode),
}

/// Known mode ids that can be requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModeId {
    Static = MODE_ID_STATIC,
    Rainbow = MODE_ID_RAINBOW,
    VelvetAnalog = MODE_ID_VELVET_ANALOG,
}

impl Default for ModeSlot {
    fn default() -> Self {
        Self::Rainbow(RainbowMode::default())
    }
}

impl ModeId {
    pub fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            MODE_ID_STATIC => Self::Static,
            MODE_ID_RAINBOW => Self::Rainbow,
            MODE_ID_VELVET_ANALOG => Self::VelvetAnalog,
            _ => return None,
        })
    }

    pub fn to_mode_slot(self, color: Rgb) -> ModeSlot {
        match self {
            Self::Static => ModeSlot::Static(StaticColorMode::new(color)),
            Self::Rainbow => ModeSlot::Rainbow(RainbowMode::default()),
            Self::VelvetAnalog => ModeSlot::VelvetAnalog(VelvetAnalogMode::new(color)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => MODE_NAME_STATIC,
            Self::Rainbow => MODE_NAME_RAINBOW,
            Self::VelvetAnalog => MODE_NAME_VELVET_ANALOG,
        }
    }

    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s {
            MODE_NAME_STATIC => Some(Self::Static),
            MODE_NAME_RAINBOW => Some(Self::Rainbow),
            MODE_NAME_VELVET_ANALOG => Some(Self::VelvetAnalog),
            _ => None,
        }
    }
}

impl ModeSlot {
    /// Render the current mode
    pub fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        match self {
            Self::Rainbow(mode) => mode.render(now, leds),
            Self::Static(mode) => mode.render(now, leds),
            Self::VelvetAnalog(mode) => mode.render(now, leds),
        };
    }

    /// Reset the mode state
    pub fn reset(&mut self) {
        match self {
            Self::Rainbow(mode) => Mode::reset(mode),
            Self::Static(mode) => Mode::reset(mode),
            Self::VelvetAnalog(mode) => Mode::reset(mode),
        }
    }

    /// Get the mode ID for external observation
    pub fn mode_id(&self) -> ModeId {
        match self {
            Self::Rainbow(_) => ModeId::Rainbow,
            Self::Static(_) => ModeId::Static,
            Self::VelvetAnalog(_) => ModeId::VelvetAnalog,
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
