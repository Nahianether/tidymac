mod categories;
mod cleaner;
mod cli;
mod output;
mod utils;

use clap::Parser;
use cleaner::Cleaner;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    output::print_banner();

    match cli.command {
        Command::Scan {
            category,
            min_size,
            path,
        } => {
            let min_bytes = parse_min_size(&min_size);
            let cleaners = resolve_cleaners(category.as_deref(), min_bytes, path.as_deref());
            if cleaners.is_empty() {
                return;
            }
            run_scan(&cleaners);
        }
        Command::Clean {
            confirm,
            category,
            min_size,
            path,
        } => {
            let min_bytes = parse_min_size(&min_size);
            let cleaners = resolve_cleaners(category.as_deref(), min_bytes, path.as_deref());
            if cleaners.is_empty() {
                return;
            }
            if !confirm {
                output::print_no_confirm_warning();
                run_scan(&cleaners);
            } else {
                run_clean(&cleaners);
            }
        }
    }
}

fn parse_min_size(s: &str) -> u64 {
    utils::parse_size(s).unwrap_or_else(|e| {
        output::print_warning(&format!("Invalid --min-size: {e}. Using 100MB."));
        104_857_600
    })
}

fn resolve_cleaners(
    category: Option<&str>,
    min_bytes: u64,
    path: Option<&str>,
) -> Vec<Box<dyn Cleaner>> {
    match category {
        Some(name) => match categories::find_cleaner(name, min_bytes, path) {
            Some(c) => vec![c],
            None => {
                output::print_warning(&format!("Unknown category: {name}"));
                output::print_info(&format!(
                    "Available: {}",
                    categories::all_cleaner_names().join(", ")
                ));
                vec![]
            }
        },
        None => categories::all_cleaners(min_bytes, path),
    }
}

fn run_scan(cleaners: &[Box<dyn Cleaner>]) {
    let mut grand_total = 0u64;
    let mut summaries: Vec<(&str, u64, bool)> = Vec::new();

    for cleaner in cleaners {
        let result = cleaner.scan();

        output::print_scan_header(cleaner.label());

        if result.entries.is_empty() {
            output::print_info("Nothing found.");
            println!();
        } else {
            for entry in &result.entries {
                output::print_scan_entry(
                    &utils::display_path(&entry.path),
                    &output::format_size(entry.size_bytes),
                );
            }
            output::print_category_total(cleaner.label(), &output::format_size(result.total_bytes));
        }

        for err in &result.errors {
            output::print_warning(err);
        }

        let is_report_only = cleaner.name() == "large-files";
        if !is_report_only {
            grand_total += result.total_bytes;
        }
        summaries.push((cleaner.name(), result.total_bytes, is_report_only));
    }

    // Print summary
    output::print_summary_header();
    for (name, bytes, report_only) in &summaries {
        if *report_only {
            output::print_summary_row_report_only(name, &output::format_size(*bytes));
        } else {
            output::print_summary_row(name, &output::format_size(*bytes));
        }
    }
    output::print_separator();
    output::print_grand_total(&output::format_size(grand_total));
    output::print_dry_run_footer();
}

fn run_clean(cleaners: &[Box<dyn Cleaner>]) {
    let mut grand_total = 0u64;

    for cleaner in cleaners {
        let is_report_only = cleaner.name() == "large-files";

        if is_report_only {
            let result = cleaner.scan();
            output::print_scan_header(cleaner.label());
            for entry in &result.entries {
                output::print_scan_entry(
                    &utils::display_path(&entry.path),
                    &output::format_size(entry.size_bytes),
                );
            }
            if !result.entries.is_empty() {
                output::print_info("Large files listed for review only. Remove manually if needed.");
            }
            println!();
            continue;
        }

        let result = cleaner.clean(false);

        output::print_scan_header(cleaner.label());

        if result.entries.is_empty() {
            output::print_info("Nothing to clean.");
            println!();
        } else {
            for entry in &result.entries {
                output::print_deleted(
                    &utils::display_path(&entry.path),
                    &output::format_size(entry.size_bytes),
                );
            }
            output::print_category_total(cleaner.label(), &output::format_size(result.total_bytes));
            grand_total += result.total_bytes;
        }

        for err in &result.errors {
            output::print_delete_error("", err);
        }
    }

    output::print_separator();
    output::print_clean_complete(&output::format_size(grand_total));
}
