//! Desktop preview app for myrtio-light-composer effects
//!
//! Renders LED strip effects in a window with interactive controls.
//! Uses the new Renderer + IntentChannel API for all state changes.

use std::time::Instant as StdInstant;

use eframe::egui::{self};
use myrtio_light_composer::{
    Duration, EffectId, FilterProcessorConfig, Instant, IntentChannel, IntentSender,
    LightChangeIntent, LightEngineConfig, LightStateIntent, Renderer, Rgb,
    TransitionTimings, U8Adjuster, bounds::RenderingBounds,
    filter::BrightnessFilterConfig, ws2812_lut,
};

/// Maximum number of LEDs the renderer supports
const MAX_LEDS: usize = 180;

/// Default number of LEDs in the simulated strip
const DEFAULT_LED_COUNT: usize = 60;

/// Size of each LED rectangle in pixels
const LED_SIZE: f32 = 12.0;

/// Gap between LEDs
const LED_GAP: f32 = 2.0;

/// Intent channel size
const INTENT_CHANNEL_SIZE: usize = 16;

/// Static intent channel for communication between UI and renderer
static INTENTS_CHANNEL: IntentChannel<INTENT_CHANNEL_SIZE> =
    IntentChannel::<INTENT_CHANNEL_SIZE>::new();

/// Default transition timings for preview (faster than production for responsiveness)
const PREVIEW_TRANSITION_TIMINGS: TransitionTimings = TransitionTimings {
    fade_out: Duration::from_millis(200),
    fade_in: Duration::from_millis(150),
    color_change: Duration::from_millis(100),
    brightness: Duration::from_millis(100),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// Render as a 1D strip, wrapped to available window width
    Strip,
    /// Render as multiple vertical lines (columns). The strip is linear; we just reshape it into a curtain.
    Curtain,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("Light Composer Preview"),
        ..Default::default()
    };

    eframe::run_native(
        "myrtio-light-preview",
        options,
        Box::new(|_cc| Ok(Box::new(PreviewApp::new()))),
    )
}

struct PreviewApp {
    /// The renderer instance
    renderer: Renderer<'static, MAX_LEDS, INTENT_CHANNEL_SIZE>,
    /// Intent sender for UI changes
    intent_sender: IntentSender<'static, INTENT_CHANNEL_SIZE>,

    // UI state (tracked to detect changes and send intents)
    /// Currently selected effect ID
    effect_id: EffectId,
    /// Synthetic time in milliseconds
    t_ms: u64,
    /// Wall-clock reference for delta time
    last_frame: StdInstant,
    /// Whether animation is playing
    playing: bool,
    /// Time scale multiplier (1.0 = realtime)
    time_scale: f32,
    /// Brightness (0-255)
    brightness: u8,
    /// Color for static/velvet effects (RGB)
    color: [u8; 3],
    /// Whether to apply WS2812 gamma correction (post-process)
    apply_gamma: bool,
    /// LED pixel size for display
    led_size: f32,
    /// Number of LEDs to display
    led_count: usize,
    /// Preview layout effect
    layout: Layout,
    /// How many identical lines to draw (used in `Layout::Lines`)
    lines: usize,
}

impl PreviewApp {
    fn new() -> Self {
        let initial_color = Rgb {
            r: 255,
            g: 180,
            b: 100,
        };
        let initial_effect = EffectId::RainbowMirrored;
        let initial_brightness: u8 = 255;
        let initial_led_count: u8 = DEFAULT_LED_COUNT as u8;

        let config = LightEngineConfig {
            effect: initial_effect,
            bounds: RenderingBounds {
                start: 0,
                end: initial_led_count,
            },
            filters: FilterProcessorConfig {
                brightness: BrightnessFilterConfig {
                    min_brightness: 0,
                    scale: 255,
                    adjust: None,
                },
                color_correction: Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
            },
            timings: PREVIEW_TRANSITION_TIMINGS,
            brightness: initial_brightness,
            color: initial_color,
        };

        let renderer = Renderer::<MAX_LEDS, INTENT_CHANNEL_SIZE>::new(
            INTENTS_CHANNEL.receiver(),
            &config,
        );
        let intent_sender = INTENTS_CHANNEL.sender();

        Self {
            renderer,
            intent_sender,
            effect_id: initial_effect,
            t_ms: 0,
            last_frame: StdInstant::now(),
            playing: true,
            time_scale: 1.0,
            brightness: initial_brightness,
            color: [initial_color.r, initial_color.g, initial_color.b],
            apply_gamma: false,
            led_size: LED_SIZE,
            led_count: DEFAULT_LED_COUNT,
            layout: Layout::Strip,
            lines: 6,
        }
    }

