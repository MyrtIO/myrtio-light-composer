use smart_leds::hsv::hsv2rgb;

use crate::color::{Hsv, Rgb};

/// Hue direction for gradient calculation
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    Forward,
    Backward,
    Shortest,
}

/// Fill gradient using fixed-point 8.24 arithmetic (ported from `FastLED`)
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap
)]
pub fn fill_gradient_fp(
    leds: &mut [Rgb],
    start_pos: usize,
    start_color: Hsv,
    end_pos: usize,
    end_color: Hsv,
    direction: GradientDirection,
) {
    if leds.is_empty() {
        return;
    }

    // Ensure proper ordering
    let (start_pos, end_pos, mut start_color, mut end_color) = if end_pos < start_pos
    {
        (end_pos, start_pos, end_color, start_color)
    } else {
        (start_pos, end_pos, start_color, end_color)
    };

    // Handle black/white edge cases for hue
    if end_color.val == 0 || end_color.sat == 0 {
        end_color.hue = start_color.hue;
    }
    if start_color.val == 0 || start_color.sat == 0 {
        start_color.hue = end_color.hue;
    }

    // Calculate distances in 8.7 fixed-point
    let sat_distance87 =
        (i16::from(end_color.sat) - i16::from(start_color.sat)) << 7;
    let val_distance87 =
        (i16::from(end_color.val) - i16::from(start_color.val)) << 7;

    let hue_delta = end_color.hue.wrapping_sub(start_color.hue);

    // Determine actual direction based on hue delta
    let actual_direction = match direction {
        GradientDirection::Shortest => {
            if hue_delta > 127 {
                GradientDirection::Backward
            } else {
                GradientDirection::Forward
            }
        }
        other => other,
    };

    let hue_distance87: i16 = if actual_direction == GradientDirection::Forward {
        i16::from(hue_delta) << 7
    } else {
        let backward_delta = 256u16.wrapping_sub(u16::from(hue_delta)) as u8;
        -((i16::from(backward_delta)) << 7)
    };

    let pixel_distance = end_pos.saturating_sub(start_pos);
    let divisor = if pixel_distance == 0 {
        1
    } else {
        pixel_distance as i32
    };

    // Calculate 8.23 fixed-point deltas
    let hue_delta823 = ((i32::from(hue_distance87) * 65536) / divisor) * 2;
    let sat_delta823 = ((i32::from(sat_distance87) * 65536) / divisor) * 2;
    let val_delta823 = ((i32::from(val_distance87) * 65536) / divisor) * 2;

    // Initialize 8.24 accumulators
    let mut hue824 = u32::from(start_color.hue) << 24;
    let mut sat824 = u32::from(start_color.sat) << 24;
    let mut val824 = u32::from(start_color.val) << 24;

    let end_pos = end_pos.min(leds.len() - 1);
    for led in leds.iter_mut().take(end_pos + 1).skip(start_pos) {
        *led = hsv2rgb(Hsv {
            hue: (hue824 >> 24) as u8,
            sat: (sat824 >> 24) as u8,
            val: (val824 >> 24) as u8,
        });
        hue824 = hue824.wrapping_add(hue_delta823 as u32);
        sat824 = sat824.wrapping_add(sat_delta823 as u32);
        val824 = val824.wrapping_add(val_delta823 as u32);
    }
}

/// Fill three-color gradient using fixed-point math
pub fn fill_gradient_three_fp(leds: &mut [Rgb], c1: Hsv, c2: Hsv, c3: Hsv) {
    if leds.is_empty() {
        return;
    }

    let len = leds.len();
    let half = len / 2;
    let last = len.saturating_sub(1);

    fill_gradient_fp(leds, 0, c1, half, c2, GradientDirection::Forward);
    if last > half {
        fill_gradient_fp(leds, half, c2, last, c3, GradientDirection::Forward);
    }
}
