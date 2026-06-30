//! md2pdf-gui — a small native window for converting a Markdown file to PDF.
//!
//! Flow: choose a `.md` file, adjust the output file name (defaulted to the
//! same folder with a `.pdf` extension), pick a paper size, then convert.

// On release builds, don't spawn a console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui;
use egui::Color32;
use md2pdf::render::{self, PaperSize, RenderOptions};

// --- Palette ----------------------------------------------------------------
const BG: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);
const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
const BORDER: Color32 = Color32::from_rgb(0xE2, 0xE5, 0xEA);
const INK: Color32 = Color32::from_rgb(0x1E, 0x24, 0x2E);
const MUTED: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);
const ACCENT: Color32 = Color32::from_rgb(0x2F, 0x6B, 0xED);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0x21, 0x57, 0xD0);
const ACCENT_SOFT: Color32 = Color32::from_rgb(0xEA, 0xF1, 0xFE);
const SUCCESS: Color32 = Color32::from_rgb(0x12, 0x8A, 0x3C);
const DANGER: Color32 = Color32::from_rgb(0xC8, 0x2A, 0x2A);

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 560.0])
            .with_min_inner_size([520.0, 500.0])
            .with_title("md2pdf"),
        ..Default::default()
    };
    eframe::run_native(
        "md2pdf",
        options,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            Box::<App>::default()
        }),
    )
}

/// Apply a clean, modern light theme: rounded widgets, comfortable spacing,
/// and a blue accent.
fn setup_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, Rounding, Stroke, TextStyle};

    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.interact_size.y = 30.0;

    let v = &mut style.visuals;
    v.dark_mode = false;
    v.override_text_color = Some(INK);
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = Color32::WHITE; // text-edit background
    v.faint_bg_color = ACCENT_SOFT;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let rounding = Rounding::same(9.0);
    v.widgets.noninteractive.rounding = rounding;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.rounding = rounding;
    v.widgets.inactive.bg_fill = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Color32::WHITE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);
    v.widgets.hovered.rounding = rounding;
    v.widgets.hovered.bg_fill = ACCENT_SOFT;
    v.widgets.hovered.weak_bg_fill = ACCENT_SOFT;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, INK);
    v.widgets.active.rounding = rounding;
    v.widgets.active.bg_fill = ACCENT_SOFT;
    v.widgets.active.weak_bg_fill = ACCENT_SOFT;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.open.rounding = rounding;

    ctx.set_style(style);
}

/// A rounded white "card" container.
fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A filled accent (primary) button that stretches to the available width.
fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let text = egui::RichText::new(label)
        .color(Color32::WHITE)
        .size(16.0)
        .strong();
    let fill = if enabled { ACCENT } else { Color32::from_rgb(0xB8, 0xC6, 0xE6) };
    let button = egui::Button::new(text)
        .fill(fill)
        .rounding(egui::Rounding::same(10.0))
        .min_size(egui::vec2(ui.available_width(), 46.0));
    let resp = ui.add_enabled(enabled, button);
    if resp.hovered() && enabled {
        // Repaint with a darker shade on hover.
        ui.painter().rect_filled(
            resp.rect,
            egui::Rounding::same(10.0),
            ACCENT_HOVER.gamma_multiply(0.18),
        );
    }
    resp
}

#[derive(Default)]
struct App {
    input: Option<PathBuf>,
    output_name: String,
    paper: PaperState,
    status: Status,
    last_output: Option<PathBuf>,
}

struct PaperState(PaperSize);
impl Default for PaperState {
    fn default() -> Self {
        PaperState(PaperSize::A4)
    }
}

#[derive(Default)]
enum Status {
    #[default]
    Idle,
    Ok(String),
    Err(String),
}

