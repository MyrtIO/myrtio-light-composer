# Light Composer Architecture Guide

**Audience:** Contributors, AI Agents, and maintainers.

## Maintenance Rule

> [!IMPORTANT]
> **Synchronize this file**: If you rename, move, or add files in `src/`, you **MUST** update the [Module Map](#module-map-src) section below in the same commit. This document serves as the ground truth for the codebase structure.

## Overview

The Light Composer is a **1D LED rendering engine** built for embedded Rust (`no_std`). It uses a message-passing architecture where "Intents" (high-level goals) are processed into "Operations" (low-level transitions) that drive a "Renderer".

**Key design principles**:
- Uses `embassy-time` for time primitives (`Instant`, `Duration`), but **never calls `Instant::now()`** — callers provide the current time, enabling use across different runtimes.
- Uses a portable intent channel (`src/channel.rs`) built on `critical-section` for thread/interrupt-safe messaging.

### Dataflow

```mermaid
flowchart LR
    User[User Code / MQTT] -->|LightChangeIntent| Channel[IntentChannel]
    Channel -->|Receiver| IntentProc[IntentProcessor]
    IntentProc -->|Push| Ops[OperationStack]
    IntentProc -->|Side Effects| Filters[Filter State]
    
    subgraph Renderer
        Ops -->|Next Op| State[LightState]
        State -->|Render| Effect[Current Effect]
        Effect -->|Raw Pixels| Frame[Frame Buffer]
        Frame -->|Apply| Filters[Filters (Brightness/Color)]
    end
    
    Filters -->|Final Output| Driver[OutputDriver]
```

1.  **Input**: External systems send `LightChangeIntent` via the portable `IntentChannel`.
2.  **Processing**: `IntentProcessor` drains the channel and converts intents into a sequence of `Operation`s (e.g., "Fade to Black", "Change Effect", "Fade Up").
3.  **Rendering**: `Renderer::render(now)` runs every frame:
    *   Advances active transitions (color/brightness).
    *   Renders the active `Effect` into a buffer.
    *   Applies `FilterProcessor` (color correction, brightness scaling).
4.  **Output**: The caller writes the frame to hardware. `FrameScheduler` can help with timing.

## Public API Map

The entry point is `src/lib.rs`. It re-exports the primary components needed to build an application:

*   **Traits**: `OutputDriver` (hardware abstraction).
*   **Components**: `Renderer`, `IntentChannel`, `FrameScheduler`.
*   **Configuration**: `LightEngineConfig`, `TransitionTimings`.
*   **Time Types**: `Duration`, `Instant` (re-exported from `embassy-time`).
*   **Data Types**: `Rgb`, `Hsv`, `EffectId`.

## Module Map (`src/`)

### Core Infrastructure

*   **[`src/channel.rs`](src/channel.rs)**
    *   **Purpose**: Portable bounded channel for intent passing.
    *   **Key Types**: `Channel<T, SIZE>`, `Sender`, `Receiver`.
    *   **Implementation**: Uses `critical-section` for synchronization and `heapless::Deque` for storage.

### Core Logic

*   **[`src/renderer.rs`](src/renderer.rs)**
    *   **Purpose**: The heart of the engine. Holds the `LightState`, `OperationStack`, and `FilterProcessor`.
    *   **Key Function**: `render(now) -> &[Rgb]`.
    *   **Invariants**: Must be called cyclically. Handles the final composition of effect output and filters.

*   **[`src/intent_processor.rs`](src/intent_processor.rs)**
    *   **Purpose**: Translates high-level user desires (`LightChangeIntent`) into concrete `Operation`s.
    *   **Key Types**: `IntentChannel` (type alias), `IntentProcessor`, `LightStateIntent`, `LightChangeIntent`.
    *   **Details**: Handles prioritization (e.g., immediate power-off vs smooth transitions). Returns "side effects" (bounds, filter configs) that don't fit into the operation stack.

*   **[`src/operation.rs`](src/operation.rs)**
    *   **Purpose**: Defines atomic actions the renderer can perform.
    *   **Key Types**: `Operation` enum (`SetBrightness`, `SwitchEffect`, `PowerOff`), `OperationStack`.
    *   **Invariants**: The stack size is fixed at compile time (generic const `N`).

*   **[`src/frame_scheduler.rs`](src/frame_scheduler.rs)**
    *   **Purpose**: Portable frame pacing helper (synchronous, no async).
    *   **Key Types**: `FrameScheduler`, `FrameResult`.
    *   **Details**: Manages timing with drift correction. Returns sleep duration; the caller performs the actual wait.

### Effects (`src/effect/`)

*   **[`src/effect/mod.rs`](src/effect/mod.rs)**: Defines the `Effect` trait and the `EffectSlot` enum (static dispatch wrapper for all effects). Effect capabilities (like `PRECISE_COLORS`) are derived from trait constants.
*   **[`src/effect/rainbow.rs`](src/effect/rainbow.rs)**: `RainbowEffect`. Uses fixed-point math for smooth hue cycling.
*   **[`src/effect/static_color.rs`](src/effect/static_color.rs)**: `StaticColorEffect`. Simple solid color with transition support. Requires precise colors.
*   **[`src/effect/velvet_analog.rs`](src/effect/velvet_analog.rs)**: `VelvetAnalogEffect`. Complex gradient generator with "breathing" and subtle analog drift.
*   **[`src/effect/flow.rs`](src/effect/flow.rs)**: `FlowEffect`. Premium flowing multi-layer gradients using fixed-point noise. Supports palette-based presets via `FlowVariant` (`Aurora`, `LavaLamp`).

### Filters (`src/filter/`)

*   **[`src/filter/mod.rs`](src/filter/mod.rs)**: `FilterProcessor` orchestrator.
*   **[`src/filter/brightness.rs`](src/filter/brightness.rs)**: Handles global brightness, fade-ins, and fade-outs. Includes min-brightness and scaling adjustments.
*   **[`src/filter/color_correction.rs`](src/filter/color_correction.rs)**: Applies per-channel white balance/correction.

### Utilities

*   **[`src/transition.rs`](src/transition.rs)**: `ValueTransition<T>`. Generic helper for smooth value interpolation over time.
*   **[`src/bounds.rs`](src/bounds.rs)**: `RenderingBounds`. Logic for mapping a 0-N virtual strip to a subset of physical LEDs.
*   **[`src/color/`](src/color/)**:
    *   `mod.rs`: Re-exports.
    *   `gradient.rs`: Fixed-point gradient algorithms (ported from FastLED).
    *   `kelvin.rs`: Color temperature (K) to RGB conversion.
    *   `utils.rs`: RGB/HSV conversion, blending, mirroring.
*   **[`src/math8.rs`](src/math8.rs)**: 8-bit integer math helpers (`scale8`, `blend8`, `progress8`) for performance on 8/32-bit MCUs.
*   **[`src/gamma.rs`](src/gamma.rs)**: `ws2812_lut`. Pre-computed gamma 2.2 lookup table for standard NeoPixels.

## Extension Guidelines

### Adding a New Effect

1.  Create `src/effect/my_effect.rs`. Implement `Effect` trait.
2.  Set `const PRECISE_COLORS: bool` appropriately (true if the effect needs color correction).
3.  Register in `src/effect/mod.rs`:
    *   Add to `EffectSlot` enum.
    *   Add to `EffectId` enum (and mapping logic).
    *   Update `EffectSlot::requires_precise_colors()` match arm.
4.  Add `EFFECT_NAME_MY_EFFECT` and `EFFECT_ID_MY_EFFECT` constants.

### Adding a New Filter

1.  Create `src/filter/my_filter.rs`.
2.  Add field to `FilterProcessor` in `src/filter/mod.rs`.
3.  Apply it in `Renderer::render` (decide if it runs before or after brightness/color correction).

## Non-Core Directories

*   **`preview/`**: A desktop GUI application (using `eframe`/`egui`) that wraps the core library. It simulates hardware by rendering to a window. Use this to test effects without flashing a device.
*   **`tests/`**: Unit tests for math, color transitions, and logic that doesn't require hardware peripherals.
