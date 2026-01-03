use crate::Rgb;

/// Bounds of the rendering area
#[derive(Debug, Clone, Copy)]
pub struct RenderingBounds {
    pub start: u8,
    pub end: u8,
}

impl RenderingBounds {
    /// Get the number of LEDs in the rendering area
    pub const fn count(self) -> u8 {
        self.end - self.start
    }

    /// Returns center of the rendering areas
    pub const fn center(self) -> u8 {
        let count = self.count();
        let mut center_len = count / 2;
        if !count.is_multiple_of(2) {
            center_len += 1;
        }

        if center_len <= count {
            return center_len;
        }
        count
    }
}

pub(crate) fn bounded(leds: &mut [Rgb], bounds: RenderingBounds) -> &mut [Rgb] {
    let start = bounds.start;
    let end = bounds.end;
    &mut leds[start as usize..end as usize]
}

pub const fn center_of<T>(leds: &[T]) -> usize {
    let count = leds.len();
    let mut center_len = count / 2;
    if !count.is_multiple_of(2) {
        center_len += 1;
    }

    if center_len <= count {
        return center_len;
    }
    count
}
