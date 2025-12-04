//! PeeringDB data module
//!
//! This module provides access to PeeringDB data via their public API.
//!
//! # Data source
//! - PeeringDB API: <https://www.peeringdb.com/api/>
//!
//! # PeeringDB API key required
//!
//! It is strongly recommended to obtain a [PeeringDB API key](https://docs.peeringdb.com/blog/api_keys/)
//! and set the `PEERINGDB_API_KEY` environment variable.
//! Without it, the API call will likely fail due to rate limiting.

use crate::{BgpkitCommonsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use tracing::warn;

const PEERINGDB_NET_API_URL: &str = "https://www.peeringdb.com/api/net";

/// PeeringDB network data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeeringdbData {
    pub asn: u32,
    pub name: Option<String>,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub irr_as_set: Option<String>,
    pub website: Option<String>,
}

/// Full PeeringDB network response from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeeringdbNet {
    pub id: u32,
    pub name: Option<String>,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub asn: Option<u32>,
    pub org_id: Option<u32>,
    pub irr_as_set: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub fac_count: Option<usize>,
    pub ix_count: Option<u32>,

    pub policy_contracts: Option<String>,
    pub policy_general: Option<String>,
    pub policy_locations: Option<String>,
    pub policy_ratio: Option<bool>,
    pub policy_url: Option<String>,

    pub info_ipv6: Option<bool>,
    pub info_multicast: Option<bool>,
    pub info_never_via_route_servers: Option<bool>,
    pub info_prefixes4: Option<u32>,
    pub info_prefixes6: Option<u32>,
    pub info_ratio: Option<String>,
    pub info_scope: Option<String>,
    pub info_traffic: Option<String>,
    pub info_type: Option<String>,
    pub info_types: Option<Vec<String>>,
    pub info_unicast: Option<bool>,

    pub rir_status: Option<String>,
    pub status: Option<String>,
    pub status_dashboard: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub route_server: Option<String>,
    pub looking_glass: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PeeringdbNetResponse {
    data: Vec<PeeringdbNet>,
}

/// Get a reader for PeeringDB API with proper authentication headers
fn get_peeringdb_reader(url: &str) -> Result<Box<dyn Read + Send>> {
    // Try to load API key from environment
    let api_key = std::env::var("PEERINGDB_API_KEY").unwrap_or_else(|_| {
        warn!("missing PEERINGDB_API_KEY env var, call may fail due to rate limiting");
        "".to_string()
    });

    let client = oneio::remote::create_client_with_headers([
        ("Authorization".to_string(), format!("Api-Key {}", api_key)),
        (
            "User-Agent".to_string(),
            format!("bgpkit-commons/{}", env!("CARGO_PKG_VERSION")),
        ),
    ])?;

    let res = client
        .execute(client.get(url).build().map_err(|e| {
            BgpkitCommonsError::data_source_error(
                crate::errors::data_sources::PEERINGDB,
                format!("failed to build request: {}", e),
            )
        })?)
        .map_err(|e| {
            BgpkitCommonsError::data_source_error(
                crate::errors::data_sources::PEERINGDB,
                format!("request failed: {}", e),
            )
        })?
        .error_for_status()
        .map_err(|e| {
            BgpkitCommonsError::data_source_error(
                crate::errors::data_sources::PEERINGDB,
                format!("API returned error status: {}", e),
            )
        })?;

    Ok(Box::new(res))
}

/// Load PeeringDB network data from the API
pub fn load_peeringdb_net() -> Result<Vec<PeeringdbNet>> {
    let mut reader = get_peeringdb_reader(PEERINGDB_NET_API_URL)?;
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    let res: PeeringdbNetResponse = serde_json::from_str(&buf)?;
    Ok(res.data)
}

/// PeeringDB data accessor with cached network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peeringdb {
    peeringdb_map: HashMap<u32, PeeringdbData>,
}

impl Peeringdb {
    /// Create a new Peeringdb accessor by loading data from the API
    pub fn new() -> Result<Self> {
        let mut peeringdb_map = HashMap::new();
        let net_vec = load_peeringdb_net()?;

        for net in net_vec {
            if let Some(asn) = net.asn {
                peeringdb_map.entry(asn).or_insert(PeeringdbData {
                    asn,
                    name: net.name,
                    name_long: net.name_long,
                    aka: net.aka,
                    irr_as_set: net.irr_as_set,
                    website: net.website,
                });
            }
        }

        Ok(Self { peeringdb_map })
    }

