use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;

use crate::cleaner::ScanResult;
use crate::disk_info::{self, DiskInfo};
use crate::utils;

// ── Color palette ──────────────────────────────────────────────────────

const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(28, 28, 38);
const CARD_FILL: egui::Color32 = egui::Color32::from_rgb(30, 30, 42);
const CARD_EXPANDED: egui::Color32 = egui::Color32::from_rgb(35, 35, 48);
const INSET_FILL: egui::Color32 = egui::Color32::from_rgb(25, 25, 35);
const BORDER: egui::Color32 = egui::Color32::from_rgb(50, 50, 65);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(60, 140, 220);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(225, 225, 235);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(140, 140, 160);
const GREEN: egui::Color32 = egui::Color32::from_rgb(80, 220, 120);
const RED: egui::Color32 = egui::Color32::from_rgb(190, 45, 45);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(220, 180, 50);
const TITLE_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 180, 220);

// ── Icon mapping ───────────────────────────────────────────────────────

fn icon_for_category(name: &str) -> (&'static str, egui::Color32) {
    match name {
        "system-caches" => ("C", egui::Color32::from_rgb(100, 160, 230)),
        "app-logs" => ("L", egui::Color32::from_rgb(220, 140, 60)),
        "browser-caches" => ("B", egui::Color32::from_rgb(80, 190, 120)),
        "xcode" => ("X", egui::Color32::from_rgb(60, 140, 220)),
        "xcode-device-support" => ("D", egui::Color32::from_rgb(140, 100, 220)),
        "xcode-archives" => ("A", egui::Color32::from_rgb(220, 100, 140)),
        "core-simulator" => ("S", egui::Color32::from_rgb(60, 200, 200)),
        "homebrew" => ("H", egui::Color32::from_rgb(220, 180, 50)),
        "package-managers" => ("P", egui::Color32::from_rgb(180, 120, 60)),
        "trash" => ("T", egui::Color32::from_rgb(190, 60, 60)),
        "ds-store" => (".", egui::Color32::from_rgb(140, 140, 160)),
        "large-files" => ("F", egui::Color32::from_rgb(200, 80, 200)),
        _ => ("?", egui::Color32::from_rgb(140, 140, 160)),
    }
}

fn paint_icon(ui: &mut egui::Ui, letter: &str, color: egui::Color32) {
    let size = 28.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 7.0, color);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        letter,
        egui::FontId::proportional(14.0),
        egui::Color32::WHITE,
    );
}

// ── Types ──────────────────────────────────────────────────────────────

pub struct CategoryState {
    pub name: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub icon_color: egui::Color32,
    pub selected: bool,
    pub expanded: bool,
    pub scan_result: Option<ScanResult>,
    pub entry_selected: Vec<bool>,
    pub is_report_only: bool,
}

impl CategoryState {
    fn selected_bytes(&self) -> u64 {
        match &self.scan_result {
            Some(r) => r
                .entries
                .iter()
                .zip(self.entry_selected.iter())
                .filter(|(_, s)| **s)
                .map(|(e, _)| e.size_bytes)
                .sum(),
            None => 0,
        }
    }

    fn selected_count(&self) -> usize {
        self.entry_selected.iter().filter(|s| **s).count()
    }

    fn entry_count(&self) -> usize {
        self.scan_result.as_ref().map(|r| r.entries.len()).unwrap_or(0)
    }

    fn set_all_entries(&mut self, val: bool) {
        for s in &mut self.entry_selected {
            *s = val;
        }
    }

    fn sync_category_from_entries(&mut self) {
        if !self.is_report_only {
            self.selected = self.entry_selected.iter().any(|s| *s);
        }
    }
}

struct DeleteItem {
    category_name: String,
    path: PathBuf,
    #[allow(dead_code)]
    size_bytes: u64,
}

