//! Rainbow cycling effects
//!
//! Provides two rainbow effect variants:
//! - `RainbowEffect`: Uses fixed-point HSV gradient algorithm (ported from
//!   `FastLED`)
//! - `RainbowFlowEffect`: Three-point mirrored gradient with smooth flow

use embassy_time::{Duration, Instant};

use super::Effect;
use crate::{
    bounds::center_of,
    color::{Hsv, Rgb, fill_gradient_three_fp, mirror_half},
};

const DEFAULT_CYCLE_MS: u64 = 12_000;
const HUE_STEP: u8 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainbowVariant {
    Long,
    Short,
    Mirrored,
}

/// Rainbow effect using fixed-point HSV gradient algorithm
///
/// This implementation is ported from the `FastLED` `fillGradient` function
/// and uses 8.24 fixed-point arithmetic for smooth color transitions.
#[derive(Debug, Clone)]
pub struct RainbowEffect {
    /// Duration of one complete rainbow cycle
    cycle_duration: Duration,
    /// Brightness value (0-255)
    value: u8,
    /// Saturation (0-255)
    saturation: u8,
    /// Direction of the rainbow
    variant: RainbowVariant,
    /// Inverse direction
    inverse: bool,
}

impl RainbowEffect {
    /// Create a new rainbow effect with default parameters
    pub const fn new(variant: RainbowVariant) -> Self {
        Self {
            cycle_duration: Duration::from_millis(DEFAULT_CYCLE_MS),
            value: 255,
            saturation: 255,
            variant,
            inverse: false,
        }
    }

    /// Set the inverse direction
    #[must_use]
    pub fn with_inverse(mut self) -> Self {
        self.inverse = true;
        self
    }

    /// Set the cycle duration
    #[must_use]
    pub fn with_cycle_duration(mut self, duration: Duration) -> Self {
        self.cycle_duration = duration;
        self
    }

    /// Set the brightness value
    #[must_use]
    pub fn with_value(mut self, value: u8) -> Self {
        self.value = value;
        self
    }

    /// Set the saturation
    #[must_use]
    pub fn with_saturation(mut self, saturation: u8) -> Self {
        self.saturation = saturation;
        self
    }
}

impl Effect for RainbowEffect {
    fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        if leds.is_empty() {
            return;
        }

        let cycle_ms = self.cycle_duration.as_millis().max(1);
        let progress_ms = now.as_millis() % cycle_ms;
        #[allow(clippy::cast_possible_truncation)]
        let base_hue = ((progress_ms * 255) / cycle_ms) as u8;

        let c1 = Hsv {
            hue: base_hue,
            sat: self.saturation,
            val: self.value,
        };
        let c2 = Hsv {
            hue: base_hue.wrapping_add(HUE_STEP),
            sat: self.saturation,
            val: self.value,
        };
        let c3 = Hsv {
            hue: base_hue.wrapping_add(HUE_STEP * 2),
            sat: self.saturation,
            val: self.value,
        };

        match self.variant {
            RainbowVariant::Short => {
                fill_gradient_three_fp(leds, c1, c2, c3);
            }
            RainbowVariant::Long => {
                fill_gradient_three_fp(leds, c3, c1, c2);
            }
            RainbowVariant::Mirrored => {
                let center_len = center_of(leds);
                let (first_half, _) = leds.split_at_mut(center_len);
                fill_gradient_three_fp(first_half, c1, c2, c3);
                mirror_half(leds);
            }
        }

        if self.inverse {
            leds.reverse();
        }
    }
}