impl App {
    /// Default output file name for a given input: same stem, `.pdf` suffix.
    fn default_output_name(input: &std::path::Path) -> String {
        input
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| format!("{s}.pdf"))
            .unwrap_or_else(|| "output.pdf".to_string())
    }

    /// Adopt `path` as the current input and reset derived state.
    fn set_input(&mut self, path: PathBuf) {
        self.output_name = Self::default_output_name(&path);
        self.input = Some(path);
        self.status = Status::Idle;
        self.last_output = None;
    }

    fn pick_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose a Markdown file")
            .add_filter("Markdown", &["md", "markdown", "mkd", "mdown", "text", "txt"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.set_input(path);
        }
    }

    /// Pick up any file dropped onto the window (first one with a path wins).
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = dropped.into_iter().find_map(|f| f.path) {
            self.set_input(path);
        }
    }

    /// Full path the PDF will be written to: the input's folder + output name.
    fn output_path(&self) -> Option<PathBuf> {
        let input = self.input.as_ref()?;
        let dir = input.parent().map(PathBuf::from).unwrap_or_default();
        let mut name = self.output_name.trim().to_string();
        if name.is_empty() {
            name = Self::default_output_name(input);
        }
        if !name.to_lowercase().ends_with(".pdf") {
            name.push_str(".pdf");
        }
        Some(dir.join(name))
    }

    fn convert(&mut self) {
        let Some(input) = self.input.clone() else {
            self.status = Status::Err("No input file selected.".into());
            return;
        };
        let Some(output) = self.output_path() else {
            self.status = Status::Err("Could not determine output path.".into());
            return;
        };

        let markdown = match std::fs::read_to_string(&input) {
            Ok(s) => s,
            Err(e) => {
                self.status = Status::Err(format!("Failed to read input: {e}"));
                return;
            }
        };

        let title = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Document")
            .to_string();

        let opts = RenderOptions {
            title,
            paper: self.paper.0,
            margin_mm: 20.0,
            base_font_size: 11,
        };

        match render::markdown_to_pdf(&markdown, &output, &opts) {
            Ok(()) => {
                self.status = Status::Ok(format!("Saved {}", output.display()));
                self.last_output = Some(output);
            }
            Err(e) => self.status = Status::Err(format!("Conversion failed: {e}")),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Accept files dropped anywhere on the window.
        self.handle_dropped_files(ctx);
        let hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::same(22.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;

                // --- Header ------------------------------------------------
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("md2pdf").size(28.0).strong().color(INK));
                });
                ui.label(
                    egui::RichText::new("Convert Markdown to a polished PDF")
                        .size(14.0)
                        .color(MUTED),
                );

                // --- Drop zone / selected file -----------------------------
                self.input_card(ui, hovering_file);

                // --- Output options ----------------------------------------
                self.output_card(ui);

                ui.add_space(2.0);

                // --- Primary action ----------------------------------------
                let can_convert = self.input.is_some();
                if primary_button(ui, "Convert to PDF", can_convert).clicked() {
                    self.convert();
                }

                // --- Result actions + status -------------------------------
                if let Some(out) = self.last_output.clone() {
                    ui.horizontal(|ui| {
                        if ui.button("📂  Open PDF").clicked() {
                            let _ = std::process::Command::new("open").arg(&out).spawn();
                        }
                        if ui.button("🔍  Reveal in Finder").clicked() {
                            let _ = std::process::Command::new("open")
                                .arg("-R")
                                .arg(&out)
                                .spawn();
                        }
                    });
                }

                self.status_bar(ui);
            });
    }
}

impl App {
    fn input_card(&mut self, ui: &mut egui::Ui, hovering_file: bool) {
        match self.input.clone() {
            None => {
                // Empty drop zone — highlighted while a file hovers.
                let (fill, stroke_col) = if hovering_file {
                    (ACCENT_SOFT, ACCENT)
                } else {
                    (Color32::from_rgb(0xFB, 0xFC, 0xFD), BORDER)
                };
                egui::Frame::none()
                    .fill(fill)
                    .stroke(egui::Stroke::new(if hovering_file { 2.0 } else { 1.5 }, stroke_col))
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 26.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("⬇").size(34.0).color(ACCENT));
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Drag & drop a Markdown file here")
                                    .size(16.0)
                                    .strong()
                                    .color(INK),
                            );
                            ui.label(egui::RichText::new("or").size(13.0).color(MUTED));
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Browse…").color(ACCENT).strong(),
                                    )
                                    .fill(Color32::WHITE)
                                    .stroke(egui::Stroke::new(1.0, ACCENT))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                self.pick_file();
                            }
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(".md  .markdown  .txt")
                                    .size(12.0)
                                    .color(MUTED),
                            );
                        });
                    });
            }
            Some(path) => {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📄").size(26.0));
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            let name = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("file");
                            ui.label(egui::RichText::new(name).size(15.0).strong().color(INK));
                            let dir = path
                                .parent()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            ui.label(egui::RichText::new(dir).size(12.0).color(MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Change…").clicked() {
                                self.pick_file();
                            }
                        });
                    });
                });
            }
        }
    }

    fn output_card(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.input.is_some(), |ui| {
            card(ui, |ui| {
                ui.label(egui::RichText::new("OUTPUT").size(11.5).strong().color(MUTED));
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label("File name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.output_name)
                            .desired_width(f32::INFINITY)
                            .hint_text("output.pdf"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Paper size");
                    egui::ComboBox::from_id_source("paper")
                        .selected_text(self.paper.0.label())
                        .show_ui(ui, |ui| {
                            for size in PaperSize::ALL {
                                ui.selectable_value(&mut self.paper.0, size, size.label());
                            }
                        });
                });

                if let Some(out) = self.output_path() {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!("→ saves to  {}", out.display()))
                            .size(12.0)
                            .color(MUTED),
                    );
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        match &self.status {
            Status::Idle => {}
            Status::Ok(msg) => {
                egui::Frame::none()
                    .fill(Color32::from_rgb(0xEC, 0xF8, 0xF0))
                    .stroke(egui::Stroke::new(1.0, SUCCESS.gamma_multiply(0.5)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(format!("✔  {msg}")).color(SUCCESS));
                    });
            }
            Status::Err(msg) => {
                egui::Frame::none()
                    .fill(Color32::from_rgb(0xFD, 0xEC, 0xEC))
                    .stroke(egui::Stroke::new(1.0, DANGER.gamma_multiply(0.5)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(format!("✖  {msg}")).color(DANGER));
                    });
            }
        }
    }
}
