//! PeeringDB table structs — faithful mirrors of the PeeringDB API responses.
//!
//! Each struct captures all fields returned by its corresponding API endpoint.
//! Fields use `Option<T>` where the API may return `null`.

use serde::{Deserialize, Serialize};

use super::HasId;

/// Social media entry (appears in several endpoints).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMediaEntry {
    pub service: String,
    pub identifier: String,
}

// ===========================================================================
// /net — Network (AS) information
// ===========================================================================

/// PeeringDB network (`/net`) — full record with all API fields.
///
/// One record per registered autonomous system in PeeringDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub id: u32,
    pub asn: Option<u32>,
    pub name: Option<String>,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub org_id: Option<u32>,
    pub irr_as_set: Option<String>,
    pub website: Option<String>,

    // Network characteristics
    pub info_traffic: Option<String>,
    pub info_scope: Option<String>,
    pub info_type: Option<String>,
    pub info_types: Option<Vec<String>>,
    pub info_ratio: Option<String>,
    pub info_prefixes4: Option<u32>,
    pub info_prefixes6: Option<u32>,
    pub info_ipv6: Option<bool>,
    pub info_unicast: Option<bool>,
    pub info_multicast: Option<bool>,
    pub info_never_via_route_servers: Option<bool>,

    // Peering policy
    pub policy_general: Option<String>,
    pub policy_url: Option<String>,
    pub policy_contracts: Option<String>,
    pub policy_locations: Option<String>,
    pub policy_ratio: Option<bool>,

    // IXP / facility links
    pub route_server: Option<String>,
    pub looking_glass: Option<String>,
    pub ix_count: Option<u32>,
    pub fac_count: Option<u32>,

    // Administrative
    pub status: Option<String>,
    pub allow_ixp_update: Option<bool>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub notes: Option<String>,
    pub rir_status: Option<String>,
    pub rir_status_updated: Option<String>,
    pub status_dashboard: Option<String>,
    pub poc_updated: Option<String>,
    pub netixlan_updated: Option<String>,
    pub netfac_updated: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for Network {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /ix — Internet exchange point
// ===========================================================================

/// PeeringDB internet exchange (`/ix`) — full record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternetExchange {
    pub id: u32,
    pub name: String,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub org_id: Option<u32>,

    // Location
    pub city: Option<String>,
    pub country: Option<String>,
    pub region_continent: Option<String>,

    // Protocols
    pub proto_unicast: Option<bool>,
    pub proto_ipv6: Option<bool>,
    pub proto_multicast: Option<bool>,

    // Statistics
    pub net_count: Option<u32>,
    pub fac_count: Option<u32>,
    pub ixf_net_count: Option<u32>,

    // Service level
    pub media: Option<String>,
    pub service_level: Option<String>,
    pub terms: Option<String>,

    // Contact
    pub website: Option<String>,
    pub url_stats: Option<String>,
    pub tech_email: Option<String>,
    pub tech_phone: Option<String>,
    pub policy_email: Option<String>,
    pub policy_phone: Option<String>,
    pub sales_email: Option<String>,
    pub sales_phone: Option<String>,

    // IX-F import
    pub ixf_import_request: Option<String>,
    pub ixf_import_request_status: Option<String>,
    pub ixf_last_import: Option<String>,

    // Administrative
    pub status: Option<String>,
    pub status_dashboard: Option<String>,
    pub notes: Option<String>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for InternetExchange {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /ixlan — IXP peering LAN
// ===========================================================================

/// PeeringDB IXP peering LAN (`/ixlan`) — full record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxLan {
    pub id: u32,
    pub ix_id: u32,
    pub name: Option<String>,
    pub descr: Option<String>,
    pub mtu: Option<u32>,
    pub rs_asn: Option<u32>,
    pub dot1q_support: Option<bool>,
    pub arp_sponge: Option<String>,
    pub ixf_ixp_import_enabled: Option<bool>,
    pub ixf_ixp_member_list_url_visible: Option<String>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for IxLan {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /ixpfx — IXP peering LAN prefix
// ===========================================================================

/// PeeringDB IXP prefix (`/ixpfx`) — peering LAN prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxPrefix {
    pub id: u32,
    pub prefix: String,
    pub protocol: String,
    pub ixlan_id: u32,
    pub in_dfz: bool,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for IxPrefix {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /netixlan — Network–IXP membership
// ===========================================================================

/// PeeringDB network–IXP membership (`/netixlan`) — represents an AS's
/// presence at a specific IXP peering LAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIxLan {
    pub id: u32,
    pub asn: u32,
    pub net_id: u32,
    pub ix_id: u32,
    pub ixlan_id: u32,
    pub name: Option<String>,
    pub speed: u64,
    pub is_rs_peer: bool,
    pub operational: bool,
    pub bfd_support: Option<bool>,
    pub ipaddr4: Option<String>,
    pub ipaddr6: Option<String>,
    pub notes: Option<String>,
    pub ix_side_id: Option<u32>,
    pub net_side_id: Option<u32>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for NetworkIxLan {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /fac — Facility (data center)
// ===========================================================================

/// PeeringDB facility (`/fac`) — full record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facility {
    pub id: u32,
    pub name: String,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub org_id: Option<u32>,
    pub org_name: Option<String>,
    pub campus_id: Option<u32>,

    // Location
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zipcode: Option<String>,
    pub region_continent: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub clli: Option<String>,
    pub npanxx: Option<String>,
    pub floor: Option<String>,
    pub suite: Option<String>,
    pub property: Option<String>,
    pub rencode: Option<String>,
    pub diverse_serving_substations: Option<bool>,

    // Statistics
    pub net_count: Option<u32>,
    pub ix_count: Option<u32>,
    pub carrier_count: Option<u32>,

    // Utilities
    pub available_voltage_services: Option<Vec<String>>,

    // Contact
    pub website: Option<String>,
    pub tech_email: Option<String>,
    pub tech_phone: Option<String>,
    pub sales_email: Option<String>,
    pub sales_phone: Option<String>,

    // Administrative
    pub status: Option<String>,
    pub status_dashboard: Option<String>,
    pub notes: Option<String>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for Facility {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /netfac — Network–facility co-location
// ===========================================================================

/// PeeringDB network–facility co-location (`/netfac`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFacility {
    pub id: u32,
    pub net_id: u32,
    pub fac_id: u32,
    pub local_asn: u32,
    pub name: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for NetworkFacility {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /ixfac — IXP–facility association
// ===========================================================================

/// PeeringDB IXP–facility association (`/ixfac`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxFacility {
    pub id: u32,
    pub ix_id: u32,
    pub fac_id: u32,
    pub name: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for IxFacility {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /org — Organization
// ===========================================================================

/// PeeringDB organization (`/org`) — full record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: u32,
    pub name: String,
    pub name_long: Option<String>,
    pub aka: Option<String>,

    // Location
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zipcode: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub floor: Option<String>,
    pub suite: Option<String>,

    // Contact
    pub website: Option<String>,

    // Administrative
    pub status: Option<String>,
    pub notes: Option<String>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for Organization {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /campus — Facility campus
// ===========================================================================

/// PeeringDB campus (`/campus`) — a group of co-located facilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campus {
    pub id: u32,
    pub name: String,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub org_id: Option<u32>,
    pub org_name: Option<String>,

    // Location
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zipcode: Option<String>,

    // Contact
    pub website: Option<String>,

    // Administrative
    pub status: Option<String>,
    pub notes: Option<String>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for Campus {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /carrier — Transport carrier
// ===========================================================================

/// PeeringDB carrier (`/carrier`) — a transport provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carrier {
    pub id: u32,
    pub name: String,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub org_id: Option<u32>,
    pub org_name: Option<String>,

    // Statistics
    pub fac_count: Option<u32>,

    // Contact
    pub website: Option<String>,

    // Administrative
    pub status: Option<String>,
    pub notes: Option<String>,
    pub social_media: Option<Vec<SocialMediaEntry>>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for Carrier {
    fn id(&self) -> u32 {
        self.id
    }
}

// ===========================================================================
// /carrierfac — Carrier–facility association
// ===========================================================================

/// PeeringDB carrier–facility association (`/carrierfac`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierFacility {
    pub id: u32,
    pub carrier_id: u32,
    pub fac_id: u32,
    pub name: Option<String>,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl HasId for CarrierFacility {
    fn id(&self) -> u32 {
        self.id
    }
}
