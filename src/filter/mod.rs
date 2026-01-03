use embassy_time::Instant;

use crate::color::Rgb;

mod brightness;
mod color_correction;

pub(crate) trait Filter {
    /// Apply the effect to a frame
    fn apply(&mut self, frame: &mut [Rgb]);

    fn tick(&mut self, _now: Instant) {}
}

use brightness::BrightnessFilter;
pub use brightness::{BrightnessFilterConfig, BrightnessRange};
pub(crate) use color_correction::ColorCorrection;

#[derive(Debug, Clone)]
pub struct FilterProcessorConfig {
    /// Brightness filter
    pub brightness: BrightnessFilterConfig,
    /// Color correction
    pub color_correction: Rgb,
}

/// Filter processor - applies post-processing to frames
///
/// This is the central hub for all output modifications.
/// Processing is applied in a specific order to ensure correct results.
#[derive(Debug)]
pub(crate) struct FilterProcessor {
    /// Brightness filter
    pub brightness: BrightnessFilter,
    /// Color correction filter
    pub color_correction: ColorCorrection,
}

impl FilterProcessor {
    /// Create a new output processor with default settings
    pub(crate) fn new(config: &FilterProcessorConfig) -> Self {
        let brightness = BrightnessFilter::new(0, &config.brightness);
        let color_correction = ColorCorrection::new(config.color_correction);
        Self {
            brightness,
            color_correction,
        }
    }

    /// Tick the filters
    pub(crate) fn tick(&mut self, now: Instant) {
        self.brightness.tick(now);
        self.color_correction.tick(now);
    }
}
