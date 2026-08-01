#![cfg(feature = "asinfo")]

use bgpkit_commons::asinfo::{AsInfo, IrrAsnInfo};
use ipnet::{Ipv4Net, Ipv6Net};
use serde_json::{Value, json};

#[test]
fn old_records_without_new_fields_still_deserialize() {
    let value = json!({
        "asn": 13335,
        "name": "CLOUDFLARENET",
        "country": "US"
    });

    let info: AsInfo = serde_json::from_value(value).unwrap();
    assert!(info.delegated.is_none());
    assert!(info.irr.is_empty());
}

#[test]
fn absent_enrichments_are_omitted_from_serialization() {
    let info = AsInfo {
        asn: 13335,
        name: "CLOUDFLARENET".to_string(),
        country: "US".to_string(),
        as2org: None,
        population: None,
        hegemony: None,
        peeringdb: None,
        delegated: None,
        irr: Vec::new(),
    };

    let value = serde_json::to_value(info).unwrap();
    assert_eq!(
        value,
        Value::Object(
            [
                ("asn".to_string(), json!(13335)),
                ("name".to_string(), json!("CLOUDFLARENET")),
                ("country".to_string(), json!("US")),
            ]
            .into_iter()
            .collect()
        )
    );
}

#[test]
fn irr_prefixes_are_typed_in_memory_and_cidr_strings_in_json() {
    let irr = IrrAsnInfo {
        as_name: "CLOUDFLARENET".to_string(),
        descr: Vec::new(),
        source: "RADB".to_string(),
        mnt_by: Vec::new(),
        route_prefixes: vec!["1.1.1.0/24".parse::<Ipv4Net>().unwrap()],
        route6_prefixes: vec!["2606:4700::/32".parse::<Ipv6Net>().unwrap()],
        member_of_sets: Vec::new(),
    };

    let value = serde_json::to_value(irr).unwrap();
    assert_eq!(value["route_prefixes"], json!(["1.1.1.0/24"]));
    assert_eq!(value["route6_prefixes"], json!(["2606:4700::/32"]));
}
