pub use smart_leds::hsv::hsv2rgb;

use crate::{
    color::{Hsv, Rgb},
    math8::blend8,
};

/// Mirror the first half of the array around the center
pub fn mirror_half(leds: &mut [Rgb]) {
    if leds.is_empty() {
        return;
    }
    // Compute center for mirroring
    let leds_len = leds.len();
    let mut center = leds_len / 2;
    if !leds_len.is_multiple_of(2) {
        center += 1;
    }
    center = center.min(leds_len);
    // Mirror the first half of the array around the center
    for i in 0..center {
        let mirrored = leds_len - 1 - i;
        leds[mirrored] = leds[i];
    }
}

/// Blend two RGB colors
///
/// # Arguments
/// * `a` - First color
/// * `b` - Second color
/// * `amount_of_b` - Blend factor (0 = all a, 255 = all b)
#[inline]
pub fn blend_colors(a: Rgb, b: Rgb, amount_of_b: u8) -> Rgb {
    Rgb {
        r: blend8(a.r, b.r, amount_of_b),
        g: blend8(a.g, b.g, amount_of_b),
        b: blend8(a.b, b.b, amount_of_b),
    }
}

/// Create an RGB color from a u32 value (0xRRGGBB format)
pub const fn rgb_from_u32(color: u32) -> Rgb {
    Rgb {
        r: ((color >> 16) & 0xFF) as u8,
        g: ((color >> 8) & 0xFF) as u8,
        b: (color & 0xFF) as u8,
    }
}

/// Convert RGB to HSV (all channels are 0-255).
///
/// Hue is represented on a 0-255 circle, matching `smart_leds::hsv::Hsv`.
#[allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn rgb2hsv(rgb: Rgb) -> Hsv {
    let r = rgb.r;
    let g = rgb.g;
    let b = rgb.b;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max.wrapping_sub(min);

    // Value is the max channel.
    let val = max;

    // Saturation: delta / max
    let sat = if max == 0 {
        0
    } else {
        ((u16::from(delta) * 255) / u16::from(max)) as u8
    };

    // Hue: 0-255 mapping across the color wheel.
    // Uses a common integer approximation: 0, 85, 171 offsets for R/G/B sectors.
    let hue = if delta == 0 {
        0
    } else if max == r {
        // between yellow & magenta
        let h = (43i16 * (i16::from(g) - i16::from(b))) / i16::from(delta);
        if h < 0 { (h + 256) as u8 } else { h as u8 }
    } else if max == g {
        // between cyan & yellow
        let h = 85i16 + (43i16 * (i16::from(b) - i16::from(r))) / i16::from(delta);
        if h < 0 { (h + 256) as u8 } else { h as u8 }
    } else {
        // max == b, between magenta & cyan
        let h = 171i16 + (43i16 * (i16::from(r) - i16::from(g))) / i16::from(delta);
        if h < 0 { (h + 256) as u8 } else { h as u8 }
    };

    Hsv { hue, sat, val }
}
