//! PeeringDB data module
//!
//! This module provides typed access to PeeringDB's API data, with structs that
//! faithfully mirror the [PeeringDB REST API](https://www.peeringdb.com/api/)
//! endpoints. Each struct captures all fields returned by its corresponding
//! endpoint — no projection or data loss.
//!
//! # Data sources
//!
//! - PeeringDB API: <https://www.peeringdb.com/api/>
//!
//! # PeeringDB API key
//!
//! It is strongly recommended to obtain a [PeeringDB API key](https://docs.peeringdb.com/blog/api_keys/)
//! and set the `PEERINGDB_API_KEY` environment variable.
//! Without it, the API call will likely fail due to rate limiting.
//!
//! # Endpoint coverage
//!
//! | Struct | API endpoint | Description |
//! |--------|------------|-------------|
//! | [`Network`] | `/net` | Network (AS) information |
//! | [`InternetExchange`] | `/ix` | Internet exchange point |
//! | [`IxLan`] | `/ixlan` | IXP peering LAN |
//! | [`IxPrefix`] | `/ixpfx` | IXP peering LAN prefix |
//! | [`NetworkIxLan`] | `/netixlan` | Network–IXP membership |
//! | [`Facility`] | `/fac` | Facility (data center) |
//! | [`NetworkFacility`] | `/netfac` | Network–facility co-location |
//! | [`IxFacility`] | `/ixfac` | IXP–facility association |
//! | [`Organization`] | `/org` | Organization |
//! | [`Campus`] | `/campus` | Facility campus |
//! | [`Carrier`] | `/carrier` | Transport carrier |
//! | [`CarrierFacility`] | `/carrierfac` | Carrier–facility association |

mod client;
mod tables;

pub use tables::*;

use std::collections::HashMap;

use crate::Result;
use tracing::info;

/// PeeringDB data container with all tables loaded from the API.
///
/// Use [`Peeringdb::new`] to fetch all data from the PeeringDB API, or
/// [`Peeringdb::new_networks_only`] for the lightweight network-only load
/// (backward-compatible with the original behavior).
pub struct Peeringdb {
    /// Network records keyed by ASN (`/net`).
    pub networks: HashMap<u32, Network>,
    /// Internet exchange records keyed by `ix_id` (`/ix`).
    pub internet_exchanges: HashMap<u32, InternetExchange>,
    /// IXP peering-LAN records keyed by `ixlan_id` (`/ixlan`).
    pub ixp_lans: HashMap<u32, IxLan>,
    /// IXP prefix records (`/ixpfx`).
    pub ixp_prefixes: Vec<IxPrefix>,
    /// Network–IXP membership records (`/netixlan`).
    pub network_ixp_membership: Vec<NetworkIxLan>,
    /// Facility records keyed by `fac_id` (`/fac`).
    pub facilities: HashMap<u32, Facility>,
    /// Network–facility co-location records (`/netfac`).
    pub network_facilities: Vec<NetworkFacility>,
    /// IXP–facility association records (`/ixfac`).
    pub ixp_facilities: Vec<IxFacility>,
    /// Organization records keyed by `org_id` (`/org`).
    pub organizations: HashMap<u32, Organization>,
    /// Campus records keyed by `campus_id` (`/campus`).
    pub campuses: HashMap<u32, Campus>,
    /// Carrier records keyed by `carrier_id` (`/carrier`).
    pub carriers: HashMap<u32, Carrier>,
    /// Carrier–facility association records (`/carrierfac`).
    pub carrier_facilities: Vec<CarrierFacility>,
}

impl Peeringdb {
    /// Fetch all PeeringDB tables from the API.
    ///
    /// This makes multiple API calls (one per endpoint). Requires
    /// `PEERINGDB_API_KEY` to avoid rate limiting.
    pub fn new() -> Result<Self> {
        info!("loading all PeeringDB tables from API...");
        let networks = Self::load_table::<Network>(client::NET_API_URL)?
            .into_iter()
            .filter_map(|n| n.asn.map(|asn| (asn, n)))
            .collect();
        let internet_exchanges = Self::load_table_id(client::IX_API_URL)?;
        let ixp_lans = Self::load_table_id(client::IXLAN_API_URL)?;
        let ixp_prefixes = Self::load_table(client::IXPFX_API_URL)?;
        let network_ixp_membership = Self::load_table(client::NETIXLAN_API_URL)?;
        let facilities = Self::load_table_id(client::FAC_API_URL)?;
        let network_facilities = Self::load_table(client::NETFAC_API_URL)?;
        let ixp_facilities = Self::load_table(client::IXFAC_API_URL)?;
        let organizations = Self::load_table_id(client::ORG_API_URL)?;
        let campuses = Self::load_table_id(client::CAMPUS_API_URL)?;
        let carriers = Self::load_table_id(client::CARRIER_API_URL)?;
        let carrier_facilities = Self::load_table(client::CARRIERFAC_API_URL)?;
        info!("loaded all PeeringDB tables");
        Ok(Self {
            networks,
            internet_exchanges,
            ixp_lans,
            ixp_prefixes,
            network_ixp_membership,
            facilities,
            network_facilities,
            ixp_facilities,
            organizations,
            campuses,
            carriers,
            carrier_facilities,
        })
    }

