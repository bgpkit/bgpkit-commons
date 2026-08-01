#![cfg(feature = "irr")]

use std::io::Cursor;

use bgpkit_commons::irr::{
    DumpFormat, IrrObject, IrrObjectType, IrrSource, Transport, all_sources, fetch, parse_reader,
    sources_by_name,
};

#[test]
fn raw_parser_preserves_unsupported_objects_and_attribute_order() {
    let input = "\
person:         Jane Doe
remarks:        first
remarks:        second
                continued
source:         TEST
";

    let records = parse_reader(Cursor::new(input), DumpFormat::WholeDb);
    assert_eq!(records.format(), DumpFormat::WholeDb);
    let records = records.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.object_type, "person");
    assert_eq!(record.attributes.len(), 4);
    assert_eq!(record.attributes[1].name, "remarks");
    assert_eq!(record.attributes[1].value, "first");
    assert_eq!(record.attributes[2].name, "remarks");
    assert_eq!(record.attributes[2].value, "second\ncontinued");
    assert_eq!(record.attributes[3].name, "source");
}

#[test]
fn raw_parser_returns_malformed_objects_as_errors() {
    let input = "aut-num: AS13335\nthis is not an attribute\n";
    let mut records = parse_reader(Cursor::new(input), DumpFormat::WholeDb);

    assert!(records.next().unwrap().is_err());
}

#[test]
fn supported_raw_record_converts_to_existing_typed_object() {
    let input = "aut-num: AS13335\nas-name: CLOUDFLARENET\nsource: RIPE\n";
    let record = parse_reader(Cursor::new(input), DumpFormat::SplitFiles)
        .next()
        .unwrap()
        .unwrap();

    match record.to_typed().unwrap() {
        Some(IrrObject::AutNum(aut_num)) => {
            assert_eq!(aut_num.asn, 13335);
            assert_eq!(aut_num.as_name, "CLOUDFLARENET");
            assert_eq!(aut_num.source, "RIPE");
        }
        other => panic!("expected typed aut-num, got {other:?}"),
    }
}

#[test]
fn explicit_source_selection_is_validated_and_deduplicated() {
    let selected = sources_by_name(&["RIPE", "RADB", "RIPE"]).unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|source| source.name)
            .collect::<Vec<_>>(),
        vec!["RIPE", "RADB"]
    );

    assert!(sources_by_name(&["RIPE", "NOT-A-REGISTRY"]).is_err());
    assert!(all_sources().len() > selected.len());
}

#[test]
fn fetch_rejects_a_source_outside_the_catalog_before_network_io() {
    let unknown = IrrSource {
        name: "NOT-A-REGISTRY",
        display_name: "Unknown",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    };

    assert!(fetch(&unknown, IrrObjectType::AutNum).is_err());
}
