//! Flow effect with palette-based presets

use embassy_time::Instant;

use super::Effect;
use crate::{
    color::{Rgb, blend_colors, rgb_from_u32},
    math8::{blend8, ease_in_out_quad, scale8},
};

/// Create a palette from a list of hex colors (0xRRGGBB format)
macro_rules! hex_palette {
    ($($color:expr),*) => {
        [
            $(rgb_from_u32($color)),*
        ]
    };
}

// Aurora palette: cool blue/teal/violet tones
#[allow(clippy::unreadable_literal)]
const AURORA_PALETTE: [Rgb; 6] = hex_palette![
    0x002EB8, // Deep blue
    0x00FFD4, // Teal (stronger)
    0x14FF78, // Green (muted/teal-leaning)
    0x00C8FF, // Cyan/teal
    0x8800FF, // Violet
    0xFF0090  // Pink/magenta
];

// Lava lamp palette: warm red/orange/purple tones
#[allow(clippy::unreadable_literal)]
const LAVA_LAMP_PALETTE: [Rgb; 5] = hex_palette![
    0x3C0014, // Dark magenta
    0xD10038, // Deep red
    0xFF5000, // Orange
    0xFF972E, // Bright yellow
    0xF2039F  // Purple accent
];

// Balanced tuning: visible motion, still premium
const LAYER1_PERIOD_MS: u64 = 8_000; // Slow base layer
const LAYER2_PERIOD_MS: u64 = 5_000; // Faster mid layer
const LAYER3_PERIOD_MS: u64 = 13_000; // Very slow shimmer

// Spatial tuning (in LED pixels).
// These are treated as "noise cell sizes" and are derived from strip length
// so the effect stays smooth on both short and long strips.
const MIN_CELL1_LEDS: u32 = 12;
const MIN_CELL2_LEDS: u32 = 6;
const MIN_CELL3_LEDS: u32 = 18;

const MAX_CELL1_LEDS: u32 = 40;
const MAX_CELL2_LEDS: u32 = 18;
const MAX_CELL3_LEDS: u32 = 60;

/// Flow effect variant selector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowVariant {
    /// Aurora: cool blue/teal/violet tones (northern lights)
    Aurora,
    /// Lava lamp: warm red/orange/purple tones
    LavaLamp,
}

/// Flow effect with layered flowing gradients
///
/// This effect uses multi-layer value noise to create smooth organic motion.
/// Different palettes produce different visual themes.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct FlowEffect {
    layer1_period: u64,
    layer2_period: u64,
    layer3_period: u64,
    variant: FlowVariant,
}

impl Default for FlowEffect {
    fn default() -> Self {
        Self::new(FlowVariant::Aurora)
    }
}

impl FlowEffect {
    /// Create a new flow effect with the specified variant
    pub const fn new(variant: FlowVariant) -> Self {
        Self {
            layer1_period: LAYER1_PERIOD_MS,
            layer2_period: LAYER2_PERIOD_MS,
            layer3_period: LAYER3_PERIOD_MS,
            variant,
        }
    }

    /// Simple deterministic hash for noise generation
    #[inline]
    const fn hash(x: u64) -> u32 {
        // SplitMix64-style mixing, then fold down to u32.
        let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        #[allow(clippy::cast_possible_truncation)]
        {
            (z ^ (z >> 31)) as u32
        }
    }

    #[inline]
    const fn clamp_u32(v: u32, min_v: u32, max_v: u32) -> u32 {
        if v < min_v {
            min_v
        } else if v > max_v {
            max_v
        } else {
            v
        }
    }

    /// Smooth 1D value noise: input is 16.16 fixed-point position.
    /// Returns 0-255.
    #[inline]
    fn value_noise(pos_fp: u64) -> u8 {
        let cell = pos_fp >> 16;
        let frac = ((pos_fp >> 8) & 0xFF) as u8;

        let v0 = (Self::hash(cell) & 0xFF) as u8;
        let v1 = (Self::hash(cell.wrapping_add(1)) & 0xFF) as u8;

        // Smooth interpolation
        let t = ease_in_out_quad(frac);
        blend8(v0, v1, t)
    }

