# MyrtIO Light Composer

1D LED rendering library for Rust, designed for embedded systems (`no_std` friendly).

## Features

- **Embedded-first**: Works on bare-metal microcontrollers with `no_std`. Uses `embassy-time` for time primitives.
- **Portable intent channel**: Thread/interrupt-safe message passing via `critical-section`.
- **Flexible**: Message-passing architecture for safe multi-task control.
- **Caller-provided time**: The library never calls `Instant::now()` — callers provide the current time, enabling use across different runtimes.

## Key Concepts

*   **Renderer**: The core engine that orchestrates effects, transitions, and frame generation.
*   **Intents**: Thread-safe message-passing (`IntentChannel`) to control the renderer from any context.
*   **Effects**: Compile-time optimized visual effects (Rainbow, Static, Velvet Analog).
*   **Filters**: Post-processing chain for brightness control and color correction.
*   **Bounds**: Flexible rendering limits to support partial strip updates.

## Usage

```rust
use myrtio_light_composer::{
    Duration, EffectId, FilterProcessorConfig, Instant, IntentChannel,
    LightChangeIntent, LightEngineConfig, LightStateIntent, Renderer, Rgb,
    TransitionTimings, bounds::RenderingBounds, filter::BrightnessFilterConfig,
};

// 1. Create communication channel (static for 'static lifetime)
static INTENTS: IntentChannel<16> = IntentChannel::new();

// 2. Configure the engine
let config = LightEngineConfig {
    effect: EffectId::Rainbow,
    bounds: RenderingBounds { start: 0, end: 60 },
    timings: TransitionTimings {
        fade_out: Duration::from_millis(200),
        fade_in: Duration::from_millis(150),
        color_change: Duration::from_millis(100),
        brightness: Duration::from_millis(100),
    },
    filters: FilterProcessorConfig {
        brightness: BrightnessFilterConfig {
            min_brightness: 0,
            scale: 255,
            adjust: None,
        },
        color_correction: Rgb::new(255, 255, 255),
    },
    brightness: 255,
    color: Rgb::new(255, 180, 100),
};

// 3. Initialize renderer
let receiver = INTENTS.receiver();
let mut renderer = Renderer::<60, 16>::new(receiver, &config);

// 4. Send commands (from anywhere - thread/interrupt safe)
let sender = INTENTS.sender();
let _ = sender.try_send(LightChangeIntent::State(LightStateIntent {
    brightness: Some(255),
    color: Some(Rgb::new(255, 0, 0)),
    ..Default::default()
}));

// 5. Render loop (caller provides timing)
let mut time_ms: u64 = 0;
loop {
    let now = Instant::from_millis(time_ms);
    let frame = renderer.render(now);
    ws2812.write(frame);
    
    // Platform-specific delay (e.g., embassy Timer, std::thread::sleep, busy-wait)
    delay_ms(16);
    time_ms += 16;
}
```

## Desktop Preview

To run the interactive desktop preview:

```bash
cargo run --manifest-path preview/Cargo.toml
```
