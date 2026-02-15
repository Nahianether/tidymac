use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::disk_info;
use crate::utils;

pub struct Monitor {
    _tray: TrayIcon,
    disk_item: MenuItem,
    mem_item: MenuItem,
    last_update: std::time::Instant,
}

fn create_icon() -> Icon {
    let size: usize = 22;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let cx = x as f32 - 10.5;
            let cy = y as f32 - 10.5;
            let dist = (cx * cx + cy * cy).sqrt();
            if dist < 9.0 {
                rgba[idx] = 60;      // R
                rgba[idx + 1] = 140; // G
                rgba[idx + 2] = 220; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).expect("failed to create tray icon")
}

fn get_memory_info() -> (u64, u64) {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    (sys.used_memory(), sys.total_memory())
}

impl Monitor {
    pub fn new() -> Option<Self> {
        let menu = Menu::new();

        let disk_item = MenuItem::new("Disk: calculating...", false, None);
        let mem_item = MenuItem::new("Memory: calculating...", false, None);
        let separator = PredefinedMenuItem::separator();
        let app_label = MenuItem::new("TidyMac Monitor", false, None);

        let _ = menu.append(&app_label);
        let _ = menu.append(&separator);
        let _ = menu.append(&disk_item);
        let _ = menu.append(&mem_item);

        let icon = create_icon();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("TidyMac")
            .with_icon(icon)
            .with_title("-- free")
            .build()
            .ok()?;

        let mut monitor = Self {
            _tray: tray,
            disk_item,
            mem_item,
            last_update: std::time::Instant::now()
                - std::time::Duration::from_secs(60),
        };
        monitor.refresh();
        Some(monitor)
    }

    pub fn refresh(&mut self) {
        // Disk info
        if let Some(info) = disk_info::get_disk_info() {
            let pct = (info.usage_percent() * 100.0) as u32;
            self.disk_item.set_text(format!(
                "Disk: {} used / {} total ({}%)",
                utils::format_size(info.used),
                utils::format_size(info.total),
                pct,
            ));
            self._tray
                .set_title(Some(format!("{} free", utils::format_size(info.available))));
        }

        // Memory info
        let (used_mem, total_mem) = get_memory_info();
        let mem_pct = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64 * 100.0) as u32
        } else {
            0
        };
        self.mem_item.set_text(format!(
            "Memory: {} / {} ({}%)",
            utils::format_size(used_mem),
            utils::format_size(total_mem),
            mem_pct,
        ));

        self.last_update = std::time::Instant::now();
    }

    /// Call this from the eframe update loop. Refreshes every 30 seconds.
    pub fn tick(&mut self) {
        if self.last_update.elapsed() >= std::time::Duration::from_secs(30) {
            self.refresh();
        }
    }
}
