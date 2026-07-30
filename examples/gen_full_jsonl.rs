//! Generate JSONL output for size comparison: with and without route prefixes.
use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::asinfo::AsInfoBuilder;
use std::io::Write;

fn main() {
    tracing_subscriber::fmt::init();

    // Variant 1: full IRR but WITHOUT route prefixes (default)
    {
        eprintln!("Loading WITHOUT route prefixes...");
        let mut commons = BgpkitCommons::new();
        let _builder = AsInfoBuilder::new()
            .with_as2org()
            .with_population()
            .with_hegemony()
            .with_peeringdb()
            .with_delegated()
            .with_irr();
        commons
            .load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Full)
            .unwrap();

        let all = commons.asinfo_all().unwrap();
        let mut f = std::fs::File::create("/tmp/asinfo_no_routes.jsonl").unwrap();
        for asn in all
            .keys()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
        {
            let json = serde_json::to_string(&all[asn]).unwrap();
            writeln!(f, "{json}").unwrap();
        }
        drop(f);
        eprintln!("Written {} ASNs (no routes)", all.len());
    }

    // Variant 2: full IRR WITH route prefixes
    {
        eprintln!("Loading WITH route prefixes...");
        let mut commons = BgpkitCommons::new();
        let _builder = AsInfoBuilder::new()
            .with_as2org()
            .with_population()
            .with_hegemony()
            .with_peeringdb()
            .with_delegated()
            .with_irr()
            .with_irr_route_prefixes();
        commons
            .load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Full)
            .unwrap();

        let all = commons.asinfo_all().unwrap();
        let mut f = std::fs::File::create("/tmp/asinfo_with_routes.jsonl").unwrap();
        for asn in all
            .keys()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
        {
            let json = serde_json::to_string(&all[asn]).unwrap();
            writeln!(f, "{json}").unwrap();
        }
        drop(f);
        eprintln!("Written {} ASNs (with routes)", all.len());
    }
}