    /// Get PeeringDB data for a specific ASN
    pub fn get_data(&self, asn: u32) -> Option<&PeeringdbData> {
        self.peeringdb_map.get(&asn)
    }

    /// Get all ASNs in the PeeringDB data
    pub fn get_all_asns(&self) -> Vec<u32> {
        self.peeringdb_map.keys().copied().collect()
    }

    /// Check if an ASN exists in PeeringDB
    pub fn contains(&self, asn: u32) -> bool {
        self.peeringdb_map.contains_key(&asn)
    }

    /// Get the number of networks in the database
    pub fn len(&self) -> usize {
        self.peeringdb_map.len()
    }

    /// Check if the database is empty
    pub fn is_empty(&self) -> bool {
        self.peeringdb_map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peeringdb_data_struct() {
        let data = PeeringdbData {
            asn: 13335,
            name: Some("Cloudflare".to_string()),
            name_long: Some("Cloudflare, Inc.".to_string()),
            aka: Some("CF".to_string()),
            irr_as_set: Some("AS-CLOUDFLARE".to_string()),
            website: Some("https://cloudflare.com".to_string()),
        };
        assert_eq!(data.asn, 13335);
        assert_eq!(data.name, Some("Cloudflare".to_string()));
        assert_eq!(data.name_long, Some("Cloudflare, Inc.".to_string()));
        assert_eq!(data.aka, Some("CF".to_string()));
        assert_eq!(data.irr_as_set, Some("AS-CLOUDFLARE".to_string()));
        assert_eq!(data.website, Some("https://cloudflare.com".to_string()));
    }

    #[test]
    fn test_peeringdb_data_with_none_fields() {
        let data = PeeringdbData {
            asn: 12345,
            name: None,
            name_long: None,
            aka: None,
            irr_as_set: None,
            website: None,
        };
        assert_eq!(data.asn, 12345);
        assert!(data.name.is_none());
        assert!(data.name_long.is_none());
        assert!(data.aka.is_none());
        assert!(data.irr_as_set.is_none());
        assert!(data.website.is_none());
    }

    #[test]
    fn test_peeringdb_data_serialization() {
        let data = PeeringdbData {
            asn: 13335,
            name: Some("Cloudflare".to_string()),
            name_long: Some("Cloudflare, Inc.".to_string()),
            aka: None,
            irr_as_set: Some("AS-CLOUDFLARE".to_string()),
            website: Some("https://cloudflare.com".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"asn\":13335"));
        assert!(json.contains("\"name\":\"Cloudflare\""));

        // Test round-trip
        let deserialized: PeeringdbData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.asn, data.asn);
        assert_eq!(deserialized.name, data.name);
    }

    #[test]
    fn test_peeringdb_data_deserialization() {
        let json = r#"{"asn":13335,"name":"Cloudflare","name_long":"Cloudflare, Inc.","aka":null,"irr_as_set":"AS-CLOUDFLARE","website":"https://cloudflare.com"}"#;
        let data: PeeringdbData = serde_json::from_str(json).unwrap();
        assert_eq!(data.asn, 13335);
        assert_eq!(data.name, Some("Cloudflare".to_string()));
        assert_eq!(data.name_long, Some("Cloudflare, Inc.".to_string()));
        assert!(data.aka.is_none());
        assert_eq!(data.irr_as_set, Some("AS-CLOUDFLARE".to_string()));
    }