    /// Send an effect change intent
    fn send_effect_change(&self, effect_id: EffectId) {
        let intent = LightChangeIntent::State(LightStateIntent {
            effect_id: Some(effect_id),
            ..Default::default()
        });
        let _ = self.intent_sender.try_send(intent);
    }

    /// Send a color change intent
    fn send_color_change(&self, r: u8, g: u8, b: u8) {
        let intent = LightChangeIntent::State(LightStateIntent {
            color: Some(Rgb { r, g, b }),
            ..Default::default()
        });
        let _ = self.intent_sender.try_send(intent);
    }

    /// Send a brightness change intent
    fn send_brightness_change(&self, brightness: u8) {
        let intent = LightChangeIntent::State(LightStateIntent {
            brightness: Some(brightness),
            ..Default::default()
        });
        let _ = self.intent_sender.try_send(intent);
    }

    /// Send a brightness adjuster change intent
    fn send_brightness_adjuster_change(&self, adjuster: Option<U8Adjuster>) {
        let intent = LightChangeIntent::Adjuster(adjuster);
        let _ = self.intent_sender.try_send(intent);
    }

    /// Send a bounds change intent
    fn send_bounds_change(&self, led_count: u8) {
        let intent = LightChangeIntent::Bounds(RenderingBounds {
            start: 0,
            end: led_count,
        });
        let _ = self.intent_sender.try_send(intent);
    }

    /// Reset time to zero
    fn reset_time(&mut self) {
        self.t_ms = 0;
        self.last_frame = StdInstant::now();
    }

    /// Toggle playing state
    fn toggle_playing(&mut self) {
        self.playing = !self.playing;
    }

    /// Update synthetic time based on wall clock and time scale
    fn update_time(&mut self) {
        let now = StdInstant::now();
        let delta = now.duration_since(self.last_frame);
        self.last_frame = now;

        if self.playing {
            let delta_ms_f64 =
                delta.as_secs_f64() * 1000.0 * f64::from(self.time_scale);
            let delta_ms_f64 = if delta_ms_f64.is_finite() {
                #[allow(clippy::cast_precision_loss)]
                delta_ms_f64.clamp(0.0, u64::MAX as f64)
            } else {
                0.0
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let delta_ms = delta_ms_f64 as u64;
            self.t_ms = self.t_ms.wrapping_add(delta_ms);
        }
    }
}

impl eframe::App for PreviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update synthetic time
        self.update_time();

        // Render the frame using synthetic time
        let now = Instant::from_millis(self.t_ms);
        let frame = self.renderer.render(now).to_vec();

        // Request continuous repaint for animation
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // <PlaybackControls>
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("⏮ Reset").clicked() {
                            self.reset_time();
                        }
                        if ui
                            .button(if self.playing {
                                "⏸ Pause"
                            } else {
                                "▶ Play"
                            })
                            .clicked()
                        {
                            self.toggle_playing();
                        }