    /// Fetch only the `/net` table (lightweight, backward-compatible).
    ///
    /// This is equivalent to the original `Peeringdb::new()` behavior before
    /// the data-layer expansion.
    pub fn new_networks_only() -> Result<Self> {
        info!("loading PeeringDB /net table...");
        let networks: HashMap<u32, Network> = Self::load_table::<Network>(client::NET_API_URL)?
            .into_iter()
            .filter_map(|n| n.asn.map(|asn| (asn, n)))
            .collect();
        info!("loaded {} PeeringDB networks", networks.len());
        Ok(Self {
            networks,
            internet_exchanges: HashMap::new(),
            ixp_lans: HashMap::new(),
            ixp_prefixes: Vec::new(),
            network_ixp_membership: Vec::new(),
            facilities: HashMap::new(),
            network_facilities: Vec::new(),
            ixp_facilities: Vec::new(),
            organizations: HashMap::new(),
            campuses: HashMap::new(),
            carriers: HashMap::new(),
            carrier_facilities: Vec::new(),
        })
    }

    /// Load a table as a `Vec<T>` of records.
    fn load_table<R: serde::de::DeserializeOwned>(url: &str) -> Result<Vec<R>> {
        let mut reader = client::get_peeringdb_reader(url)?;
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        let res: PeeringdbResponse<R> = serde_json::from_str(&buf)?;
        Ok(res.data)
    }

    /// Load a table as a `HashMap<u32, T>` keyed by the `id` field.
    fn load_table_id<T: serde::de::DeserializeOwned + HasId>(url: &str) -> Result<HashMap<u32, T>> {
        Ok(Self::load_table::<T>(url)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect())
    }

    // ---- Network accessors ----

    /// Get network data for a specific ASN.
    pub fn get_network(&self, asn: u32) -> Option<&Network> {
        self.networks.get(&asn)
    }

    /// Check if an ASN exists in the PeeringDB data.
    pub fn contains_network(&self, asn: u32) -> bool {
        self.networks.contains_key(&asn)
    }

    /// Number of networks loaded.
    pub fn network_count(&self) -> usize {
        self.networks.len()
    }

    /// Get all ASNs present in PeeringDB.
    pub fn all_asns(&self) -> Vec<u32> {
        self.networks.keys().copied().collect()
    }

    // ---- IXP accessors ----

    /// Get an internet exchange by its `ix_id`.
    pub fn get_ixp(&self, ix_id: u32) -> Option<&InternetExchange> {
        self.internet_exchanges.get(&ix_id)
    }

    /// Get all IXP memberships for a given ASN.
    pub fn get_ixp_memberships(&self, asn: u32) -> Vec<&NetworkIxLan> {
        self.network_ixp_membership
            .iter()
            .filter(|m| m.asn == asn)
            .collect()
    }

    /// Get all ASNs present at a given IXP.
    pub fn get_asns_at_ixp(&self, ix_id: u32) -> Vec<&NetworkIxLan> {
        self.network_ixp_membership
            .iter()
            .filter(|m| m.ix_id == ix_id)
            .collect()
    }

    /// Look up IXP prefix records by exact prefix string (e.g., `"206.223.115.0/24"`).
    pub fn lookup_ixp_prefix(&self, prefix: &str) -> Vec<&IxPrefix> {
        self.ixp_prefixes
            .iter()
            .filter(|p| p.prefix == prefix)
            .collect()
    }

    /// Check whether a prefix string is an IXP peering-LAN prefix.
    pub fn is_ixp_prefix(&self, prefix: &str) -> bool {
        self.ixp_prefixes.iter().any(|p| p.prefix == prefix)
    }