    #[test]
    fn test_peeringdb_net_struct() {
        let net = PeeringdbNet {
            id: 1,
            name: Some("Test Network".to_string()),
            name_long: Some("Test Network Inc.".to_string()),
            aka: None,
            asn: Some(12345),
            org_id: Some(100),
            irr_as_set: Some("AS-TEST".to_string()),
            website: Some("https://test.com".to_string()),
            notes: None,
            fac_count: Some(5),
            ix_count: Some(3),
            policy_contracts: None,
            policy_general: Some("Open".to_string()),
            policy_locations: None,
            policy_ratio: None,
            policy_url: None,
            info_ipv6: Some(true),
            info_multicast: Some(false),
            info_never_via_route_servers: Some(false),
            info_prefixes4: Some(100),
            info_prefixes6: Some(50),
            info_ratio: None,
            info_scope: Some("Global".to_string()),
            info_traffic: None,
            info_type: Some("NSP".to_string()),
            info_types: None,
            info_unicast: Some(true),
            rir_status: None,
            status: Some("ok".to_string()),
            status_dashboard: None,
            created: Some("2020-01-01".to_string()),
            updated: Some("2024-01-01".to_string()),
            route_server: None,
            looking_glass: None,
        };
        assert_eq!(net.id, 1);
        assert_eq!(net.asn, Some(12345));
        assert_eq!(net.name, Some("Test Network".to_string()));
        assert_eq!(net.info_prefixes4, Some(100));
    }

    #[test]
    fn test_peeringdb_net_deserialization() {
        let json = r#"{"id":1,"name":"Test","name_long":null,"aka":null,"asn":12345,"org_id":100,"irr_as_set":null,"website":null,"notes":null,"fac_count":null,"ix_count":null,"policy_contracts":null,"policy_general":null,"policy_locations":null,"policy_ratio":null,"policy_url":null,"info_ipv6":null,"info_multicast":null,"info_never_via_route_servers":null,"info_prefixes4":null,"info_prefixes6":null,"info_ratio":null,"info_scope":null,"info_traffic":null,"info_type":null,"info_types":null,"info_unicast":null,"rir_status":null,"status":"ok","status_dashboard":null,"created":null,"updated":null,"route_server":null,"looking_glass":null}"#;
        let net: PeeringdbNet = serde_json::from_str(json).unwrap();
        assert_eq!(net.id, 1);
        assert_eq!(net.asn, Some(12345));
        assert_eq!(net.name, Some("Test".to_string()));
    }

