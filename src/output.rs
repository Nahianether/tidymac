use colored::Colorize;

pub fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.2} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn print_banner() {
    println!(
        "{}",
        "tidymac - macOS Cleanup Tool v0.1.0"
            .bold()
            .cyan()
    );
    println!();
}

pub fn print_scan_header(label: &str) {
    println!("{}", format!("=== {label} ===").bold().white());
}

pub fn print_scan_entry(path: &str, size: &str) {
    println!("  {}  {}", path.dimmed(), size.yellow());
}

pub fn print_category_total(label: &str, total: &str) {
    println!(
        "  {} {}",
        format!("{label} total:").bold(),
        total.green()
    );
    println!();
}

pub fn print_summary_header() {
    println!("{}", "=== Summary ===".bold().white());
}

pub fn print_summary_row(label: &str, size: &str) {
    println!("  {:<30} {}", label, size.green());
}

pub fn print_summary_row_report_only(label: &str, size: &str) {
    println!(
        "  {:<30} {}  {}",
        label,
        size.green(),
        "[report only]".dimmed()
    );
}

pub fn print_separator() {
    println!("  {}", "─".repeat(45).dimmed());
}

pub fn print_grand_total(total: &str) {
    println!(
        "  {:<30} {}",
        "Total reclaimable:".bold(),
        total.green().bold()
    );
    println!();
}

pub fn print_warning(msg: &str) {
    println!("{} {}", "Warning:".red().bold(), msg.red());
}

pub fn print_info(msg: &str) {
    println!("{} {}", "Info:".cyan().bold(), msg);
}

pub fn print_dry_run_footer() {
    println!(
        "{}",
        "This was a dry run. Run `tidymac clean --confirm` to delete."
            .yellow()
            .bold()
    );
}

pub fn print_clean_complete(freed: &str) {
    println!(
        "{} {}",
        "Cleaned!".green().bold(),
        format!("{freed} freed.").green()
    );
}

pub fn print_deleted(path: &str, size: &str) {
    println!(
        "  {} {}  {}",
        "Deleted".red(),
        path.dimmed(),
        size.yellow()
    );
}

pub fn print_delete_error(path: &str, err: &str) {
    println!(
        "  {} {} — {}",
        "Failed".red().bold(),
        path.dimmed(),
        err.red()
    );
}

pub fn print_no_confirm_warning() {
    println!(
        "{}",
        "No --confirm flag provided. Running as dry-run scan."
            .yellow()
            .bold()
    );
    println!();
}
