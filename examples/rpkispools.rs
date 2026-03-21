//! Example demonstrating RPKI data loading from RPKISPOOL archives (CCR format)
//!
//! Run with: cargo run --example rpkispools --features rpki

use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::rpki::{HistoricalRpkiSource, RpkiSpoolsCollector};
use chrono::NaiveDate;

fn main() {
    tracing_subscriber::fmt::init();

    let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let mut commons = BgpkitCommons::new();

    println!("Loading RPKISPOOL data for {} ...", date);
    let source = HistoricalRpkiSource::RpkiSpools(RpkiSpoolsCollector::default());
    commons
        .load_rpki_historical(date, source)
        .expect("failed to load RPKISPOOL data");

    let prefix = "1.1.1.0/24";
    println!("\nROAs covering {}:", prefix);
    match commons.rpki_lookup_by_prefix(prefix) {
        Ok(roas) => {
            for roa in &roas {
                println!(
                    "  prefix={} AS{} max_length={}",
                    roa.prefix, roa.asn, roa.max_length,
                );
            }
            if roas.is_empty() {
                println!("  (none)");
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Validate Cloudflare's origin for 1.1.1.0/24
    let asn = 13335;
    println!(
        "\nValidation for {} origin AS{}: {:?}",
        prefix,
        asn,
        commons.rpki_validate(asn, prefix).unwrap()
    );

    // Look up ASPA for AS400644
    let customer_asn = 400644;
    println!("\nASPA for AS{}:", customer_asn);
    match commons.rpki_lookup_aspa(customer_asn) {
        Ok(Some(aspa)) => {
            println!(
                "  customer AS{} -> providers: {:?}",
                aspa.customer_asn, aspa.providers
            );
        }
        Ok(None) => println!("  (no ASPA found)"),
        Err(e) => println!("  Error: {}", e),
    }
}
