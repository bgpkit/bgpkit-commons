//! Benchmark each asinfo data source individually.
use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::asinfo::IrrSourceConfig;
use std::time::Instant;

fn time_it<F: FnOnce()>(label: &str, f: F) {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed();
    eprintln!("{label:50} {elapsed:>8.1?}");
}

fn main() {
    tracing_subscriber::fmt::init();

    eprintln!("=== Individual data source timing ===\n");

    // 1. Core asn.txt only
    time_it("asn.txt only:", || {
        let mut c = BgpkitCommons::new();
        c.load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Minimum)
            .unwrap();
    });

    // 2. + delegated stats
    time_it("asn.txt + delegated:", || {
        let mut c = BgpkitCommons::new();
        let b = c.asinfo_builder().with_delegated();
        c.load_asinfo_with(b).unwrap();
    });

    // 3. IRR sources individually
    for src in &[
        "RIPE", "APNIC", "ARIN", "LACNIC", "AFRINIC", "NTTCOM", "RADB",
    ] {
        time_it(
            &format!("IRR source: {src} (aut-num+route+route6+as-set):"),
            || {
                let mut c = BgpkitCommons::new();
                let b = c
                    .asinfo_builder()
                    .with_irr_sources(IrrSourceConfig::only(&[*src]).unwrap());
                c.load_asinfo_with(b).unwrap();
            },
        );
    }

    // 4. All catalogued IRR sources
    time_it("IRR all sources:", || {
        let mut c = BgpkitCommons::new();
        let b = c.asinfo_builder().with_irr();
        c.load_asinfo_with(b).unwrap();
    });

    // 5. Full load
    time_it(
        "FULL (as2org+population+hegemony+pdb+delegated+irr):",
        || {
            let mut c = BgpkitCommons::new();
            c.load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Full)
                .unwrap();
        },
    );

    eprintln!("\n=== Memory stats ===");
    // Print the map size for the full load
    let mut c = BgpkitCommons::new();
    c.load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Full)
        .unwrap();
    let all = c.asinfo_all().unwrap();
    let irr_count = all.values().filter(|i| !i.irr.is_empty()).count();
    let del_count = all.values().filter(|i| i.delegated.is_some()).count();
    let unknown_count = all.values().filter(|i| i.name == "UNKNOWN").count();
    eprintln!("Total ASNs: {}", all.len());
    eprintln!("ASNs with IRR data: {irr_count}");
    eprintln!("ASNs with delegated data: {del_count}");
    eprintln!("ASNs still UNKNOWN: {unknown_count}");
}
