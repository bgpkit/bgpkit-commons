#![cfg(feature = "delegated")]

use std::io::Cursor;

use bgpkit_commons::delegated::parse_reader;

#[test]
fn parser_preserves_source_records_without_asinfo_policy() {
    let input = "\
ripencc|GB|asn|219157|1|20260722|allocated
arin|US|asn|300000|1|20200101|reserved
ripencc|NL|ipv4|185.0.0.0|65536|20000101|allocated
";

    let records = parse_reader(Cursor::new(input))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(records.len(), 3);
    assert_eq!(records[0].registry, "ripencc");
    assert_eq!(records[0].country, "GB");
    assert_eq!(records[0].record_type, "asn");
    assert_eq!(records[0].start, "219157");
    assert_eq!(records[0].value, "1");
    assert_eq!(records[0].status, "allocated");
    assert_eq!(records[1].status, "reserved");
    assert_eq!(records[2].record_type, "ipv4");
}

#[test]
fn parser_reports_malformed_records() {
    let input = "ripencc|GB|asn\n";
    let mut records = parse_reader(Cursor::new(input));

    assert!(records.next().unwrap().is_err());
}
