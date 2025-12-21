#![no_std]
#![no_main]

use embassy_time::Instant;

pub mod color;
pub mod effect;
pub mod engine;
pub mod gamma;
pub mod math8;
pub mod mode;
pub mod operation;
pub mod transition;
pub mod bounds;

pub use effect::EffectProcessorConfig;
pub use engine::{
    IntentChannel, IntentReceiver, IntentSender, LightEngine, LightEngineConfig, LightIntent,
    TransitionTimings,
};
pub use gamma::ws2812_lut;
pub use mode::{ModeId, ModeSlot};
pub use operation::{Operation, OperationStack};

pub use color::{Hsv, Rgb};
pub use math8::{U8Adjuster, ease_in_out_quad};

/// Abstract LED driver trait
///
/// Implement this trait to support different hardware platforms.
/// The light engine is generic over this trait.
pub trait LedDriver {
    /// Write colors to the LED strip
    fn write(&mut self, colors: &[Rgb]);
}