    /// Get all IXP prefixes.
    pub fn all_ixp_prefixes(&self) -> &[IxPrefix] {
        &self.ixp_prefixes
    }

    // ---- Facility accessors ----

    /// Get a facility by its `fac_id`.
    pub fn get_facility(&self, fac_id: u32) -> Option<&Facility> {
        self.facilities.get(&fac_id)
    }

    /// Get all facilities a network is present at.
    pub fn get_network_facilities(&self, asn: u32) -> Vec<&NetworkFacility> {
        self.network_facilities
            .iter()
            .filter(|nf| nf.local_asn == asn)
            .collect()
    }

    // ---- Organization accessors ----

    /// Get an organization by its `org_id`.
    pub fn get_organization(&self, org_id: u32) -> Option<&Organization> {
        self.organizations.get(&org_id)
    }
}

/// Trait for structs that have an `id` field, used by `load_table_id`.
pub trait HasId {
    fn id(&self) -> u32;
}

#[derive(serde::Deserialize)]
struct PeeringdbResponse<T> {
    data: Vec<T>,
}

impl std::fmt::Debug for Peeringdb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peeringdb")
            .field("networks", &self.networks.len())
            .field("internet_exchanges", &self.internet_exchanges.len())
            .field("ixp_lans", &self.ixp_lans.len())
            .field("ixp_prefixes", &self.ixp_prefixes.len())
            .field("network_ixp_membership", &self.network_ixp_membership.len())
            .field("facilities", &self.facilities.len())
            .field("network_facilities", &self.network_facilities.len())
            .field("ixp_facilities", &self.ixp_facilities.len())
            .field("organizations", &self.organizations.len())
            .field("campuses", &self.campuses.len())
            .field("carriers", &self.carriers.len())
            .field("carrier_facilities", &self.carrier_facilities.len())
            .finish()
    }
}

// ===========================================================================
// Backward compatibility: PeeringdbData and get_data
// ===========================================================================

/// Backward-compatible alias for [`Network`].
///
/// Code written before the data-layer expansion used `PeeringdbData` as the
/// primary PeeringDB type. It is now an alias for the full [`Network`] struct.
pub type PeeringdbData = Network;

impl Peeringdb {
    /// Get PeeringDB data for a specific ASN (backward-compatible accessor).
    ///
    /// This is the original accessor from before the data-layer expansion.
    /// New code should prefer [`Peeringdb::get_network`].
    pub fn get_data(&self, asn: u32) -> Option<&PeeringdbData> {
        self.networks.get(&asn)
    }

    /// Get all ASNs in the PeeringDB data.
    #[allow(dead_code)]
    pub fn get_all_asns(&self) -> Vec<u32> {
        self.all_asns()
    }

    /// Check if an ASN exists in the PeeringDB data.
    #[allow(dead_code)]
    pub fn contains(&self, asn: u32) -> bool {
        self.contains_network(asn)
    }

