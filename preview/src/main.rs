//! Desktop preview app for myrtio-light-composer modes
//!
//! Renders LED strip modes in a window with interactive controls.

use std::time::Instant;

use eframe::egui;
use embassy_time::Instant as EmbassyInstant;
use myrtio_light_composer::{
    mode::{ModeId, ModeSlot},
    ws2812_lut, Rgb,
};

/// Number of LEDs in the simulated strip
const LED_COUNT: usize = 60;

/// Size of each LED rectangle in pixels
const LED_SIZE: f32 = 12.0;

/// Gap between LEDs
const LED_GAP: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// Render as a 1D strip, wrapped to available window width
    Strip,
    /// Render as multiple vertical lines (columns). The strip is linear; we just reshape it.
    Lines,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 400.0])
            .with_title("Light Preview"),
        ..Default::default()
    };

    eframe::run_native(
        "myrtio-light-preview",
        options,
        Box::new(|_cc| Ok(Box::new(PreviewApp::new()))),
    )
}

struct PreviewApp {
    /// Current mode slot
    mode: ModeSlot,
    /// Currently selected mode ID (UI state)
    mode_id: ModeId,
    /// Synthetic time in milliseconds
    t_ms: u64,
    /// Wall-clock reference for delta time
    last_frame: Instant,
    /// Whether animation is playing
    playing: bool,
    /// Time scale multiplier (1.0 = realtime)
    time_scale: f32,
    /// Brightness (0-255)
    brightness: u8,
    /// Color for static/velvet modes (RGB)
    color: [u8; 3],
    /// Whether to apply WS2812 gamma correction
    apply_gamma: bool,
    /// LED pixel size
    led_size: f32,
    /// Number of LEDs to display
    led_count: usize,
    /// Preview layout mode
    layout: Layout,
    /// How many identical lines to draw (used in `Layout::Lines`)
    lines: usize,
}

impl PreviewApp {
    fn new() -> Self {
        let color = Rgb {
            r: 255,
            g: 180,
            b: 100,
        };
        let mode_id = ModeId::Rainbow;
        Self {
            mode: mode_id.to_mode_slot(color),
            mode_id,
            t_ms: 0,
            last_frame: Instant::now(),
            playing: true,
            time_scale: 1.0,
            brightness: 255,
            color: [color.r, color.g, color.b],
            apply_gamma: false,
            led_size: LED_SIZE,
            led_count: LED_COUNT,
            layout: Layout::Strip,
            lines: 6,
        }
    }

    fn set_mode(&mut self, mode_id: ModeId) {
        self.mode_id = mode_id;
        let color = Rgb {
            r: self.color[0],
            g: self.color[1],
            b: self.color[2],
        };
        self.mode = mode_id.to_mode_slot(color);
    }

    fn reset_time(&mut self) {
        self.t_ms = 0;
        self.last_frame = Instant::now();
    }

    fn render_frame(&mut self) -> Vec<Rgb> {
        let now = EmbassyInstant::from_millis(self.t_ms);

        // Dispatch based on LED count (use a reasonable max)
        let frame: Vec<Rgb> = match self.led_count {
            1..=30 => self.render_with_count::<30>(now),
            31..=60 => self.render_with_count::<60>(now),
            61..=120 => self.render_with_count::<120>(now),
            _ => self.render_with_count::<180>(now),
        };

        // Truncate to actual count and apply post-processing
        frame
            .into_iter()
            .take(self.led_count)
            .map(|mut pixel| {
                // Apply brightness
                pixel.r = scale8(pixel.r, self.brightness);
                pixel.g = scale8(pixel.g, self.brightness);
                pixel.b = scale8(pixel.b, self.brightness);
                // Apply gamma if enabled
                if self.apply_gamma {
                    pixel.r = ws2812_lut(pixel.r);
                    pixel.g = ws2812_lut(pixel.g);
                    pixel.b = ws2812_lut(pixel.b);
                }
                pixel
            })
            .collect()
    }

    fn render_with_count<const N: usize>(&mut self, now: EmbassyInstant) -> Vec<Rgb> {
        let frame: [Rgb; N] = self.mode.render(now);
        frame.to_vec()
    }
}

/// Scale a u8 value by another u8 (0-255 treated as 0.0-1.0)
#[inline]
fn scale8(value: u8, scale: u8) -> u8 {
    ((u16::from(value) * u16::from(scale)) >> 8) as u8
}

