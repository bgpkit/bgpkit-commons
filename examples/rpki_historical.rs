//! Example demonstrating historical RPKI data loading from RIPE NCC and RPKIviews
//!
//! Run with: cargo run --example rpki_historical --features rpki

use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::rpki::{HistoricalRpkiSource, RpkiViewsCollector};
use chrono::NaiveDate;

fn main() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let commons = BgpkitCommons::new();

    println!("=== Listing available RPKI files for {} ===\n", date);

    // List files from RIPE NCC (one file per RIR)
    println!("RIPE NCC files:");
    match commons.list_rpki_files(date, HistoricalRpkiSource::Ripe) {
        Ok(files) => {
            for file in &files {
                println!(
                    "  - {} (RIR: {})",
                    file.url,
                    file.rir
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "N/A".to_string())
                );
            }
        }
        Err(e) => println!("  Error listing RIPE files: {}", e),
    }
    println!();

    // List files from RPKIviews (multiple snapshots per day)
    println!("RPKIviews files (Kerfuffle collector):");
    let source = HistoricalRpkiSource::RpkiViews(RpkiViewsCollector::Kerfuffle);
    match commons.list_rpki_files(date, source) {
        Ok(files) => {
            println!("  Found {} files for {}", files.len(), date);
            // Show first 5 files
            for file in files.iter().take(5) {
                println!(
                    "  - {} ({} bytes, timestamp: {})",
                    file.url,
                    file.size.unwrap_or(0),
                    file.timestamp
                );
            }
            if files.len() > 5 {
                println!("  ... and {} more files", files.len() - 5);
            }
        }
        Err(e) => println!("  Error listing RPKIviews files: {}", e),
    }
    println!();

    // Show available collectors
    println!("Available RPKIviews collectors:");
    for collector in RpkiViewsCollector::all() {
        println!("  - {} ({})", collector, collector.base_url());
    }
}