    #[test]
    fn test_peeringdb_net_response_deserialization() {
        let json = r#"{"data":[{"id":1,"name":"Test1","name_long":null,"aka":null,"asn":11111,"org_id":100,"irr_as_set":null,"website":null,"notes":null,"fac_count":null,"ix_count":null,"policy_contracts":null,"policy_general":null,"policy_locations":null,"policy_ratio":null,"policy_url":null,"info_ipv6":null,"info_multicast":null,"info_never_via_route_servers":null,"info_prefixes4":null,"info_prefixes6":null,"info_ratio":null,"info_scope":null,"info_traffic":null,"info_type":null,"info_types":null,"info_unicast":null,"rir_status":null,"status":"ok","status_dashboard":null,"created":null,"updated":null,"route_server":null,"looking_glass":null},{"id":2,"name":"Test2","name_long":null,"aka":null,"asn":22222,"org_id":200,"irr_as_set":null,"website":null,"notes":null,"fac_count":null,"ix_count":null,"policy_contracts":null,"policy_general":null,"policy_locations":null,"policy_ratio":null,"policy_url":null,"info_ipv6":null,"info_multicast":null,"info_never_via_route_servers":null,"info_prefixes4":null,"info_prefixes6":null,"info_ratio":null,"info_scope":null,"info_traffic":null,"info_type":null,"info_types":null,"info_unicast":null,"rir_status":null,"status":"ok","status_dashboard":null,"created":null,"updated":null,"route_server":null,"looking_glass":null}]}"#;
        let response: PeeringdbNetResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].asn, Some(11111));
        assert_eq!(response.data[1].asn, Some(22222));
    }

    #[test]
    fn test_peeringdb_struct_from_hashmap() {
        let mut peeringdb_map = HashMap::new();
        peeringdb_map.insert(
            13335,
            PeeringdbData {
                asn: 13335,
                name: Some("Cloudflare".to_string()),
                name_long: Some("Cloudflare, Inc.".to_string()),
                aka: None,
                irr_as_set: Some("AS-CLOUDFLARE".to_string()),
                website: Some("https://cloudflare.com".to_string()),
            },
        );
        peeringdb_map.insert(
            15169,
            PeeringdbData {
                asn: 15169,
                name: Some("Google".to_string()),
                name_long: Some("Google LLC".to_string()),
                aka: None,
                irr_as_set: Some("AS-GOOGLE".to_string()),
                website: Some("https://google.com".to_string()),
            },
        );

        let peeringdb = Peeringdb { peeringdb_map };

        // Test get_data
        let cf_data = peeringdb.get_data(13335);
        assert!(cf_data.is_some());
        assert_eq!(cf_data.unwrap().name, Some("Cloudflare".to_string()));

        let google_data = peeringdb.get_data(15169);
        assert!(google_data.is_some());
        assert_eq!(google_data.unwrap().name, Some("Google".to_string()));

        // Test non-existent ASN
        let nonexistent = peeringdb.get_data(999999);
        assert!(nonexistent.is_none());

        // Test contains
        assert!(peeringdb.contains(13335));
        assert!(peeringdb.contains(15169));
        assert!(!peeringdb.contains(999999));

        // Test len
        assert_eq!(peeringdb.len(), 2);

        // Test is_empty
        assert!(!peeringdb.is_empty());

        // Test get_all_asns
        let asns = peeringdb.get_all_asns();
        assert_eq!(asns.len(), 2);
        assert!(asns.contains(&13335));
        assert!(asns.contains(&15169));
    }

    #[test]
    fn test_peeringdb_empty() {
        let peeringdb = Peeringdb {
            peeringdb_map: HashMap::new(),
        };
        assert!(peeringdb.is_empty());
        assert_eq!(peeringdb.len(), 0);
        assert!(peeringdb.get_all_asns().is_empty());
        assert!(peeringdb.get_data(12345).is_none());
        assert!(!peeringdb.contains(12345));
    }

    #[test]
    fn test_peeringdb_serialization() {
        let mut peeringdb_map = HashMap::new();
        peeringdb_map.insert(
            13335,
            PeeringdbData {
                asn: 13335,
                name: Some("Cloudflare".to_string()),
                name_long: None,
                aka: None,
                irr_as_set: None,
                website: None,
            },
        );

        let peeringdb = Peeringdb { peeringdb_map };
        let json = serde_json::to_string(&peeringdb).unwrap();
        assert!(json.contains("13335"));
        assert!(json.contains("Cloudflare"));

        // Test round-trip
        let deserialized: Peeringdb = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert!(deserialized.contains(13335));
    }

    // Integration tests that require network access - marked as ignored by default
    #[test]
    #[ignore]
    fn test_load_peeringdb_net() {
        // This test requires PEERINGDB_API_KEY to be set
        let result = load_peeringdb_net();
        assert!(result.is_ok());
        let nets = result.unwrap();
        assert!(!nets.is_empty());
        // Check that we got some networks with ASNs
        let with_asn: Vec<_> = nets.iter().filter(|n| n.asn.is_some()).collect();
        assert!(!with_asn.is_empty());
    }

    #[test]
    #[ignore]
    fn test_peeringdb_new() {
        // This test requires PEERINGDB_API_KEY to be set
        let result = Peeringdb::new();
        assert!(result.is_ok());
        let peeringdb = result.unwrap();
        assert!(!peeringdb.is_empty());

        // Test that we can look up well-known networks
        // Cloudflare (AS13335) should be in PeeringDB
        let cf = peeringdb.get_data(13335);
        assert!(cf.is_some());
    }

    #[test]
    #[ignore]
    fn test_peeringdb_get_data_existing() {
        // This test requires PEERINGDB_API_KEY to be set
        let peeringdb = Peeringdb::new().expect("Failed to load PeeringDB");
        
        // Test with Cloudflare
        let data = peeringdb.get_data(13335);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.asn, 13335);
        // Cloudflare should have a name
        assert!(data.name.is_some());
    }

    #[test]
    #[ignore]
    fn test_peeringdb_get_data_nonexistent() {
        // This test requires PEERINGDB_API_KEY to be set
        let peeringdb = Peeringdb::new().expect("Failed to load PeeringDB");
        
        // Test with a non-existent ASN
        let data = peeringdb.get_data(999999999);
        assert!(data.is_none());
    }
}
