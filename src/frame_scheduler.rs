//! Frame scheduling and timing utilities.
//!
//! Provides portable frame pacing without async/await or platform-specific timers.
//! The caller is responsible for sleeping/waiting between frames.

use embassy_time::{Duration, Instant};
use crate::{OutputDriver, Renderer};

/// Default target frame rate (90 FPS).
pub const DEFAULT_FPS: u32 = 90;

/// Default frame duration based on target FPS.
pub const DEFAULT_FRAME_DURATION: Duration = Duration::from_millis(1000 / DEFAULT_FPS as u64);

/// Maximum drift before resetting frame timing (2 frames worth).
///
/// If we fall behind by more than this, we skip the backlog instead of catching up.
pub const MAX_DRIFT: Duration = Duration::from_millis(2 * (1000 / DEFAULT_FPS as u64));

/// Result of a frame tick operation.
#[derive(Debug, Clone, Copy)]
pub struct FrameResult {
    /// The deadline for the next frame.
    pub next_deadline: Instant,
    /// How long to wait until the next frame (may be zero if behind schedule).
    pub sleep_duration: Duration,
}

/// Portable frame scheduler that manages timing without async.
///
/// This scheduler:
/// - Tracks frame timing with drift correction
/// - Calls the renderer and output driver
/// - Returns timing info so the caller can sleep appropriately
///
/// # Usage
///
/// ```ignore
/// let mut scheduler = FrameScheduler::new(renderer, driver);
///
/// loop {
///     let now = get_current_time_ms();
///     let result = scheduler.tick(Instant::from_millis(now));
///     
///     // Platform-specific sleep
///     sleep_ms(result.sleep_duration.as_millis() as u64);
/// }
/// ```
pub struct FrameScheduler<'a, O: OutputDriver, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize>
{
    output: O,
    renderer: Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE>,
    next_frame: Instant,
    frame_duration: Duration,
}

impl<'a, O: OutputDriver, const MAX_LEDS: usize, const INTENT_CHANNEL_SIZE: usize>
    FrameScheduler<'a, O, MAX_LEDS, INTENT_CHANNEL_SIZE>
{
    /// Create a new frame scheduler.
    ///
    /// Uses `DEFAULT_FRAME_DURATION` (90 FPS) for frame timing.
    pub fn new(renderer: Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE>, driver: O) -> Self {
        Self::with_frame_duration(renderer, driver, DEFAULT_FRAME_DURATION)
    }

    /// Create a new frame scheduler with custom frame duration.
    pub fn with_frame_duration(
        renderer: Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE>,
        driver: O,
        frame_duration: Duration,
    ) -> Self {
        Self {
            output: driver,
            renderer,
            next_frame: Instant::from_millis(0),
            frame_duration,
        }
    }

    /// Process one frame and return timing information.
    ///
    /// This method:
    /// 1. Applies drift correction if we've fallen too far behind
    /// 2. Renders the current frame
    /// 3. Writes to the output driver
    /// 4. Returns the deadline for the next frame
    ///
    /// The caller is responsible for waiting until `next_deadline` before
    /// calling `tick` again.
    pub fn tick(&mut self, now: Instant) -> FrameResult {
        // Drift correction: if we've fallen too far behind, reset to now
        // This prevents catch-up bursts after long stalls
        let max_drift_ms = self.frame_duration.as_millis() * 2;
        let max_drift = Duration::from_millis(max_drift_ms);
        if now.as_millis() > self.next_frame.as_millis() + max_drift.as_millis() {
            self.next_frame = now;
        }

        // Render and output
        let frame = self.renderer.render(now);
        self.output.write(frame);

        // Calculate next frame deadline
        self.next_frame += self.frame_duration;

        // Calculate sleep duration (may be zero if we're behind)
        let sleep_duration = if self.next_frame.as_millis() > now.as_millis() {
            Duration::from_millis(self.next_frame.as_millis() - now.as_millis())
        } else {
            Duration::from_millis(0)
        };

        FrameResult {
            next_deadline: self.next_frame,
            sleep_duration,
        }
    }

    /// Get a reference to the renderer.
    pub fn renderer(&self) -> &Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE> {
        &self.renderer
    }

    /// Get a mutable reference to the renderer.
    pub fn renderer_mut(&mut self) -> &mut Renderer<'a, MAX_LEDS, INTENT_CHANNEL_SIZE> {
        &mut self.renderer
    }
}
