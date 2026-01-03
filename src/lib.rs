#![no_std]

pub mod bounds;
pub mod channel;
pub mod color;
pub mod effect;
pub mod filter;
pub mod frame_scheduler;
pub mod gamma;
pub mod intent_processor;
pub mod math8;
pub mod operation;
pub mod renderer;
pub mod transition;

pub use filter::{BrightnessRange, FilterProcessorConfig};
pub use intent_processor::{
    IntentChannel, IntentEffects, IntentProcessor, IntentReceiver, IntentSender,
    LightChangeIntent, LightStateIntent,
};
pub use renderer::{LightEngineConfig, LightState, Renderer, TransitionTimings};
pub use frame_scheduler::FrameScheduler;
pub use gamma::ws2812_lut;
pub use effect::{EffectId, EffectSlot};
pub use operation::{Operation, OperationStack};

pub use color::{Hsv, Rgb};
pub use math8::{U8Adjuster, ease_in_out_quad};
pub use embassy_time::{Duration, Instant};

/// Abstract LED driver trait
///
/// Implement this trait to support different hardware platforms.
/// The light engine is generic over this trait.
pub trait OutputDriver {
    /// Write colors to the LED strip
    fn write(&mut self, colors: &[Rgb]);
}