                        ui.add_space(8.0);
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        let secs = self.t_ms / 1000;
                        let ms = self.t_ms % 1000;
                        ui.label(format!("Time: {secs}.{ms:03}s"));
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(
                            egui::Slider::new(&mut self.time_scale, 0.1..=5.0)
                                .logarithmic(true),
                        );
                    });
                });
                // </PlaybackControls>
                ui.add_space(16.0);
                // <LayoutSelector>
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Size: ");
                        ui.add(egui::Slider::new(&mut self.led_size, 4.0..=32.0));
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Layout:");
                        ui.selectable_value(
                            &mut self.layout,
                            Layout::Strip,
                            "strip",
                        );
                        ui.selectable_value(
                            &mut self.layout,
                            Layout::Curtain,
                            "curtain",
                        );
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("LEDs:");
                        let old_led_count = self.led_count;
                        ui.add(egui::Slider::new(
                            &mut self.led_count,
                            1usize..=MAX_LEDS,
                        ));
                        if self.led_count != old_led_count {
                            #[allow(clippy::cast_possible_truncation)]
                            self.send_bounds_change(self.led_count as u8);
                        }

                        if self.layout == Layout::Curtain {
                            ui.add_space(8.0);

                            ui.label("Lines:");
                            let old_lines = self.lines;
                            ui.add(egui::Slider::new(
                                &mut self.lines,
                                1usize..=64usize,
                            ));
                            if self.lines != old_lines {
                                self.send_bounds_change(self.lines as u8);
                            }
                        }
                    });
                });
                // </LayoutControls>
            });

            ui.add_space(16.0);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Effect:");
                    let mut selected_effect = self.effect_id;
                    egui::ComboBox::from_id_salt("effect_selector")
                        .selected_text(self.effect_id.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::RainbowMirrored,
                                "rainbow",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::RainbowLong,
                                "rainbow_long",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::RainbowShort,
                                "rainbow_short",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::RainbowLongInverse,
                                "rainbow_long_inverse",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::RainbowShortInverse,
                                "rainbow_short_inverse",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::Static,
                                "static",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::VelvetAnalog,
                                "velvet_analog",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::Aurora,
                                "aurora",
                            );
                            ui.selectable_value(
                                &mut selected_effect,
                                EffectId::LavaLamp,
                                "lava_lamp",
                            );
                        });
                    if selected_effect != self.effect_id {
                        self.effect_id = selected_effect;
                        self.send_effect_change(selected_effect);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let old_color = self.color;
                    if ui.color_edit_button_srgb(&mut self.color).changed()
                        && old_color != self.color
                    {
                        self.send_color_change(
                            self.color[0],
                            self.color[1],
                            self.color[2],
                        );
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Brightness:");
                    let old_brightness = self.brightness;
                    ui.add(
                        egui::DragValue::new(&mut self.brightness)
                            .range(0u8..=255u8),
                    );
                    if self.brightness != old_brightness {
                        self.send_brightness_change(self.brightness);
                    }

                    ui.add_space(8.0);

                    let old_apply_gamma = self.apply_gamma;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.apply_gamma, "WS2812 Gamma");
                    });
                    if self.apply_gamma != old_apply_gamma {
                        let adjuster: Option<U8Adjuster> = if self.apply_gamma {
                            Some(ws2812_lut)
                        } else {
                            None
                        };
                        self.send_brightness_adjuster_change(adjuster);
                    }
                });
            });

            ui.add_space(16.0);

            // === LED Display ===
            let available_width = ui.available_width();
            let led_pitch = self.led_size + LED_GAP;

            match self.layout {
                Layout::Strip => {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss
                    )]
                    let leds_per_row =
                        (available_width / led_pitch).floor().max(1.0) as usize;
                    let rows = self.led_count.div_ceil(leds_per_row);
                    #[allow(clippy::cast_precision_loss)]
                    let height = rows as f32 * led_pitch;

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(available_width, height),
                        egui::Sense::hover(),
                    );
                    let origin = response.rect.min;

                    #[allow(clippy::cast_precision_loss)]
                    for (i, pixel) in frame.iter().enumerate() {
                        let row = i / leds_per_row;
                        let col = i % leds_per_row;
                        let x = origin.x + col as f32 * led_pitch;
                        let y = origin.y + row as f32 * led_pitch;

                        let rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(self.led_size, self.led_size),
                        );
                        let color =
                            egui::Color32::from_rgb(pixel.r, pixel.g, pixel.b);
                        painter.rect_filled(rect, 3.0, color);
                    }
                }
                Layout::Curtain => {
                    let per_line = self.led_count.max(1);
                    let line_count = self.lines.max(1);

                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss
                    )]
                    let lines_per_row =
                        (available_width / led_pitch).floor().max(1.0) as usize;
                    let block_rows = line_count.div_ceil(lines_per_row);

                    #[allow(clippy::cast_precision_loss)]
                    let height = (block_rows * per_line) as f32 * led_pitch;

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(available_width, height),
                        egui::Sense::hover(),
                    );
                    let origin = response.rect.min;

                    #[allow(clippy::cast_precision_loss)]
                    for line in 0..line_count {
                        let block_row = line / lines_per_row;
                        let block_col = line % lines_per_row;

                        for (offset, pixel) in frame.iter().enumerate() {
                            let x = origin.x + block_col as f32 * led_pitch;
                            let y = origin.y
                                + (block_row * per_line + offset) as f32 * led_pitch;

                            let rect = egui::Rect::from_min_size(
                                egui::pos2(x, y),
                                egui::vec2(self.led_size, self.led_size),
                            );
                            let color =
                                egui::Color32::from_rgb(pixel.r, pixel.g, pixel.b);
                            painter.rect_filled(rect, 2.0, color);
                        }
                    }
                }
            }
        });
    }
}
