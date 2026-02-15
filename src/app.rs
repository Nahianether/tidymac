use std::path::PathBuf;
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
    pub entry_selected: Vec<bool>, // parallel to scan_result.entries
    pub is_report_only: bool,
}

impl CategoryState {
    /// Total bytes of only the selected entries.
    fn selected_bytes(&self) -> u64 {
        match &self.scan_result {
            Some(result) => result
                .entries
                .iter()
                .zip(self.entry_selected.iter())
                .filter(|(_, sel)| **sel)
                .map(|(e, _)| e.size_bytes)
                .sum(),
            None => 0,
        }
    }

    /// Number of selected entries.
    fn selected_count(&self) -> usize {
        self.entry_selected.iter().filter(|s| **s).count()
    }

    /// Total entry count.
    fn entry_count(&self) -> usize {
        self.scan_result
            .as_ref()
            .map(|r| r.entries.len())
            .unwrap_or(0)
    }

    /// Set all entries to selected or deselected.
    fn set_all_entries(&mut self, val: bool) {
        for s in &mut self.entry_selected {
            *s = val;
        }
    }

    /// Sync the category checkbox from entry selections:
    /// selected = true if at least one entry is selected.
    fn sync_category_from_entries(&mut self) {
        if !self.is_report_only {
            self.selected = self.entry_selected.iter().any(|s| *s);
        }
    }
}

/// Represents a file to delete — sent to the background thread.
struct DeleteItem {
    category_name: String,
    path: PathBuf,
    size_bytes: u64,
}