pub enum BgMessage {
    ScanComplete(String, ScanResult),
    AllScansComplete,
    DeletedFile(String, PathBuf, u64),
    DeleteError(String, PathBuf, String),
    AllCleansComplete,
    Progress(String),
}

#[derive(PartialEq)]
pub enum AppPhase {
    Idle,
    Scanning,
    Cleaning,
}

pub struct ConfirmDialog {
    pub visible: bool,
    pub total_bytes: u64,
    pub file_count: usize,
    pub category_names: Vec<String>,
}

pub struct TidyMacApp {
    categories: Vec<CategoryState>,
    phase: AppPhase,
    receiver: Option<mpsc::Receiver<BgMessage>>,
    progress_label: String,
    confirm_dialog: ConfirmDialog,
    errors: Vec<String>,
    cleaned_bytes: u64,
    about_visible: bool,
    disk_info: Option<DiskInfo>,
}

// ── App impl ───────────────────────────────────────────────────────────

impl TidyMacApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // ── Custom dark theme ──
        let mut style = (*cc.egui_ctx.style()).clone();
        let mut visuals = egui::Visuals::dark();

        let bg_dark = egui::Color32::from_rgb(20, 20, 28);
        let bg_widget = egui::Color32::from_rgb(40, 40, 55);
        let bg_widget_hover = egui::Color32::from_rgb(50, 50, 68);
        let bg_widget_active = egui::Color32::from_rgb(60, 60, 80);

        visuals.panel_fill = BG_PANEL;
        visuals.window_fill = bg_dark;
        visuals.extreme_bg_color = bg_dark;
        visuals.faint_bg_color = egui::Color32::from_rgb(35, 35, 48);

        visuals.widgets.inactive.bg_fill = bg_widget;
        visuals.widgets.inactive.weak_bg_fill = bg_widget;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = bg_widget_hover;
        visuals.widgets.hovered.weak_bg_fill = bg_widget_hover;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);

        visuals.widgets.active.bg_fill = bg_widget_active;
        visuals.widgets.active.weak_bg_fill = bg_widget_active;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, ACCENT);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, TEXT_PRIMARY);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

        visuals.widgets.open.bg_fill = bg_widget_active;
        visuals.widgets.open.weak_bg_fill = bg_widget_active;
        visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);

        visuals.widgets.noninteractive.bg_fill = BG_PANEL;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, BORDER);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);

        visuals.selection.bg_fill = ACCENT;
        visuals.selection.stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);

        visuals.window_corner_radius = egui::CornerRadius::same(12);
        visuals.window_stroke = egui::Stroke::new(1.0, BORDER);

        use egui::{FontId, TextStyle};
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(26.0));
        style.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(11.0));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        style.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));

        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(16);

        style.visuals = visuals;
        cc.egui_ctx.set_style(style);

        // ── Build categories ──
        let cleaners = crate::categories::all_cleaners(104_857_600, None);
        let categories: Vec<CategoryState> = cleaners
            .iter()
            .map(|c| {
                let (icon, icon_color) = icon_for_category(c.name());
                CategoryState {
                name: c.name(),
                label: c.label(),
                icon,
                icon_color,
                selected: c.name() != "large-files",
                expanded: false,
                scan_result: None,
                entry_selected: vec![],
                is_report_only: c.name() == "large-files",
            }})
            .collect();

        Self {
            categories,
            phase: AppPhase::Idle,
            receiver: None,
            progress_label: String::new(),
            confirm_dialog: ConfirmDialog {
                visible: false,
                total_bytes: 0,
                file_count: 0,
                category_names: vec![],
            },
            errors: vec![],
            cleaned_bytes: 0,
            about_visible: false,
            disk_info: disk_info::get_disk_info(),
        }
    }

    // ── Background operations ──────────────────────────────────────────

    fn start_scan(&mut self) {
        self.phase = AppPhase::Scanning;
        self.progress_label = "Starting scan...".to_string();
        self.errors.clear();
        self.cleaned_bytes = 0;

        for cat in &mut self.categories {
            cat.scan_result = None;
            cat.entry_selected.clear();
        }

        let (tx, rx) = mpsc::channel::<BgMessage>();
        self.receiver = Some(rx);

        std::thread::spawn(move || {
            let cleaners = crate::categories::all_cleaners(104_857_600, None);
            for cleaner in &cleaners {
                let _ = tx.send(BgMessage::Progress(cleaner.label().to_string()));
                let result = cleaner.scan();
                let _ = tx.send(BgMessage::ScanComplete(cleaner.name().to_string(), result));
            }
            let _ = tx.send(BgMessage::AllScansComplete);
        });
    }

    fn start_clean(&mut self) {
        self.phase = AppPhase::Cleaning;
        self.progress_label = "Starting cleanup...".to_string();
        self.confirm_dialog.visible = false;
        self.cleaned_bytes = 0;

        let mut items: Vec<DeleteItem> = Vec::new();
        for cat in &self.categories {
            if !cat.selected || cat.is_report_only {
                continue;
            }
            if let Some(ref result) = cat.scan_result {
                for (entry, sel) in result.entries.iter().zip(cat.entry_selected.iter()) {
                    if *sel {
                        items.push(DeleteItem {
                            category_name: cat.name.to_string(),
                            path: entry.path.clone(),
                            size_bytes: entry.size_bytes,
                        });
                    }
                }
            }
        }

        let (tx, rx) = mpsc::channel::<BgMessage>();
        self.receiver = Some(rx);

        std::thread::spawn(move || {
            for item in &items {
                let _ = tx.send(BgMessage::Progress(format!(
                    "Deleting: {}",
                    item.path.display()
                )));
                match utils::safe_remove(&item.path) {
                    Ok(freed) => {
                        let _ = tx.send(BgMessage::DeletedFile(
                            item.category_name.clone(),
                            item.path.clone(),
                            freed,
                        ));
                    }
                    Err(e) => {
                        let _ = tx.send(BgMessage::DeleteError(
                            item.category_name.clone(),
                            item.path.clone(),
                            e.to_string(),
                        ));
                    }
                }
            }
            let _ = tx.send(BgMessage::AllCleansComplete);
        });
    }

    fn drain_messages(&mut self) {
        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    BgMessage::Progress(label) => {
                        self.progress_label = label;
                    }
                    BgMessage::ScanComplete(name, result) => {
                        if let Some(cat) = self.categories.iter_mut().find(|c| c.name == name) {
                            let count = result.entries.len();
                            cat.scan_result = Some(result);
                            cat.entry_selected = vec![true; count];
                        }
                    }
                    BgMessage::AllScansComplete => {
                        self.phase = AppPhase::Idle;
                        self.progress_label.clear();
                    }
                    BgMessage::DeletedFile(cat_name, path, freed) => {
                        self.cleaned_bytes += freed;
                        if let Some(cat) = self.categories.iter_mut().find(|c| c.name == cat_name) {
                            if let Some(ref mut result) = cat.scan_result {
                                if let Some(idx) = result.entries.iter().position(|e| e.path == path)
                                {
                                    result.entries.remove(idx);
                                    cat.entry_selected.remove(idx);
                                    result.total_bytes =
                                        result.entries.iter().map(|e| e.size_bytes).sum();
                                }
                            }
                        }
                    }
                    BgMessage::DeleteError(_cat_name, path, err) => {
                        self.errors
                            .push(format!("Failed to delete {}: {err}", path.display()));
                    }
                    BgMessage::AllCleansComplete => {
                        self.phase = AppPhase::Idle;
                        self.progress_label.clear();
                        self.disk_info = disk_info::get_disk_info();
                    }
                }
            }
        }
    }

    fn show_confirm_dialog(&mut self) {
        let mut total_bytes = 0u64;
        let mut file_count = 0usize;
        let mut category_names = Vec::new();

        for cat in &self.categories {
            if !cat.selected || cat.is_report_only {
                continue;
            }
            let sel_count = cat.selected_count();
            if sel_count > 0 {
                total_bytes += cat.selected_bytes();
                file_count += sel_count;
                category_names.push(format!(
                    "{} {} ({} items, {})",
                    cat.icon,
                    cat.label,
                    sel_count,
                    utils::format_size(cat.selected_bytes())
                ));
            }
        }

        self.confirm_dialog = ConfirmDialog {
            visible: true,
            total_bytes,
            file_count,
            category_names,
        };
    }

    // ── Rendering ──────────────────────────────────────────────────────

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() - 50.0);
            let about_btn = egui::Button::new(
                egui::RichText::new("i")
                    .size(14.0)
                    .strong()
                    .color(ACCENT),
            )
            .corner_radius(egui::CornerRadius::same(12))
            .min_size(egui::vec2(24.0, 24.0));
            if ui.add(about_btn).on_hover_text("About TidyMac").clicked() {
                self.about_visible = true;
            }
        });
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("TidyMac")
                    .size(32.0)
                    .strong()
                    .color(TITLE_BLUE),
            );
            ui.label(
                egui::RichText::new("macOS Cleanup Tool")
                    .size(13.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 2.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 1.0, ACCENT);
        });
        ui.add_space(12.0);
    }

    fn render_disk_bar(&self, ui: &mut egui::Ui) {
        let Some(ref info) = self.disk_info else {
            return;
        };

        egui::Frame::NONE
            .fill(CARD_FILL)
            .corner_radius(egui::CornerRadius::same(10))
            .stroke(egui::Stroke::new(0.5, BORDER))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Labels row
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Disk Usage")
                            .size(12.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} free of {}",
                                utils::format_size(info.available),
                                utils::format_size(info.total),
                            ))
                            .size(11.0)
                            .color(TEXT_SECONDARY),
                        );
                    });
                });

                ui.add_space(6.0);

                // Bar
                let bar_height = 14.0;
                let (bar_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), bar_height),
                    egui::Sense::hover(),
                );
                let painter = ui.painter();

                // Background (free space)
                painter.rect_filled(bar_rect, 7.0, egui::Color32::from_rgb(40, 40, 55));

                // Used portion
                let pct = info.usage_percent();
                let used_width = bar_rect.width() * pct;
                let used_rect = egui::Rect::from_min_size(
                    bar_rect.min,
                    egui::vec2(used_width, bar_height),
                );

                let bar_color = if pct < 0.6 {
                    GREEN
                } else if pct < 0.8 {
                    YELLOW
                } else {
                    egui::Color32::from_rgb(220, 60, 60)
                };
                painter.rect_filled(used_rect, 7.0, bar_color);

                ui.add_space(4.0);

                // Used / Total text
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Used: {}",
                            utils::format_size(info.used),
                        ))
                        .size(11.0)
                        .color(bar_color),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", pct * 100.0))
                                .size(11.0)
                                .strong()
                                .color(bar_color),
                        );
                    });
                });
            });

        ui.add_space(6.0);
    }

    fn render_action_bar(&mut self, ui: &mut egui::Ui) {
        let is_busy = self.phase != AppPhase::Idle;

        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Scan All button (blue)
            let scan_btn = egui::Button::new(
                egui::RichText::new("Scan All")
                    .size(15.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(if is_busy {
                egui::Color32::from_rgb(40, 70, 100)
            } else {
                egui::Color32::from_rgb(45, 120, 200)
            })
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(130.0, 36.0));

            if ui.add_enabled(!is_busy, scan_btn).clicked() {
                self.start_scan();
            }

            ui.add_space(4.0);

            // Select/Deselect All
            let all_selected = self
                .categories
                .iter()
                .filter(|c| !c.is_report_only)
                .all(|c| c.selected);
            let toggle_label = if all_selected { "Deselect All" } else { "Select All" };
            let toggle_btn = egui::Button::new(
                egui::RichText::new(toggle_label)
                    .size(13.0)
                    .color(egui::Color32::from_rgb(180, 180, 200)),
            )
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(100.0, 36.0));

            if ui.add(toggle_btn).clicked() {
                let new_val = !all_selected;
                for cat in &mut self.categories {
                    if !cat.is_report_only {
                        cat.selected = new_val;
                        cat.set_all_entries(new_val);
                    }
                }
            }

            ui.add_space(4.0);

            // Clean Selected button (red)
            let has_scanned = self.categories.iter().any(|c| c.scan_result.is_some());
            let has_any_selected = self
                .categories
                .iter()
                .any(|c| c.selected && !c.is_report_only && c.selected_count() > 0);
            let can_clean = !is_busy && has_scanned && has_any_selected;

            let clean_btn = egui::Button::new(
                egui::RichText::new("Clean Selected")
                    .size(15.0)
                    .strong()
                    .color(if can_clean {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(100, 100, 110)
                    }),
            )
            .fill(if can_clean {
                RED
            } else {
                egui::Color32::from_rgb(60, 40, 40)
            })
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(170.0, 36.0));

            if ui.add_enabled(can_clean, clean_btn).clicked() {
                self.show_confirm_dialog();
            }
        });

        // Progress bar
        if is_busy {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                let t = (ui.input(|i| i.time) % 2.0) as f32 / 2.0;
                let bar = egui::ProgressBar::new(t)
                    .animate(true)
                    .desired_width(ui.available_width() - 16.0);
                ui.add(bar);
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(&self.progress_label)
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                );
            });
        }

        // Last cleanup freed
        if self.cleaned_bytes > 0 && self.phase == AppPhase::Idle {
            ui.add_space(6.0);
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(25, 50, 30))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[OK]").size(16.0));
                        ui.label(
                            egui::RichText::new(format!(
                                "Last cleanup freed: {}",
                                utils::format_size(self.cleaned_bytes)
                            ))
                            .size(14.0)
                            .color(GREEN),
                        );
                    });
                });
        }

        ui.add_space(6.0);
    }

    fn render_category_list(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for i in 0..self.categories.len() {
                    Self::render_category_row(ui, &mut self.categories[i]);
                    ui.add_space(4.0);
                }
            });
    }

    fn render_category_row(ui: &mut egui::Ui, cat: &mut CategoryState) {
        let selected_size = cat.selected_bytes();
        let total_size = cat.scan_result.as_ref().map(|r| r.total_bytes).unwrap_or(0);

        let size_text = if cat.scan_result.is_none() {
            "---".to_string()
        } else if selected_size == total_size {
            utils::format_size(total_size)
        } else {
            format!(
                "{} / {}",
                utils::format_size(selected_size),
                utils::format_size(total_size)
            )
        };

        let card_fill = if cat.expanded { CARD_EXPANDED } else { CARD_FILL };

        egui::Frame::NONE
            .fill(card_fill)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .stroke(egui::Stroke::new(0.5, BORDER))
            .show(ui, |ui| {
                // ── Header row ──
                ui.horizontal(|ui| {
                    if cat.is_report_only {
                        let mut dummy = false;
                        ui.add_enabled(false, egui::Checkbox::new(&mut dummy, ""));
                    } else {
                        let before = cat.selected;
                        ui.checkbox(&mut cat.selected, "");
                        if cat.selected != before {
                            cat.set_all_entries(cat.selected);
                        }
                    }

                    paint_icon(ui, cat.icon, cat.icon_color);
                    ui.add_space(4.0);

                    let label_text = if cat.is_report_only {
                        format!("{} [report only]", cat.label)
                    } else {
                        cat.label.to_string()
                    };

                    let sel_info = if cat.entry_count() > 0 && !cat.is_report_only {
                        format!(" ({}/{})", cat.selected_count(), cat.entry_count())
                    } else {
                        String::new()
                    };

                    let arrow = if cat.expanded { "\u{25BC}" } else { "\u{25B6}" };

                    if ui
                        .selectable_label(
                            false,
                            egui::RichText::new(format!("{} {}{}", arrow, label_text, sel_info))
                                .size(14.0)
                                .strong()
                                .color(egui::Color32::from_rgb(210, 210, 225)),
                        )
                        .clicked()
                    {
                        cat.expanded = !cat.expanded;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let color = if cat.scan_result.is_some() { GREEN } else { BORDER };
                        ui.label(
                            egui::RichText::new(&size_text)
                                .size(14.0)
                                .strong()
                                .color(color),
                        );
                    });
                });

                // ── Expanded entries ──
                if !cat.expanded {
                    return;
                }

                ui.add_space(6.0);

                egui::Frame::NONE
                    .fill(INSET_FILL)
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        let has_result = cat.scan_result.is_some();
                        let entry_count = cat.entry_count();

                        if !has_result {
                            ui.label(
                                egui::RichText::new("Not yet scanned. Click \"Scan All\" to start.")
                                    .italics()
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                            );
                            return;
                        }

                        if entry_count == 0 {
                            ui.label(
                                egui::RichText::new("Nothing found.")
                                    .italics()
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                            );
                        } else {
                            if !cat.is_report_only {
                                ui.horizontal(|ui| {
                                    let s_all = egui::Button::new(
                                        egui::RichText::new("Select All")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(160, 160, 180)),
                                    )
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .min_size(egui::vec2(70.0, 22.0));
                                    if ui.add(s_all).clicked() {
                                        cat.set_all_entries(true);
                                        cat.selected = true;
                                    }

                                    let s_none = egui::Button::new(
                                        egui::RichText::new("Select None")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(160, 160, 180)),
                                    )
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .min_size(egui::vec2(80.0, 22.0));
                                    if ui.add(s_none).clicked() {
                                        cat.set_all_entries(false);
                                        cat.selected = false;
                                    }
                                });
                                ui.add_space(4.0);
                            }

                            for idx in 0..entry_count {
                                let (path_display, size_bytes) = {
                                    let entry = &cat.scan_result.as_ref().unwrap().entries[idx];
                                    (utils::display_path(&entry.path), entry.size_bytes)
                                };

                                ui.horizontal(|ui| {
                                    if !cat.is_report_only && idx < cat.entry_selected.len() {
                                        let before = cat.entry_selected[idx];
                                        ui.checkbox(&mut cat.entry_selected[idx], "");
                                        if cat.entry_selected[idx] != before {
                                            cat.sync_category_from_entries();
                                        }
                                    }

                                    ui.label(
                                        egui::RichText::new(&path_display)
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(150, 150, 165)),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(utils::format_size(size_bytes))
                                                    .size(12.0)
                                                    .color(YELLOW),
                                            );
                                        },
                                    );
                                });
                            }
                        }

                        // Errors
                        let errors: Vec<String> = cat
                            .scan_result
                            .as_ref()
                            .map(|r| r.errors.clone())
                            .unwrap_or_default();

                        for err in &errors {
                            if err.contains("Full Disk Access") {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("[!] Requires Full Disk Access.")
                                            .size(12.0)
                                            .color(YELLOW),
                                    );
                                    let btn = egui::Button::new(
                                        egui::RichText::new("Open System Settings").size(11.0),
                                    )
                                    .corner_radius(egui::CornerRadius::same(4));
                                    if ui.add(btn).clicked() {
                                        let _ = std::process::Command::new("open")
                                            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
                                            .spawn();
                                    }
                                });
                            } else {
                                ui.label(
                                    egui::RichText::new(format!("[!] {err}"))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(220, 100, 50)),
                                );
                            }
                        }
                    });
            });
    }

    fn render_summary(&self, ui: &mut egui::Ui) {
        let total: u64 = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only)
            .map(|c| c.selected_bytes())
            .sum();

        ui.add_space(8.0);

        egui::Frame::NONE
            .fill(CARD_FILL)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(20, 14))
            .stroke(egui::Stroke::new(1.0, BORDER))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Selected Reclaimable Space")
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(utils::format_size(total))
                            .size(32.0)
                            .strong()
                            .color(GREEN),
                    );
                });
            });

        ui.add_space(8.0);
    }

    fn render_confirm_dialog(&mut self, ctx: &egui::Context) {
        let mut should_clean = false;
        let mut should_cancel = false;

        // Dark overlay
        egui::Area::new(egui::Id::new("confirm_overlay"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                ui.allocate_rect(screen, egui::Sense::click());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(180));
            });

        egui::Window::new("")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([380.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("[!]").size(40.0));
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Confirm Deletion")
                            .size(20.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                });
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new(format!(
                        "Are you sure you want to permanently delete {} items?",
                        self.confirm_dialog.file_count
                    ))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(200, 200, 210)),
                );
                ui.add_space(8.0);

                egui::Frame::NONE
                    .fill(INSET_FILL)
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        for name in &self.confirm_dialog.category_names {
                            ui.label(
                                egui::RichText::new(format!("\u{2022} {name}"))
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(180, 180, 195)),
                            );
                        }
                    });

                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Total: {} will be freed",
                            utils::format_size(self.confirm_dialog.total_bytes)
                        ))
                        .size(16.0)
                        .strong()
                        .color(GREEN),
                    );
                });

                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("This action cannot be undone.")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(200, 100, 100)),
                    );
                });
                ui.add_space(14.0);

                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        let btn = egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(180, 180, 200)),
                        )
                        .corner_radius(egui::CornerRadius::same(8))
                        .min_size(egui::vec2(150.0, 36.0));
                        if ui.add(btn).clicked() {
                            should_cancel = true;
                        }
                    });
                    cols[1].vertical_centered(|ui| {
                        let btn = egui::Button::new(
                            egui::RichText::new("Delete Files")
                                .size(14.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(RED)
                        .corner_radius(egui::CornerRadius::same(8))
                        .min_size(egui::vec2(150.0, 36.0));
                        if ui.add(btn).clicked() {
                            should_clean = true;
                        }
                    });
                });
                ui.add_space(10.0);
            });

        if should_cancel {
            self.confirm_dialog.visible = false;
        }
        if should_clean {
            self.start_clean();
        }
    }

    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        let mut should_close = false;

        // Dark overlay
        egui::Area::new(egui::Id::new("about_overlay"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                if ui
                    .allocate_rect(screen, egui::Sense::click())
                    .clicked()
                {
                    should_close = true;
                }
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(200));
            });

        let dialog_fill = egui::Color32::from_rgb(25, 25, 35);
        let dialog_width = 380.0;

        egui::Window::new("")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([dialog_width, 0.0])
            .frame(
                egui::Frame::NONE
                    .fill(dialog_fill)
                    .corner_radius(egui::CornerRadius::same(14))
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .inner_margin(egui::Margin::same(24)),
            )
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_width(dialog_width - 48.0);

                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    // App icon badge
                    let size = 56.0;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 14.0, ACCENT);
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "T",
                        egui::FontId::proportional(28.0),
                        egui::Color32::WHITE,
                    );

                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("TidyMac")
                            .size(26.0)
                            .strong()
                            .color(TITLE_BLUE),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Version 0.2.0")
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                });

                ui.add_space(14.0);

                // Accent divider
                let (line_rect, _) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    line_rect,
                    0.0,
                    egui::Color32::from_rgb(45, 45, 60),
                );

                ui.add_space(14.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "A lightweight macOS cleanup tool that scans and\nremoves junk files to free up disk space.",
                        )
                        .size(13.0)
                        .color(egui::Color32::from_rgb(170, 170, 185)),
                    );
                });

                ui.add_space(16.0);

                // Developer info card — full width
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(32, 32, 45))
                    .corner_radius(egui::CornerRadius::same(10))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 70)))
                    .inner_margin(egui::Margin::symmetric(16, 14))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        ui.label(
                            egui::RichText::new("DEVELOPER")
                                .size(10.0)
                                .color(TEXT_SECONDARY),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("Intishar-Ul Islam")
                                .size(16.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(10.0);

                        // GitHub row
                        ui.horizontal(|ui| {
                            ui.set_min_width(ui.available_width());
                            let badge_size = 22.0;
                            let (badge_rect, _) = ui.allocate_exact_size(
                                egui::vec2(badge_size, badge_size),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                badge_rect,
                                5.0,
                                egui::Color32::from_rgb(45, 45, 60),
                            );
                            ui.painter().text(
                                badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "G",
                                egui::FontId::proportional(11.0),
                                ACCENT,
                            );

                            if ui
                                .link(
                                    egui::RichText::new("github.com/Nahianether")
                                        .size(13.0)
                                        .color(ACCENT),
                                )
                                .clicked()
                            {
                                let _ = std::process::Command::new("open")
                                    .arg("https://github.com/Nahianether")
                                    .spawn();
                            }
                        });

                        ui.add_space(6.0);

                        // Portfolio row
                        ui.horizontal(|ui| {
                            ui.set_min_width(ui.available_width());
                            let badge_size = 22.0;
                            let (badge_rect, _) = ui.allocate_exact_size(
                                egui::vec2(badge_size, badge_size),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                badge_rect,
                                5.0,
                                egui::Color32::from_rgb(45, 45, 60),
                            );
                            ui.painter().text(
                                badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "W",
                                egui::FontId::proportional(11.0),
                                GREEN,
                            );

                            if ui
                                .link(
                                    egui::RichText::new("intishar.xyz")
                                        .size(13.0)
                                        .color(GREEN),
                                )
                                .clicked()
                            {
                                let _ = std::process::Command::new("open")
                                    .arg("https://intishar.xyz/")
                                    .spawn();
                            }
                        });
                    });

                ui.add_space(18.0);

                // Close button — full width, styled
                ui.vertical_centered(|ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("Close")
                            .size(14.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(50, 50, 68))
                    .corner_radius(egui::CornerRadius::same(8))
                    .min_size(egui::vec2(ui.available_width(), 36.0));
                    if ui.add(btn).clicked() {
                        should_close = true;
                    }
                });

                ui.add_space(4.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Built with Rust + egui")
                            .size(10.0)
                            .color(egui::Color32::from_rgb(80, 80, 100)),
                    );
                });
            });

        if should_close {
            self.about_visible = false;
        }
    }

    fn render_errors(&self, ui: &mut egui::Ui) {
        if self.errors.is_empty() {
            return;
        }

        ui.add_space(4.0);
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(45, 35, 25))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                egui::CollapsingHeader::new(
                    egui::RichText::new(format!("[!] Warnings ({})", self.errors.len()))
                        .size(13.0)
                        .color(YELLOW),
                )
                .default_open(false)
                .show(ui, |ui| {
                    for err in &self.errors {
                        ui.label(
                            egui::RichText::new(err)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(220, 100, 50)),
                        );
                    }
                });
            });
    }
}

// ── eframe::App ────────────────────────────────────────────────────────

impl eframe::App for TidyMacApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        if self.phase != AppPhase::Idle {
            ctx.request_repaint();
        }

        if self.confirm_dialog.visible {
            self.render_confirm_dialog(ctx);
        }

        if self.about_visible {
            self.render_about_dialog(ctx);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ctx, |ui| {
                self.render_header(ui);
                self.render_disk_bar(ui);
                self.render_action_bar(ui);
                self.render_category_list(ui);
                self.render_summary(ui);
                self.render_errors(ui);
            });
    }
}