impl eframe::App for PreviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update time
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame);
        self.last_frame = now;

        if self.playing {
            let delta_ms_f64 = delta.as_secs_f64() * 1000.0 * f64::from(self.time_scale);
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

        // Render the frame
        let frame = self.render_frame();

        // Request continuous repaint for animation
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Light Preview");
            ui.add_space(8.0);

            // Controls
            ui.horizontal(|ui| {
                // Mode selector (use temp variable to detect changes reliably)
                ui.label("Mode:");
                let mut selected_mode = self.mode_id;
                egui::ComboBox::from_id_salt("mode_selector")
                    .selected_text(self.mode_id.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected_mode, ModeId::Rainbow, "rainbow");
                        ui.selectable_value(&mut selected_mode, ModeId::Static, "static");
                        ui.selectable_value(
                            &mut selected_mode,
                            ModeId::VelvetAnalog,
                            "velvet_analog",
                        );
                    });
                if selected_mode != self.mode_id {
                    self.set_mode(selected_mode);
                }

                ui.add_space(16.0);

                // Play/Pause
                if ui.button(if self.playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    self.playing = !self.playing;
                }

                if ui.button("⏮ Reset").clicked() {
                    self.reset_time();
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Layout:");
                ui.selectable_value(&mut self.layout, Layout::Strip, "strip");
                ui.selectable_value(&mut self.layout, Layout::Lines, "lines");

                if self.layout == Layout::Lines {
                    ui.add_space(16.0);
                    ui.label("Lines:");
                    ui.add(egui::Slider::new(&mut self.lines, 1usize..=64usize));
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Color picker (for static/velvet modes)
                ui.label("Color:");
                if ui.color_edit_button_srgb(&mut self.color).changed() {
                    // Update mode color
                    let rgb = Rgb {
                        r: self.color[0],
                        g: self.color[1],
                        b: self.color[2],
                    };
                    self.mode = self.mode_id.to_mode_slot(rgb);
                }

                ui.add_space(16.0);

                // Brightness
                ui.label("Brightness:");
                ui.add(egui::Slider::new(&mut self.brightness, 0u8..=255u8));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Time scale
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.time_scale, 0.1..=5.0).logarithmic(true));

                ui.add_space(16.0);

                // LED count
                ui.label("LEDs:");
                ui.add(egui::Slider::new(&mut self.led_count, 1usize..=180usize));

                ui.add_space(16.0);

                // LED size
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut self.led_size, 4.0..=32.0));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.apply_gamma, "WS2812 Gamma");

                ui.add_space(16.0);

                let secs = self.t_ms / 1000;
                let ms = self.t_ms % 1000;
                ui.label(format!("Time: {secs}.{ms:03}s"));
            });

            ui.add_space(16.0);

            // Draw LEDs
            let available_width = ui.available_width();
            let led_pitch = self.led_size + LED_GAP;

            match self.layout {
                Layout::Strip => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let leds_per_row = (available_width / led_pitch).floor().max(1.0) as usize;
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
                        let color = egui::Color32::from_rgb(pixel.r, pixel.g, pixel.b);
                        painter.rect_filled(rect, 2.0, color);
                    }
                }
                Layout::Lines => {
                    // In Lines layout we render a single line (the strip) and repeat it for each line.
                    let per_line = self.led_count.max(1);
                    let line_count = self.lines.max(1);

                    // How many columns (lines) can we fit per visual row?
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let lines_per_row = (available_width / led_pitch).floor().max(1.0) as usize;
                    let block_rows = line_count.div_ceil(lines_per_row);

                    #[allow(clippy::cast_precision_loss)]
                    let height = (block_rows * per_line) as f32 * led_pitch;

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(available_width, height),
                        egui::Sense::hover(),
                    );
                    let origin = response.rect.min;

                    // Draw repeated lines: same colors and same length as the first line.
                    #[allow(clippy::cast_precision_loss)]
                    for line in 0..line_count {
                        let block_row = line / lines_per_row;
                        let block_col = line % lines_per_row;

                        for (offset, pixel) in frame.iter().enumerate() {
                            let x = origin.x + block_col as f32 * led_pitch;
                            let y = origin.y + (block_row * per_line + offset) as f32 * led_pitch;

                            let rect = egui::Rect::from_min_size(
                                egui::pos2(x, y),
                                egui::vec2(self.led_size, self.led_size),
                            );
                            let color = egui::Color32::from_rgb(pixel.r, pixel.g, pixel.b);
                            painter.rect_filled(rect, 2.0, color);
                        }
                    }
                }
            }
        });
    }
}