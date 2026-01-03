mod gradient;
mod kelvin;
mod utils;

pub use gradient::{GradientDirection, fill_gradient_fp, fill_gradient_three_fp};
pub use kelvin::kelvin_to_rgb;
use smart_leds::{RGB8, hsv::Hsv as HSV};
pub use utils::{blend_colors, hsv2rgb, mirror_half, rgb_from_u32, rgb2hsv};

pub type Rgb = RGB8;
pub type Hsv = HSV;
