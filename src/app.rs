use std::sync::mpsc;

use eframe::egui;

use crate::cleaner::ScanResult;
use crate::utils;

/// Per-category state held by the GUI.
pub struct CategoryState {
    pub name: &'static str,
    pub label: &'static str,
    pub selected: bool,
    pub expanded: bool,
    pub scan_result: Option<ScanResult>,
    pub is_report_only: bool,
}

/// Messages sent from background threads to the UI thread.
pub enum BgMessage {
    ScanComplete(String, ScanResult),
    AllScansComplete,
    CleanComplete(String, ScanResult),
    AllCleansComplete,
    Progress(String),
}

/// Overall application operation state.
#[derive(PartialEq)]
pub enum AppPhase {
    Idle,
    Scanning,
    Cleaning,
}

/// Confirmation dialog state.
pub struct ConfirmDialog {
    pub visible: bool,
    pub categories_to_clean: Vec<String>,
    pub total_bytes: u64,
}

pub struct TidyMacApp {
    categories: Vec<CategoryState>,
    phase: AppPhase,
    receiver: Option<mpsc::Receiver<BgMessage>>,
    progress_label: String,
    confirm_dialog: ConfirmDialog,
    errors: Vec<String>,
}

impl TidyMacApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let cleaners = crate::categories::all_cleaners(104_857_600, None);
        let categories: Vec<CategoryState> = cleaners
            .iter()
            .map(|c| CategoryState {
                name: c.name(),
                label: c.label(),
                selected: c.name() != "large-files",
                expanded: false,
                scan_result: None,
                is_report_only: c.name() == "large-files",
            })
            .collect();

        Self {
            categories,
            phase: AppPhase::Idle,
            receiver: None,
            progress_label: String::new(),
            confirm_dialog: ConfirmDialog {
                visible: false,
                categories_to_clean: vec![],
                total_bytes: 0,
            },
            errors: vec![],
        }
    }

    fn start_scan(&mut self) {
        self.phase = AppPhase::Scanning;
        self.progress_label = "Starting scan...".to_string();
        self.errors.clear();

        for cat in &mut self.categories {
            cat.scan_result = None;
        }

        let (tx, rx) = mpsc::channel::<BgMessage>();
        self.receiver = Some(rx);

        std::thread::spawn(move || {
            let cleaners = crate::categories::all_cleaners(104_857_600, None);
            for cleaner in &cleaners {
                let _ = tx.send(BgMessage::Progress(cleaner.label().to_string()));
                let result = cleaner.scan();
                let _ = tx.send(BgMessage::ScanComplete(
                    cleaner.name().to_string(),
                    result,
                ));
            }
            let _ = tx.send(BgMessage::AllScansComplete);
        });
    }

    fn start_clean(&mut self) {
        self.phase = AppPhase::Cleaning;
        self.progress_label = "Starting cleanup...".to_string();
        self.confirm_dialog.visible = false;

        let selected_names: Vec<String> = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only)
            .map(|c| c.name.to_string())
            .collect();

        let (tx, rx) = mpsc::channel::<BgMessage>();
        self.receiver = Some(rx);

        std::thread::spawn(move || {
            for name in &selected_names {
                if let Some(cleaner) =
                    crate::categories::find_cleaner(name, 104_857_600, None)
                {
                    let _ = tx.send(BgMessage::Progress(cleaner.label().to_string()));
                    let result = cleaner.clean(false);
                    let _ = tx.send(BgMessage::CleanComplete(
                        cleaner.name().to_string(),
                        result,
                    ));
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
                        self.progress_label = format!("Processing: {}...", label);
                    }
                    BgMessage::ScanComplete(name, result) => {
                        if let Some(cat) =
                            self.categories.iter_mut().find(|c| c.name == name)
                        {
                            cat.scan_result = Some(result);
                        }
                    }
                    BgMessage::AllScansComplete => {
                        self.phase = AppPhase::Idle;
                        self.progress_label.clear();
                    }
                    BgMessage::CleanComplete(name, result) => {
                        for err in &result.errors {
                            self.errors.push(err.clone());
                        }
                        if let Some(cat) =
                            self.categories.iter_mut().find(|c| c.name == name)
                        {
                            cat.scan_result = Some(result);
                        }
                    }
                    BgMessage::AllCleansComplete => {
                        self.phase = AppPhase::Idle;
                        self.progress_label.clear();
                    }
                }
            }
        }
    }

    fn show_confirm_dialog(&mut self) {
        let selected: Vec<String> = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only && c.scan_result.is_some())
            .map(|c| c.name.to_string())
            .collect();

        let total: u64 = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only)
            .filter_map(|c| c.scan_result.as_ref())
            .map(|r| r.total_bytes)
            .sum();

        self.confirm_dialog = ConfirmDialog {
            visible: true,
            categories_to_clean: selected,
            total_bytes: total,
        };
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.heading(
                egui::RichText::new("TidyMac")
                    .size(28.0)
                    .strong()
                    .color(egui::Color32::from_rgb(80, 180, 220)),
            );
            ui.label(
                egui::RichText::new("macOS Cleanup Tool")
                    .size(14.0)
                    .color(egui::Color32::GRAY),
            );
        });
        ui.add_space(8.0);
    }

    fn render_action_bar(&mut self, ui: &mut egui::Ui) {
        let is_busy = self.phase != AppPhase::Idle;

        ui.horizontal(|ui| {
            ui.add_space(4.0);

            if ui
                .add_enabled(!is_busy, egui::Button::new("Scan All"))
                .clicked()
            {
                self.start_scan();
            }

            let all_selected = self
                .categories
                .iter()
                .filter(|c| !c.is_report_only)
                .all(|c| c.selected);
            let toggle_label = if all_selected {
                "Deselect All"
            } else {
                "Select All"
            };
            if ui.button(toggle_label).clicked() {
                let new_val = !all_selected;
                for cat in &mut self.categories {
                    if !cat.is_report_only {
                        cat.selected = new_val;
                    }
                }
            }

            let has_scanned = self.categories.iter().any(|c| c.scan_result.is_some());
            let has_selected = self
                .categories
                .iter()
                .any(|c| c.selected && !c.is_report_only);
            let can_clean = !is_busy && has_scanned && has_selected;

            if ui
                .add_enabled(
                    can_clean,
                    egui::Button::new(
                        egui::RichText::new("Clean Selected")
                            .color(if can_clean {
                                egui::Color32::from_rgb(220, 60, 60)
                            } else {
                                egui::Color32::GRAY
                            }),
                    ),
                )
                .clicked()
            {
                self.show_confirm_dialog();
            }

            if is_busy {
                ui.add_space(8.0);
                ui.spinner();
                ui.label(&self.progress_label);
            }
        });
        ui.add_space(4.0);
    }

    fn render_category_list(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let len = self.categories.len();
                for i in 0..len {
                    Self::render_category_row(ui, &mut self.categories[i]);
                    if i < len - 1 {
                        ui.separator();
                    }
                }
            });
    }

    fn render_category_row(ui: &mut egui::Ui, cat: &mut CategoryState) {
        let size_text = match &cat.scan_result {
            Some(result) => utils::format_size(result.total_bytes),
            None => "---".to_string(),
        };

        let id = ui.make_persistent_id(cat.name);

        // Header row with checkbox, label, and size
        ui.horizontal(|ui| {
            // Checkbox
            if cat.is_report_only {
                let mut dummy = false;
                ui.add_enabled(false, egui::Checkbox::new(&mut dummy, ""));
            } else {
                ui.checkbox(&mut cat.selected, "");
            }

            // Clickable label to toggle expand
            let label_text = if cat.is_report_only {
                format!("{} [report only]", cat.label)
            } else {
                cat.label.to_string()
            };

            let arrow = if cat.expanded { "\u{25BC}" } else { "\u{25B6}" };

            if ui
                .selectable_label(
                    false,
                    egui::RichText::new(format!("{} {}", arrow, label_text)).strong(),
                )
                .clicked()
            {
                cat.expanded = !cat.expanded;
            }

            // Right-aligned size
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let color = if cat.scan_result.is_some() {
                    egui::Color32::from_rgb(80, 200, 80)
                } else {
                    egui::Color32::GRAY
                };
                ui.label(egui::RichText::new(&size_text).strong().color(color));
            });
        });

        // Expanded entries
        if cat.expanded {
            let mut state =
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    true,
                );
            state.set_open(true);
            state.show_body_unindented(ui, |ui| {
                ui.indent(id, |ui| {
                    if let Some(ref result) = cat.scan_result {
                        if result.entries.is_empty() {
                            ui.label(
                                egui::RichText::new("Nothing found.")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                        } else {
                            for entry in &result.entries {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(utils::display_path(
                                            &entry.path,
                                        ))
                                        .color(egui::Color32::from_rgb(
                                            160, 160, 170,
                                        )),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(
                                            egui::Align::Center,
                                        ),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    utils::format_size(
                                                        entry.size_bytes,
                                                    ),
                                                )
                                                .color(egui::Color32::from_rgb(
                                                    220, 180, 50,
                                                )),
                                            );
                                        },
                                    );
                                });
                            }
                        }
                        for err in &result.errors {
                            if err.contains("Full Disk Access") {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Requires Full Disk Access.")
                                            .color(egui::Color32::from_rgb(220, 150, 50)),
                                    );
                                    if ui.button("Open System Settings").clicked() {
                                        let _ = std::process::Command::new("open")
                                            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
                                            .spawn();
                                    }
                                });
                            } else {
                                ui.label(
                                    egui::RichText::new(format!("Warning: {err}"))
                                        .color(egui::Color32::from_rgb(220, 100, 50)),
                                );
                            }
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Not yet scanned. Click \"Scan All\" to start.")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
                    }
                });
            });
        }
    }

    fn render_summary(&self, ui: &mut egui::Ui) {
        let total: u64 = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only)
            .filter_map(|c| c.scan_result.as_ref())
            .map(|r| r.total_bytes)
            .sum();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Selected reclaimable space:").strong());
            ui.label(
                egui::RichText::new(utils::format_size(total))
                    .strong()
                    .size(16.0)
                    .color(egui::Color32::from_rgb(80, 200, 80)),
            );
        });
        ui.add_space(4.0);
    }

    fn render_confirm_dialog(&mut self, ctx: &egui::Context) {
        let mut should_clean = false;
        let mut should_cancel = false;

        egui::Window::new("Confirm Cleanup")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete files in these categories?");
                ui.add_space(8.0);

                for name in &self.confirm_dialog.categories_to_clean {
                    ui.label(format!("  \u{2022} {name}"));
                }

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Total: {} will be freed",
                        utils::format_size(self.confirm_dialog.total_bytes)
                    ))
                    .strong(),
                );
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_cancel = true;
                    }
                    ui.add_space(20.0);
                    if ui
                        .button(
                            egui::RichText::new("Delete Files")
                                .color(egui::Color32::from_rgb(220, 60, 60)),
                        )
                        .clicked()
                    {
                        should_clean = true;
                    }
                });
            });

        if should_cancel {
            self.confirm_dialog.visible = false;
        }
        if should_clean {
            self.start_clean();
        }
    }

    fn render_errors(&self, ui: &mut egui::Ui) {
        if !self.errors.is_empty() {
            ui.add_space(4.0);
            egui::CollapsingHeader::new(
                egui::RichText::new(format!("Warnings ({})", self.errors.len()))
                    .color(egui::Color32::from_rgb(220, 150, 50)),
            )
            .default_open(false)
            .show(ui, |ui| {
                for err in &self.errors {
                    ui.label(
                        egui::RichText::new(err)
                            .color(egui::Color32::from_rgb(220, 100, 50)),
                    );
                }
            });
        }
    }
}

impl eframe::App for TidyMacApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        if self.phase != AppPhase::Idle {
            ctx.request_repaint();
        }

        if self.confirm_dialog.visible {
            self.render_confirm_dialog(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);
            self.render_action_bar(ui);
            ui.separator();
            self.render_category_list(ui);
            ui.separator();
            self.render_summary(ui);
            self.render_errors(ui);
        });
    }
}
