//! Static color fill effect
//!
//! Fills all LEDs with a single solid color.
//! Supports smooth color transitions via [`ColorTransition`].

use embassy_time::{Duration, Instant};

use super::Effect;
use crate::{color::Rgb, transition::ValueTransition};

/// Static color effect - fills all LEDs with one color
///
/// Supports smooth crossfade transitions when changing colors.
#[derive(Debug, Clone)]
pub struct StaticColorEffect {
    /// Color with transition support
    color: ValueTransition<Rgb>,
}

impl StaticColorEffect {
    /// Create a new static color effect
    pub fn new(color: Rgb) -> Self {
        Self {
            color: ValueTransition::new_rgb(color),
        }
    }

    /// Set the color with smooth transition
    ///
    /// # Arguments
    /// * `color` - Target color
    /// * `duration` - Transition duration
    pub fn set_color(&mut self, color: Rgb, duration: Duration, now: Instant) {
        self.color.set(color, duration, now);
    }
}

impl Effect for StaticColorEffect {
    const PRECISE_COLORS: bool = true;

    fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        self.color.tick(now);

        for led in leds {
            *led = self.color.current();
        }
    }

    fn is_transitioning(&self) -> bool {
        self.color.is_transitioning()
    }
}
