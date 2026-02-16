use sysinfo::{Networks, System};
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::disk_info;
use crate::utils;

pub struct Monitor {
    _tray: TrayIcon,
    disk_item: MenuItem,
    mem_item: MenuItem,
    cpu_item: MenuItem,
    net_item: MenuItem,
    sys: System,
    networks: Networks,
    last_update: std::time::Instant,
    /// Track elapsed time between refreshes for accurate speed calculation.
    last_net_time: std::time::Instant,
}

fn create_storage_icon() -> Icon {
    let size: usize = 22;
    let mut rgba = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;

            // Draw a simple disk/storage shape:
            // Rounded rectangle body (4..18 x 5..17)
            let in_body = x >= 4 && x <= 17 && y >= 5 && y <= 16;
            // Top rounded edge
            let in_top = x >= 5 && x <= 16 && y == 4;
            let in_bottom = x >= 5 && x <= 16 && y == 17;
            // Activity LED dot (bottom-right)
            let led_cx = 14.5f32;
            let led_cy = 14.0f32;
            let led_dist = ((x as f32 - led_cx).powi(2) + (y as f32 - led_cy).powi(2)).sqrt();
            let in_led = led_dist < 1.8;
            // Slot line (top area, like a disk slot)
            let in_slot = x >= 7 && x <= 14 && y >= 7 && y <= 8;

            if in_led {
                // Green LED
                rgba[idx] = 80;
                rgba[idx + 1] = 220;
                rgba[idx + 2] = 120;
                rgba[idx + 3] = 255;
            } else if in_slot {
                // Darker slot line
                rgba[idx] = 120;
                rgba[idx + 1] = 125;
                rgba[idx + 2] = 140;
                rgba[idx + 3] = 255;
            } else if in_body || in_top || in_bottom {
                // Body - light gray for visibility on dark & light menu bars
                rgba[idx] = 180;
                rgba[idx + 1] = 185;
                rgba[idx + 2] = 195;
                rgba[idx + 3] = 255;
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).expect("failed to create tray icon")
}

/// Format bytes-per-second into a human-readable rate string.
fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{:.0} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else if bytes_per_sec < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB/s", bytes_per_sec / (1024.0 * 1024.0 * 1024.0))
    }
}

impl Monitor {
    pub fn new() -> Option<Self> {
        let menu = Menu::new();

        let app_label = MenuItem::new("TidyMac Monitor", false, None);
        let separator = PredefinedMenuItem::separator();
        let disk_item = MenuItem::new("Disk: calculating...", false, None);
        let mem_item = MenuItem::new("Memory: calculating...", false, None);
        let cpu_item = MenuItem::new("CPU: calculating...", false, None);
        let net_item = MenuItem::new("Net: calculating...", false, None);

        let _ = menu.append(&app_label);
        let _ = menu.append(&separator);
        let _ = menu.append(&cpu_item);
        let _ = menu.append(&mem_item);
        let _ = menu.append(&disk_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&net_item);

        let icon = create_storage_icon();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("TidyMac")
            .with_icon(icon)
            .with_title("--")
            .build()
            .ok()?;

        // Initialize system with CPU baseline
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_usage(); // baseline — first reading will be inaccurate

        // Initialize networks with baseline
        let networks = Networks::new_with_refreshed_list();

        let now = std::time::Instant::now();
        let mut monitor = Self {
            _tray: tray,
            disk_item,
            mem_item,
            cpu_item,
            net_item,
            sys,
            networks,
            last_update: now - std::time::Duration::from_secs(60),
            last_net_time: now,
        };
        // Wait briefly then refresh for initial accurate CPU reading
        std::thread::sleep(std::time::Duration::from_millis(200));
        monitor.refresh();
        Some(monitor)
    }

    pub fn refresh(&mut self) {
        let now = std::time::Instant::now();

        // ── CPU ──
        self.sys.refresh_cpu_usage();
        let cpu_usage = self.sys.global_cpu_usage();
        self.cpu_item.set_text(format!("CPU: {:.1}%", cpu_usage));

        // ── Memory ──
        self.sys.refresh_memory();
        let used_mem = self.sys.used_memory();
        let total_mem = self.sys.total_memory();
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

        // ── Disk ──
        if let Some(info) = disk_info::get_disk_info() {
            let pct = (info.usage_percent() * 100.0) as u32;
            self.disk_item.set_text(format!(
                "Disk: {} used / {} total ({}%)",
                utils::format_size(info.used),
                utils::format_size(info.total),
                pct,
            ));
            let free = utils::format_size(info.available);
            self._tray.set_title(Some(free));
        }

        // ── Network ──
        let elapsed = now.duration_since(self.last_net_time).as_secs_f64();
        self.networks.refresh(true);

        let rx_bytes: u64 = self.networks.list().values().map(|d| d.received()).sum();
        let tx_bytes: u64 = self.networks.list().values().map(|d| d.transmitted()).sum();

        if elapsed > 0.1 {
            let rx_rate = rx_bytes as f64 / elapsed;
            let tx_rate = tx_bytes as f64 / elapsed;
            self.net_item.set_text(format!(
                "Net: \u{2191}{} \u{2193}{}",
                format_rate(tx_rate),
                format_rate(rx_rate),
            ));
        }

        self.last_net_time = now;
        self.last_update = now;
    }

    /// Call this from the eframe update loop. Refreshes every 3 seconds
    /// for responsive CPU and network readings.
    pub fn tick(&mut self) {
        if self.last_update.elapsed() >= std::time::Duration::from_secs(3) {
            self.refresh();
        }
    }
}
