#![cfg(feature = "asinfo")]

/// Integration test for basic AS information retrieval.
#[test]
fn test_basic_info() {
    // Create a new instance of BgpkitCommons.
    let mut commons = bgpkit_commons::BgpkitCommons::new();

    // Load AS information (core asn.txt only).
    commons
        .load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Minimum)
        .unwrap();

    // Assert that the AS name for AS number 3333 is correct.
    assert_eq!(
        commons.asinfo_get(3333).unwrap().unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );

    // Assert that the AS name for AS number 400644 is correct.
    assert!(
        commons
            .asinfo_get(400644)
            .unwrap()
            .unwrap()
            .name
            .contains("BGPKIT")
    );

    // Assert that the country for AS number 400644 is correct.
    assert_eq!(commons.asinfo_get(400644).unwrap().unwrap().country, "US");

    // Retrieve all AS information and assert that the AS name for AS number 3333 is correct.
    let all_asinfo = commons.asinfo_all().unwrap();
    assert_eq!(
        all_asinfo.get(&3333).unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );
}

#[test]
fn test_loading_cached() {
    // Create a new instance of BgpkitCommons.
    let mut commons = bgpkit_commons::BgpkitCommons::new();

    // Load AS information previously generated and cached.
    commons.load_asinfo_cached().unwrap();

    // Assert that the AS name for AS number 3333 is correct.
    assert_eq!(
        commons.asinfo_get(3333).unwrap().unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );

    let bgpkit_info = commons.asinfo_get(400644).unwrap().unwrap();

    // Assert that the AS name for AS number 400644 contains "BGPKIT-LLC".
    // (The full name string may vary as upstream asn.txt is updated.)
    assert!(bgpkit_info.name.contains("BGPKIT-LLC"));

    // Assert that the country for AS number 400644 is correct.
    assert_eq!(bgpkit_info.country, "US");

    // Assert that the additional datatsets are also loaded.
    assert!(bgpkit_info.peeringdb.is_some());
    assert!(bgpkit_info.hegemony.is_some());
    assert!(bgpkit_info.as2org.is_some());

    // make sure the preferred name is retrieved correctly.
    assert_eq!(bgpkit_info.get_preferred_name(), "BGPKIT");

    assert_eq!(
        bgpkit_info.peeringdb.unwrap().irr_as_set.unwrap(),
        "AS400644:AS-BGPKIT"
    );
}

/// Verify IRR enrichment: per-source arrays, route prefixes, as-set memberships,
/// delegated data on all ASNs, and UNKNOWN name fill.
#[test]
#[ignore = "network: downloads asn.txt + delegated stats + IRR dumps (~80MB total)"]
fn test_irr_and_delegated_enrichment() {
    let mut commons = bgpkit_commons::BgpkitCommons::new();
    commons
        .load_asinfo_with_profile(bgpkit_commons::asinfo::AsInfoProfile::Full)
        .unwrap();
    let cf = commons.asinfo_get(13335).unwrap().unwrap();
    assert!(cf.delegated.is_some(), "AS13335 should have delegated data");
    let del = cf.delegated.as_ref().unwrap();
    assert_eq!(del.country, "US");
    assert!(!del.registry.is_empty());
    assert!(!del.status.is_empty());

    // === Per-source IRR arrays ===
    assert!(
        !cf.irr.is_empty(),
        "AS13335 should have IRR data from at least one source"
    );
    // Find the RIPE source entry
    let ripe_entry = cf.irr.iter().find(|i| i.source == "RIPE");
    assert!(ripe_entry.is_some(), "AS13335 should have RIPE IRR entry");
    let ripe = ripe_entry.unwrap();
    // AS13335's RIPE aut-num is registered in the ripe-nonauth source, which
    // is not part of the ripe split files the catalog fetches, so the RIPE
    // entry is built from route objects and carries no as_name. Verify the
    // route prefixes instead.
    assert!(
        !ripe.route_prefixes.is_empty(),
        "AS13335 RIPE entry should have route prefixes from ripe.db.route"
    );

    // Aut-num enrichment is verified on AS3333, whose aut-num is in the ripe
    // source with a stable as-name.
    let ncc = commons.asinfo_get(3333).unwrap().unwrap();
    let ncc_ripe = ncc.irr.iter().find(|i| i.source == "RIPE");
    assert!(ncc_ripe.is_some(), "AS3333 should have RIPE IRR entry");
    assert_eq!(
        ncc_ripe.unwrap().as_name,
        "RIPE-NCC-AS",
        "AS3333 RIPE as_name should be RIPE-NCC-AS"
    );

    // === Route prefixes ===
    // AS13335 should have route prefixes from RADB (1.1.1.0/24 etc.)
    let radb_entry = cf.irr.iter().find(|i| i.source == "RADB");
    if let Some(radb) = radb_entry {
        assert!(
            !radb.route_prefixes.is_empty(),
            "AS13335 should have route prefixes from RADB"
        );
        assert!(
            radb.route_prefixes
                .iter()
                .any(|prefix| prefix.addr().octets().starts_with(&[1, 1, 1])),
            "AS13335 RADB routes should include 1.1.1.x, got: {:?}",
            radb.route_prefixes.iter().take(5).collect::<Vec<_>>()
        );
    }

    // === UNKNOWN name fill ===
    if let Some(info) = commons.asinfo_get(219125).unwrap() {
        assert_ne!(
            info.name, "UNKNOWN",
            "AS219125 name should be filled from IRR, not UNKNOWN"
        );
        let ripe = info.irr.iter().find(|i| i.source == "RIPE");
        assert!(ripe.is_some(), "AS219125 should have RIPE IRR entry");
        assert_eq!(ripe.unwrap().as_name, "ACKENS");
    }

    // === Delegated data on gap ASNs ===
    if let Some(info) = commons.asinfo_get(219157).unwrap() {
        assert!(
            info.delegated.is_some(),
            "AS219157 should have delegated data"
        );
        assert_eq!(info.delegated.as_ref().unwrap().country, "GB");
    }

    // === Summary counts ===
    let all = commons.asinfo_all().unwrap();
    let irr_count = all.values().filter(|i| !i.irr.is_empty()).count();
    let delegated_count = all.values().filter(|i| i.delegated.is_some()).count();
    let unknown_count = all.values().filter(|info| info.name == "UNKNOWN").count();
    let routes_count = all
        .values()
        .filter(|i| i.irr.iter().any(|s| !s.route_prefixes.is_empty()))
        .count();
    eprintln!("ASNs with IRR data: {irr_count}");
    eprintln!("ASNs with delegated data: {delegated_count}");
    eprintln!("ASNs with IRR route prefixes: {routes_count}");
    eprintln!("ASNs still UNKNOWN: {unknown_count}");

    assert!(
        irr_count > 50_000,
        "expected >50k with IRR data, got {irr_count}"
    );
    assert!(
        delegated_count > 120_000,
        "expected >120k with delegated data, got {delegated_count}"
    );
    assert!(
        unknown_count < 500,
        "expected <500 UNKNOWN, got {unknown_count}"
    );
}
