//! Integration tests for the IRR module using live IRR dump sources.
//!
//! These tests download real RPSL data from IRR registries and verify
//! that the parser correctly extracts typed objects for well-known ASNs.

#![cfg(feature = "irr")]

use std::collections::HashMap;

use bgpkit_commons::irr;
use bgpkit_commons::irr::sources::{DumpFormat, Transport, default_sources};
use bgpkit_commons::irr::types::{IrrObject, IrrObjectType};

/// Parse RIPE aut-num split file and verify known ASNs are present.
#[test]
#[ignore = "network: downloads ~8MB from RIPE"]
fn test_ripe_autnum_cloudflare_and_google() {
    let source = default_sources()
        .into_iter()
        .find(|s| s.name == "RIPE")
        .expect("RIPE source should be in defaults");

    let dump_urls = source.dump_urls(IrrObjectType::AutNum);
    assert_eq!(dump_urls.len(), 1);

    let mut by_asn: HashMap<u32, irr::AutNum> = HashMap::new();
    let stats = irr::parse_dump(&dump_urls[0], |obj| {
        if let IrrObject::AutNum(a) = obj {
            by_asn.insert(a.asn, a);
        }
    })
    .unwrap();

    // Should parse tens of thousands of objects
    assert!(
        stats.extracted > 10_000,
        "expected >10k objects, got {}",
        stats.extracted
    );

    // AS13335 = Cloudflare (registered in RIPE region via European entity)
    if let Some(cf) = by_asn.get(&13335) {
        assert!(
            cf.as_name.contains("CLOUDFLARE") || cf.as_name.contains("Cloudflare"),
            "AS13335 as_name should contain CLOUDFLARE, got: {}",
            cf.as_name
        );
        assert_eq!(cf.source, "RIPE");
    }

    // AS15169 = Google
    if let Some(google) = by_asn.get(&15169) {
        assert!(
            google.as_name.contains("GOOGLE"),
            "AS15169 as_name should contain GOOGLE, got: {}",
            google.as_name
        );
    }

    // AS1299 = Arelion/Telia (large European transit, definitely in RIPE)
    assert!(
        by_asn.contains_key(&1299),
        "AS1299 (Arelion) should be present in RIPE aut-num dump"
    );
}

/// Parse RIPE route split file and verify known prefixes.
#[test]
#[ignore = "network: downloads ~12MB from RIPE"]
fn test_ripe_route_objects() {
    let source = default_sources()
        .into_iter()
        .find(|s| s.name == "RIPE")
        .expect("RIPE source should be in defaults");

    let dump_urls = source.dump_urls(IrrObjectType::Route);
    assert_eq!(dump_urls.len(), 1);

    let mut route_count = 0usize;
    let mut saw_origin_13335 = false;

    let stats = irr::parse_dump(&dump_urls[0], |obj| {
        if let IrrObject::Route(r) = obj {
            route_count += 1;
            if r.origin == 13335 {
                saw_origin_13335 = true;
            }
        }
    })
    .unwrap();

    assert!(
        stats.extracted > 100_000,
        "expected >100k route objects from RIPE, got {}",
        stats.extracted
    );
    // Cloudflare has route objects in RIPE
    assert!(
        saw_origin_13335,
        "expected to find at least one route with origin AS13335"
    );
}

/// Parse RADB via FTP and verify it works.
/// RADB is the largest third-party IRR and is FTP-only.
#[test]
#[ignore = "network: downloads ~25MB via FTP from RADB"]
fn test_radb_ftp_route_objects() {
    let source = default_sources()
        .into_iter()
        .find(|s| s.name == "RADB")
        .expect("RADB should be in defaults");

    assert_eq!(source.transport, Transport::Ftp);

    let dump_urls = source.dump_urls(IrrObjectType::Route);
    assert_eq!(dump_urls.len(), 1);
    assert_eq!(dump_urls[0].transport, Transport::Ftp);

    let mut route_count = 0usize;
    let mut saw_cloudflare = false;

    let stats = irr::parse_dump(&dump_urls[0], |obj| {
        if let IrrObject::Route(r) = obj {
            route_count += 1;
            if r.origin == 13335 && r.prefix.to_string().starts_with("1.1.1") {
                saw_cloudflare = true;
            }
        }
    })
    .unwrap();

    // RADB has over 1 million route objects
    assert!(
        stats.extracted > 500_000,
        "expected >500k route objects from RADB, got {}",
        stats.extracted
    );
    // 1.1.1.0/24 via AS13335 is a well-known RADB entry
    assert!(
        saw_cloudflare,
        "expected to find 1.1.1.0/24 origin AS13335 in RADB"
    );
}