    /// Sample the palette at position t (0-255)
    #[allow(clippy::cast_possible_truncation)]
    fn sample_palette(palette: &[Rgb], t: u8) -> Rgb {
        // Map t (0-255) across N colors (N-1 segments) with blending.
        let segments = palette.len().saturating_sub(1);
        if segments == 0 {
            return palette.first().copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        }

        let scaled = u16::from(t) * (segments as u16); // 0..255*(N-1)
        let segment = (scaled >> 8).min(segments.saturating_sub(1) as u16) as usize;
        let local_t = (scaled & 0xFF) as u8;

        blend_colors(palette[segment], palette[segment + 1], local_t)
    }

    /// Get the palette for the current variant
    fn palette(&self) -> &'static [Rgb] {
        match self.variant {
            FlowVariant::Aurora => &AURORA_PALETTE,
            FlowVariant::LavaLamp => &LAVA_LAMP_PALETTE,
        }
    }

    /// Combine multiple noise layers into a final value
    #[allow(clippy::cast_possible_truncation)]
    fn combined_noise(&self, i: u32, len: u32, now: Instant) -> u8 {
        let time_ms = now.as_millis();

        // Derive cell sizes from strip length so the effect stays smooth.
        // These values are "LEDs per noise cell" (bigger => smoother, slower spatial
        // change).
        let cell1 = Self::clamp_u32(len / 6, MIN_CELL1_LEDS, MAX_CELL1_LEDS).max(1);
        let cell2 = Self::clamp_u32(len / 12, MIN_CELL2_LEDS, MAX_CELL2_LEDS).max(1);
        let cell3 = Self::clamp_u32(len / 4, MIN_CELL3_LEDS, MAX_CELL3_LEDS).max(1);

        // Convert LED index to 16.16 fixed-point in "cell space".
        let i64 = u64::from(i);
        let x1 = (i64 << 16) / u64::from(cell1);
        let x2 = (i64 << 16) / u64::from(cell2);
        let x3 = (i64 << 16) / u64::from(cell3);

        // High-resolution phase (16.16): continuous motion with no stepping.
        // We intentionally do NOT modulo time to avoid visible jumps on wrap.
        let p1 = (time_ms << 16) / self.layer1_period;
        let p2 = (time_ms << 16) / self.layer2_period;
        let p3 = (time_ms << 16) / self.layer3_period;

        // Layer directions differ slightly for depth/parallax.
        let n1 = Self::value_noise(x1.wrapping_add(p1));
        let n2 = Self::value_noise(x2.wrapping_sub(p2));
        let n3 = Self::value_noise(x3.wrapping_add(p3.wrapping_mul(2)));

        // Blend layers: 50% base, 30% detail, 20% shimmer
        let combined =
            (u16::from(n1) * 128 + u16::from(n2) * 77 + u16::from(n3) * 51) >> 8;
        combined as u8
    }
}

impl Effect for FlowEffect {
    fn render(&mut self, now: Instant, leds: &mut [Rgb]) {
        if leds.is_empty() {
            return;
        }

        let len = u32::try_from(leds.len()).unwrap_or(u32::MAX);
        let palette = self.palette();

        for (i, led) in leds.iter_mut().enumerate() {
            // Get combined noise value
            let i_u32 = u32::try_from(i).unwrap_or(u32::MAX);
            let noise = self.combined_noise(i_u32, len, now);

            // Sample palette and apply subtle brightness modulation
            let base_color = Self::sample_palette(palette, noise);

            // Add subtle brightness variation based on noise for "silky" feel
            let brightness_mod = scale8(noise, 64).saturating_add(191); // 75%-100% range
            *led = Rgb {
                r: scale8(base_color.r, brightness_mod),
                g: scale8(base_color.g, brightness_mod),
                b: scale8(base_color.b, brightness_mod),
            };
        }
    }
}