/// Messages sent from background threads to the UI thread.
pub enum BgMessage {
    ScanComplete(String, ScanResult),
    AllScansComplete,
    DeletedFile(String, PathBuf, u64),
    DeleteError(String, PathBuf, String),
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
                entry_selected: vec![],
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
                total_bytes: 0,
                file_count: 0,
                category_names: vec![],
            },
            errors: vec![],
            cleaned_bytes: 0,
        }
    }

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
        self.cleaned_bytes = 0;

        // Collect specific files to delete from selected entries
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
                let _ = tx.send(BgMessage::Progress(
                    format!("Deleting: {}", item.path.display()),
                ));
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
                        if let Some(cat) =
                            self.categories.iter_mut().find(|c| c.name == name)
                        {
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
                        // Remove the deleted entry from the category
                        if let Some(cat) =
                            self.categories.iter_mut().find(|c| c.name == cat_name)
                        {
                            if let Some(ref mut result) = cat.scan_result {
                                if let Some(idx) =
                                    result.entries.iter().position(|e| e.path == path)
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
                        self.errors.push(format!(
                            "Failed to delete {}: {err}",
                            path.display()
                        ));
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
                    "{} ({} items, {})",
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
                        cat.set_all_entries(new_val);
                    }
                }
            }

            let has_scanned = self.categories.iter().any(|c| c.scan_result.is_some());
            let has_any_selected = self
                .categories
                .iter()
                .any(|c| c.selected && !c.is_report_only && c.selected_count() > 0);
            let can_clean = !is_busy && has_scanned && has_any_selected;

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

        // Show cleaned bytes after a clean operation
        if self.cleaned_bytes > 0 && self.phase == AppPhase::Idle {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Last cleanup freed: {}",
                        utils::format_size(self.cleaned_bytes)
                    ))
                    .color(egui::Color32::from_rgb(80, 200, 80)),
                );
            });
        }

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
        let selected_size = cat.selected_bytes();
        let total_size = cat
            .scan_result
            .as_ref()
            .map(|r| r.total_bytes)
            .unwrap_or(0);

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

        // Header row with checkbox, label, and size
        ui.horizontal(|ui| {
            // Category checkbox
            if cat.is_report_only {
                let mut dummy = false;
                ui.add_enabled(false, egui::Checkbox::new(&mut dummy, ""));
            } else {
                let before = cat.selected;
                ui.checkbox(&mut cat.selected, "");
                // If category checkbox was toggled, update all entry selections
                if cat.selected != before {
                    cat.set_all_entries(cat.selected);
                }
            }

            // Clickable label to toggle expand
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
                    egui::RichText::new(format!(
                        "{} {}{}",
                        arrow, label_text, sel_info
                    ))
                    .strong(),
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

        // Expanded entries with per-entry checkboxes
        if !cat.expanded {
            return;
        }

        let entry_count = cat.entry_count();
        let has_result = cat.scan_result.is_some();

        ui.indent(cat.name, |ui| {
            if !has_result {
                ui.label(
                    egui::RichText::new(
                        "Not yet scanned. Click \"Scan All\" to start.",
                    )
                    .italics()
                    .color(egui::Color32::GRAY),
                );
                return;
            }

            if entry_count == 0 {
                ui.label(
                    egui::RichText::new("Nothing found.")
                        .italics()
                        .color(egui::Color32::GRAY),
                );
            } else {
                // Select All / None buttons for this category
                if !cat.is_report_only {
                    ui.horizontal(|ui| {
                        if ui.small_button("Select All").clicked() {
                            cat.set_all_entries(true);
                            cat.selected = true;
                        }
                        if ui.small_button("Select None").clicked() {
                            cat.set_all_entries(false);
                            cat.selected = false;
                        }
                    });
                    ui.add_space(2.0);
                }

                // Render each entry — extract display data first to avoid borrow conflicts
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
                                .color(egui::Color32::from_rgb(160, 160, 170)),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(utils::format_size(size_bytes))
                                        .color(egui::Color32::from_rgb(220, 180, 50)),
                                );
                            },
                        );
                    });
                }
            }

            // Render errors — extract to avoid borrow conflict
            let errors: Vec<String> = cat
                .scan_result
                .as_ref()
                .map(|r| r.errors.clone())
                .unwrap_or_default();

            for err in &errors {
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
        });
    }

    fn render_summary(&self, ui: &mut egui::Ui) {
        let total: u64 = self
            .categories
            .iter()
            .filter(|c| c.selected && !c.is_report_only)
            .map(|c| c.selected_bytes())
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

        // Dark overlay behind the dialog to block background interaction
        egui::Area::new(egui::Id::new("confirm_overlay"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                ui.allocate_rect(screen, egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    egui::Color32::from_black_alpha(160),
                );
            });

        egui::Window::new("")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([360.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("\u{26A0}")
                            .size(36.0)
                            .color(egui::Color32::from_rgb(220, 180, 50)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Confirm Deletion")
                            .size(18.0)
                            .strong(),
                    );
                });
                ui.add_space(8.0);

                ui.label(format!(
                    "Are you sure you want to permanently delete {} items?",
                    self.confirm_dialog.file_count
                ));
                ui.add_space(8.0);

                egui::Frame::group(ui.style())
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        for name in &self.confirm_dialog.category_names {
                            ui.label(format!("\u{2022} {name}"));
                        }
                    });

                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Total: {} will be freed",
                            utils::format_size(self.confirm_dialog.total_bytes)
                        ))
                        .strong()
                        .size(15.0)
                        .color(egui::Color32::from_rgb(80, 200, 80)),
                    );
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("This action cannot be undone.")
                        .small()
                        .color(egui::Color32::from_rgb(200, 100, 100)),
                );
                ui.add_space(12.0);

                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        if ui
                            .add_sized(
                                [140.0, 32.0],
                                egui::Button::new("Cancel"),
                            )
                            .clicked()
                        {
                            should_cancel = true;
                        }
                    });
                    cols[1].vertical_centered(|ui| {
                        if ui
                            .add_sized(
                                [140.0, 32.0],
                                egui::Button::new(
                                    egui::RichText::new("Delete Files")
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(200, 50, 50)),
                            )
                            .clicked()
                        {
                            should_clean = true;
                        }
                    });
                });
                ui.add_space(8.0);
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
