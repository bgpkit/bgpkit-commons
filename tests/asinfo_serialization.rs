#![cfg(feature = "asinfo")]

use bgpkit_commons::asinfo::{AsInfo, DelegatedInfo, IrrAsnInfo};
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
fn old_records_with_null_enrichment_fields_still_deserialize() {
    // Shape produced by the currently-deployed asninfo cronjob output:
    // every optional enrichment key present, with null for missing data.
    let value = json!({
        "as2org": null,
        "asn": 0,
        "country": "ZZ",
        "hegemony": null,
        "name": "-Reserved AS-",
        "peeringdb": null,
        "population": null
    });

    let info: AsInfo = serde_json::from_value(value).unwrap();
    assert_eq!(info.asn, 0);
    assert_eq!(info.name, "-Reserved AS-");
    assert!(info.as2org.is_none());
    assert!(info.population.is_none());
    assert!(info.hegemony.is_none());
    assert!(info.peeringdb.is_none());
    assert!(info.delegated.is_none());
    assert!(info.irr.is_empty());
}

#[test]
fn newly_serialized_records_round_trip_through_deserialization() {
    // Serialization omits absent optional fields (including `as2org` etc.), so
    // deserialization needs serde defaults on every optional field for the
    // round trip to succeed.
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

    let serialized = serde_json::to_string(&info).unwrap();
    let deserialized: AsInfo = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        serde_json::to_value(&info).unwrap(),
        serde_json::to_value(&deserialized).unwrap()
    );

    // Same round trip with every optional field present.
    let full = AsInfo {
        asn: 13335,
        name: "CLOUDFLARENET".to_string(),
        country: "US".to_string(),
        as2org: Some(bgpkit_commons::asinfo::As2orgInfo {
            name: "CLOUDFLARENET".to_string(),
            country: "US".to_string(),
            org_id: "CLOUDFLARE-INC".to_string(),
            org_name: "Cloudflare, Inc.".to_string(),
        }),
        population: None,
        hegemony: None,
        peeringdb: None,
        delegated: Some(DelegatedInfo {
            registry: "arin".to_string(),
            country: "US".to_string(),
            date: "20100726".to_string(),
            status: "allocated".to_string(),
        }),
        irr: vec![IrrAsnInfo {
            as_name: "CLOUDFLARENET".to_string(),
            descr: Vec::new(),
            source: "RIPE".to_string(),
            mnt_by: Vec::new(),
            route_prefixes: Vec::new(),
            route6_prefixes: Vec::new(),
            member_of_sets: Vec::new(),
        }],
    };
    let serialized = serde_json::to_string(&full).unwrap();
    let deserialized: AsInfo = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        serde_json::to_value(&full).unwrap(),
        serde_json::to_value(&deserialized).unwrap()
    );
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
fn present_enrichments_are_included_in_serialization() {
    let info = AsInfo {
        asn: 13335,
        name: "CLOUDFLARENET".to_string(),
        country: "US".to_string(),
        as2org: Some(bgpkit_commons::asinfo::As2orgInfo {
            name: "CLOUDFLARENET".to_string(),
            country: "US".to_string(),
            org_id: "CLOUDFLARE-INC".to_string(),
            org_name: "Cloudflare, Inc.".to_string(),
        }),
        population: None,
        hegemony: None,
        peeringdb: None,
        delegated: Some(DelegatedInfo {
            registry: "arin".to_string(),
            country: "US".to_string(),
            date: "20100726".to_string(),
            status: "allocated".to_string(),
        }),
        irr: vec![IrrAsnInfo {
            as_name: "CLOUDFLARENET".to_string(),
            descr: Vec::new(),
            source: "RIPE".to_string(),
            mnt_by: Vec::new(),
            route_prefixes: Vec::new(),
            route6_prefixes: Vec::new(),
            member_of_sets: Vec::new(),
        }],
    };

    let value = serde_json::to_value(&info).unwrap();
    assert_eq!(value["as2org"]["org_id"], json!("CLOUDFLARE-INC"));
    assert_eq!(value["delegated"]["registry"], json!("arin"));
    assert_eq!(value["irr"][0]["source"], json!("RIPE"));
    // absent fields stay omitted even when siblings are present
    assert!(value.get("population").is_none());
    assert!(value.get("hegemony").is_none());
    assert!(value.get("peeringdb").is_none());
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