/// Parse ARIN whole-DB and verify aut-num objects.
/// ARIN's IRR is opt-in only, so coverage is sparser.
#[test]
#[ignore = "network: downloads ~5MB from ARIN"]
fn test_arin_whole_db_autnum() {
    let source = default_sources()
        .into_iter()
        .find(|s| s.name == "ARIN")
        .expect("ARIN should be in defaults");

    assert_eq!(source.format, DumpFormat::WholeDb);

    let dump_urls = source.dump_urls(IrrObjectType::AutNum);
    assert_eq!(dump_urls.len(), 1);
    // Whole-DB URL is used for all object types
    assert_eq!(
        source.dump_urls(IrrObjectType::Route)[0].url,
        dump_urls[0].url
    );

    let mut autnum_count = 0usize;
    let mut other_count = 0usize;

    irr::parse_dump(&dump_urls[0], |obj| match obj {
        IrrObject::AutNum(_) => autnum_count += 1,
        _ => other_count += 1,
    })
    .unwrap();

    // ARIN has ~4k aut-num objects
    assert!(
        autnum_count > 1_000,
        "expected >1k aut-num objects from ARIN, got {}",
        autnum_count
    );
    // Whole-DB should also yield route objects
    assert!(
        other_count > 10_000,
        "expected >10k other objects (routes etc.) from ARIN whole-DB, got {}",
        other_count
    );
}

/// Verify that APNIC split files work correctly.
#[test]
#[ignore = "network: downloads ~2MB from APNIC"]
fn test_apnic_autnum() {
    let source = default_sources()
        .into_iter()
        .find(|s| s.name == "APNIC")
        .expect("APNIC should be in defaults");

    let dump_urls = source.dump_urls(IrrObjectType::AutNum);
    assert_eq!(dump_urls.len(), 1);
    assert_eq!(dump_urls[0].transport, Transport::Https);

    let mut autnum_count = 0usize;

    let _stats = irr::parse_dump(&dump_urls[0], |obj| {
        if let IrrObject::AutNum(a) = obj {
            autnum_count += 1;
            assert_eq!(a.source, "APNIC");
        }
    })
    .unwrap();

    assert!(
        autnum_count > 1_000,
        "expected >1k aut-num from APNIC, got {}",
        autnum_count
    );
}

/// Verify source registry covers all default sources.
#[test]
fn test_default_sources_coverage() {
    let sources = default_sources();
    let names: Vec<&str> = sources.iter().map(|s| s.name).collect();

    // Must include the 5 RIRs + RADB + NTTCOM
    for required in &[
        "RIPE", "APNIC", "ARIN", "LACNIC", "AFRINIC", "RADB", "NTTCOM",
    ] {
        assert!(
            names.contains(required),
            "default sources should include {required}"
        );
    }
}

/// Verify all default sources have URLs for each supported object type.
#[test]
fn test_default_sources_have_urls() {
    for source in default_sources() {
        for obj_type in IrrObjectType::all() {
            let urls = source.dump_urls(*obj_type);
            assert!(
                !urls.is_empty(),
                "source {} should have a URL for {:?}",
                source.name,
                obj_type
            );
        }
    }
}

/// Verify IrrDumpUrl URLs point to the right transports.
#[test]
fn test_transport_correctness() {
    for source in default_sources() {
        let urls = source.dump_urls(IrrObjectType::AutNum);
        for url in &urls {
            assert_eq!(
                url.transport, source.transport,
                "source {} URL transport mismatch",
                source.name
            );
            if url.transport == Transport::Ftp {
                assert!(
                    url.url.starts_with("ftp://"),
                    "FTP source {} URL should start with ftp://",
                    source.name
                );
            } else {
                assert!(
                    url.url.starts_with("https://"),
                    "HTTPS source {} URL should start with https://",
                    source.name
                );
            }
        }
    }
}

/// Verify all_sources() includes more registries than default_sources().
#[test]
fn test_all_sources_superset() {
    let all = irr::all_sources();
    let defaults = default_sources();

    assert!(
        all.len() > defaults.len(),
        "all_sources should have more entries than defaults"
    );

    for default_src in &defaults {
        assert!(
            all.iter().any(|s| s.name == default_src.name),
            "default source {} should also be in all_sources",
            default_src.name
        );
    }

    // all_sources should include regional registries not in defaults
    let all_names: Vec<&str> = all.iter().map(|s| s.name).collect();
    for expected in &["ALTDB", "BELL", "BBOI", "JPIRR", "TC", "CANARIE", "REACH"] {
        assert!(
            all_names.contains(expected),
            "all_sources should include {expected}"
        );
    }
}
