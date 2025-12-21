//! Velvet Analog mode
//!
//! Calm “premium” gradient derived from a single selected color.
//! Uses a small analog hue shift and very gentle breathing + midpoint drift.

use embassy_time::{Duration, Instant};

use super::Mode;
use crate::color::{fill_gradient_fp, rgb2hsv, GradientDirection, Hsv, Rgb};
use crate::math8::{blend8, ease_in_out_quad, scale8};
use crate::transition::ValueTransition;

const DEFAULT_BREATHE_PERIOD_MS: u64 = 14_000;
const DEFAULT_DRIFT_PERIOD_MS: u64 = 27_000;

// Small analog hue offset (0-255 hue circle).
const HUE_SHIFT: u8 = 10;

// Breathing: scale values by ~92%..100%.
const BREATHE_MIN_SCALE: u8 = 235;
const BREATHE_MAX_SCALE: u8 = 255;

#[derive(Debug, Clone)]
pub struct VelvetAnalogMode {
    color: ValueTransition<Rgb>,
    breathe_period: Duration,
    drift_period: Duration,
}

impl VelvetAnalogMode {
    pub fn new(color: Rgb) -> Self {
        Self {
            color: ValueTransition::new_rgb(color),
            breathe_period: Duration::from_millis(DEFAULT_BREATHE_PERIOD_MS),
            drift_period: Duration::from_millis(DEFAULT_DRIFT_PERIOD_MS),
        }
    }

    /// Set the anchor color with smooth transition.
    pub fn set_color(&mut self, color: Rgb, duration: Duration, now: Instant) {
        self.color.set(color, duration, now);
    }

    fn breathe_scale(&self, now: Instant) -> u8 {
        let period_ms = self.breathe_period.as_millis().max(1);
        let progress_ms = now.as_millis() % period_ms;
        #[allow(clippy::cast_possible_truncation)]
        let p = ((progress_ms * 255) / period_ms) as u8;
        let e = ease_in_out_quad(p);
        blend8(BREATHE_MIN_SCALE, BREATHE_MAX_SCALE, e)
    }

    fn midpoint(&self, now: Instant, leds: &mut [Rgb]) -> usize {
        if leds.len() <= 1 {
            return 0;
        }

        let last = leds.len() - 1;

        // Drift range: keep subtle (few pixels) and safe for small strips.
        let range = core::cmp::min(12usize, core::cmp::max(1usize, leds.len() / 10));

        let period_ms = self.drift_period.as_millis().max(1);
        let progress_ms = now.as_millis() % period_ms;
        #[allow(clippy::cast_possible_truncation)]
        let p = ((progress_ms * 255) / period_ms) as u8;

        // Triangle wave 0..255..0
        let tri = if p & 0x80 != 0 { 255 - p } else { p };
        let tri2 = tri << 1; // 0..254..0
        let e = ease_in_out_quad(tri2); // 0..255..0

        let offset: i16 = (i16::from(e) - 128) * (range as i16) / 128;
        let base_mid: i16 = (leds.len() / 2) as i16;

        let mid = (base_mid + offset).clamp(0, last as i16) as usize;
        mid
    }

    fn palette_from_anchor(anchor: Hsv, breathe_scale: u8) -> (Hsv, Hsv, Hsv) {
        // Keep saturation a bit subdued to avoid “neon”.
        let base_sat = anchor.sat.min(220);

        let shadow = Hsv {
            hue: anchor.hue.wrapping_sub(HUE_SHIFT),
            sat: scale8(base_sat, 170),
            val: scale8(anchor.val, scale8(120, breathe_scale)),
        };

        let body = Hsv {
            hue: anchor.hue,
            sat: scale8(base_sat, 200),
            val: scale8(anchor.val, scale8(200, breathe_scale)),
        };

        let highlight = Hsv {
            hue: anchor.hue.wrapping_add(HUE_SHIFT),
            sat: scale8(base_sat, 150),
            val: scale8(anchor.val, breathe_scale),
        };

        (shadow, body, highlight)
    }
}

impl Mode for VelvetAnalogMode {
    fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        self.color.tick(now);
        let rgb = self.color.current();

        if leds.is_empty() {
            return;
        }

        let breathe = self.breathe_scale(now);
        let anchor = rgb2hsv(rgb);
        let (c1, c2, c3) = Self::palette_from_anchor(anchor, breathe);

        let last = leds.len() - 1;
        let mid = self.midpoint(now, leds);

        fill_gradient_fp(leds, 0, c1, mid, c2, GradientDirection::Shortest);
        fill_gradient_fp(leds, mid, c2, last, c3, GradientDirection::Shortest);

    }

    fn is_transitioning(&self) -> bool {
        self.color.is_transitioning()
    }
}


















