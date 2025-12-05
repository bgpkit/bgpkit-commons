use bgpkit_commons::rpki::{RpkiTrie, RpkiViewsCollector};
use chrono::NaiveDate;

fn main() {
    println!("Counting ASPA objects on the first day of each year (2020-2025)");
    println!("{}", "=".repeat(60));

    for year in 2020..=2025 {
        let date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();

        // Try RIPE historical first
        match RpkiTrie::from_ripe_historical(date) {
            Ok(trie) => {
                println!("{}-01-01: {} ASPAs (from RIPE)", year, trie.aspas.len());
                continue;
            }
            Err(_) => {
                // RIPE failed, try RPKIviews
            }
        }

        // Fallback to RPKIviews
        match RpkiTrie::from_rpkiviews(RpkiViewsCollector::default(), date) {
            Ok(trie) => {
                println!(
                    "{}-01-01: {} ASPAs (from RPKIviews)",
                    year,
                    trie.aspas.len()
                );
            }
            Err(_) => {
                println!("{}-01-01: No data available", year);
            }
        }
    }

    println!("{}", "=".repeat(60));
    println!("Done!");
}