    /// Get the number of networks in the database.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.network_count()
    }

    /// Check if the database is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.networks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peeringdb_data_is_network() {
        // Compile-time check that PeeringdbData is Network
        let _: PeeringdbData = Network {
            id: 1,
            asn: Some(13335),
            name: Some("Cloudflare".to_string()),
            name_long: None,
            aka: None,
            org_id: None,
            irr_as_set: None,
            website: None,
            info_traffic: None,
            info_scope: None,
            info_type: None,
            info_types: None,
            info_ratio: None,
            info_prefixes4: None,
            info_prefixes6: None,
            info_ipv6: None,
            info_unicast: None,
            info_multicast: None,
            info_never_via_route_servers: None,
            policy_general: None,
            policy_url: None,
            policy_contracts: None,
            policy_locations: None,
            policy_ratio: None,
            route_server: None,
            looking_glass: None,
            ix_count: None,
            fac_count: None,
            status: None,
            allow_ixp_update: None,
            social_media: None,
            notes: None,
            rir_status: None,
            rir_status_updated: None,
            status_dashboard: None,
            poc_updated: None,
            netixlan_updated: None,
            netfac_updated: None,
            created: None,
            updated: None,
        };
    }

    #[test]
    fn test_peeringdb_data_struct() {
        let data = PeeringdbData {
            id: 4224,
            asn: Some(13335),
            name: Some("Cloudflare".to_string()),
            name_long: Some("Cloudflare, Inc.".to_string()),
            aka: Some("CF".to_string()),
            org_id: Some(8061),
            irr_as_set: Some("AS-CLOUDFLARE".to_string()),
            website: Some("https://cloudflare.com".to_string()),
            info_traffic: Some("1-5Tbps+".to_string()),
            info_scope: Some("Global".to_string()),
            info_type: Some("Content".to_string()),
            info_types: Some(vec!["Content".to_string()]),
            info_ratio: Some("Heavy Outbound".to_string()),
            info_prefixes4: Some(300),
            info_prefixes6: Some(150),
            info_ipv6: Some(true),
            info_unicast: Some(true),
            info_multicast: Some(false),
            info_never_via_route_servers: Some(false),
            policy_general: Some("Open".to_string()),
            policy_url: Some("https://www.cloudflare.com/peering-policy".to_string()),
            policy_contracts: None,
            policy_locations: None,
            policy_ratio: Some(false),
            route_server: None,
            looking_glass: None,
            ix_count: Some(300),
            fac_count: Some(200),
            status: Some("ok".to_string()),
            allow_ixp_update: Some(false),
            social_media: Some(vec![]),
            notes: None,
            rir_status: Some("ok".to_string()),
            rir_status_updated: None,
            status_dashboard: None,
            poc_updated: None,
            netixlan_updated: None,
            netfac_updated: None,
            created: None,
            updated: None,
        };
        assert_eq!(data.asn, Some(13335));
        assert_eq!(data.name.as_deref(), Some("Cloudflare"));
        assert_eq!(data.info_traffic.as_deref(), Some("1-5Tbps+"));
        assert_eq!(data.ix_count, Some(300));
    }

    #[test]
    fn test_network_deserialization_minimal() {
        let json = r#"{"id":1,"name":"Test","asn":12345}"#;
        let net: Network = serde_json::from_str(json).unwrap();
        assert_eq!(net.id, 1);
        assert_eq!(net.asn, Some(12345));
        assert_eq!(net.name.as_deref(), Some("Test"));
        assert!(net.website.is_none());
    }

    #[test]
    fn test_ixprefix_deserialization() {
        let json = r#"{"prefix":"206.223.115.0/24","protocol":"IPv4","ixlan_id":1,"in_dfz":true,"id":1,"status":"ok","created":"2020-01-01T00:00:00Z","updated":"2020-01-01T00:00:00Z"}"#;
        let pfx: IxPrefix = serde_json::from_str(json).unwrap();
        assert_eq!(pfx.prefix, "206.223.115.0/24");
        assert_eq!(pfx.protocol, "IPv4");
        assert!(pfx.in_dfz);
    }

    #[test]
    fn test_networkixlan_deserialization() {
        let json = r#"{"id":13,"net_id":694,"ix_id":12,"ixlan_id":12,"asn":8075,"speed":100000,"is_rs_peer":false,"operational":true,"name":"Equinix New York","ipaddr4":"196.201.2.20","ipaddr6":null,"bfd_support":false,"status":"ok","notes":"","created":"2011-09-28T00:00:00Z","updated":"2018-10-11T06:11:28Z"}"#;
        let nxl: NetworkIxLan = serde_json::from_str(json).unwrap();
        assert_eq!(nxl.asn, 8075);
        assert_eq!(nxl.ix_id, 12);
        assert_eq!(nxl.speed, 100000);
        assert!(!nxl.is_rs_peer);
    }

    // Integration tests that require network access - marked as ignored by default

    #[test]
    #[ignore]
    fn test_peeringdb_new_networks_only() {
        let pdb = Peeringdb::new_networks_only().expect("Failed to load PeeringDB");
        assert!(!pdb.is_empty());
        let cf = pdb.get_data(13335);
        assert!(cf.is_some());
        let cf = cf.unwrap();
        assert_eq!(cf.asn, Some(13335));
        assert!(cf.info_traffic.is_some() || cf.info_traffic.is_none()); // just check field exists
    }

    #[test]
    #[ignore]
    fn test_peeringdb_ixp_prefixes() {
        let pdb = Peeringdb::new().expect("Failed to load all PeeringDB");
        // DE-CIX Frankfurt peering LAN prefix
        let decix = pdb.lookup_ixp_prefix("80.81.192.0/21");
        assert!(
            !decix.is_empty(),
            "DE-CIX Frankfurt prefix should be present"
        );
        // Cloudflare should have IXP memberships
        let cf_ixps = pdb.get_ixp_memberships(13335);
        assert!(
            !cf_ixps.is_empty(),
            "Cloudflare should have IXP memberships"
        );
    }
}
