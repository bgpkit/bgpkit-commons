//! Typed RPSL object representations for IRR data.
//!
//! Each struct corresponds to a specific RPSL object type. Fields are
//! extracted from parsed `rpsl::Object<Raw>` instances and stored as owned
//! data so the original text can be dropped.

use std::collections::BTreeMap;

use ipnet::IpNet;
use serde::{Deserialize, Serialize};

/// RPSL object types that this module knows how to extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrrObjectType {
    /// `aut-num` — AS name/description/maintainer.
    AutNum,
    /// `route` — IPv4 prefix-origin registration.
    Route,
    /// `route6` — IPv6 prefix-origin registration.
    Route6,
    /// `as-set` — named group of ASes (may be recursive).
    AsSet,
    /// `route-set` — named group of prefixes.
    RouteSet,
    /// `mntner` — maintainer object with auth methods.
    Mntner,
    /// `organisation` — organisation details (RIPE) / org (others).
    Organisation,
}

impl IrrObjectType {
    /// The RPSL attribute name that starts an object of this type.
    /// This is also the key of the first attribute in the parsed object.
    pub fn key_attr(&self) -> &'static str {
        match self {
            IrrObjectType::AutNum => "aut-num",
            IrrObjectType::Route => "route",
            IrrObjectType::Route6 => "route6",
            IrrObjectType::AsSet => "as-set",
            IrrObjectType::RouteSet => "route-set",
            IrrObjectType::Mntner => "mntner",
            IrrObjectType::Organisation => "organisation",
        }
    }

    /// Parse the key attribute name from a parsed RPSL object's first
    /// attribute to determine the object type. Returns `None` for unknown
    /// or unsupported types.
    pub fn from_first_attr(name: &str) -> Option<Self> {
        match name {
            "aut-num" => Some(IrrObjectType::AutNum),
            "route" => Some(IrrObjectType::Route),
            "route6" => Some(IrrObjectType::Route6),
            "as-set" => Some(IrrObjectType::AsSet),
            "route-set" => Some(IrrObjectType::RouteSet),
            "mntner" => Some(IrrObjectType::Mntner),
            "organisation" | "org" => Some(IrrObjectType::Organisation),
            _ => None,
        }
    }

    /// Returns all supported object types.
    pub fn all() -> &'static [IrrObjectType] {
        &[
            IrrObjectType::AutNum,
            IrrObjectType::Route,
            IrrObjectType::Route6,
            IrrObjectType::AsSet,
            IrrObjectType::RouteSet,
            IrrObjectType::Mntner,
            IrrObjectType::Organisation,
        ]
    }
}

// ============================================================================
// Typed RPSL Objects
// ============================================================================

/// An `aut-num` object: AS-level routing registry entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutNum {
    /// The AS number, e.g. `13335`.
    pub asn: u32,
    /// The `as-name` attribute.
    pub as_name: String,
    /// The `descr` attributes (may be multiple).
    pub descr: Vec<String>,
    /// The `source` attribute (registry provenance).
    pub source: String,
    /// All other attributes as (name, value) pairs, preserving order.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// A `route` or `route6` object: prefix-origin registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// The registered prefix.
    pub prefix: IpNet,
    /// The origin AS number.
    pub origin: u32,
    /// The `descr` attributes.
    pub descr: Vec<String>,
    /// The `source` attribute (registry provenance).
    pub source: String,
    /// All other attributes.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// An `as-set` object: named collection of ASes and/or other as-sets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsSet {
    /// The as-set name, e.g. `AS-EXAMPLE`.
    pub name: String,
    /// Direct AS members (numeric).
    pub members: Vec<u32>,
    /// AS-set members (named references, may require recursive resolution).
    pub set_members: Vec<String>,
    /// The `descr` attributes.
    pub descr: Vec<String>,
    /// The `source` attribute.
    pub source: String,
    /// All other attributes.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// A `route-set` object: named collection of prefixes and/or other route-sets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSet {
    /// The route-set name, e.g. `RS-EXAMPLE`.
    pub name: String,
    /// Direct prefix members.
    pub members: Vec<IpNet>,
    /// Route-set members (named references, may require recursive resolution).
    pub set_members: Vec<String>,
    /// The `descr` attributes.
    pub descr: Vec<String>,
    /// The `source` attribute.
    pub source: String,
    /// All other attributes.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// A `mntner` object: database maintainer with authentication info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mntner {
    /// The maintainer name, e.g. `MAINT-AS13335`.
    pub name: String,
    /// Authentication methods declared (e.g. `MD5-PW`, `PGPKEY-...`).
    /// Values are kept verbatim but passwords are always stripped by IRR dumps.
    pub auth: Vec<String>,
    /// The `upd-to` notification email.
    pub upd_to: Vec<String>,
    /// The `mnt-nfy` notification email.
    pub mnt_nfy: Vec<String>,
    /// The `source` attribute.
    pub source: String,
    /// All other attributes.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// An `organisation` (or `org`) object: entity details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organisation {
    /// The organisation ID, e.g. `ORG-GCI2-RIPE`.
    pub id: String,
    /// The organisation name.
    pub name: String,
    /// The organisation type (RIPE-specific, e.g. `LIR`, `RIR`).
    pub org_type: Option<String>,
    /// Address lines.
    pub address: Vec<String>,
    /// Country code.
    pub country: Option<String>,
    /// Abuse contact email.
    pub abuse_c: Option<String>,
    /// The `source` attribute.
    pub source: String,
    /// All other attributes.
    pub extra: BTreeMap<String, Vec<String>>,
}

/// A typed IRR object, tagged by its type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrrObject {
    AutNum(AutNum),
    Route(Route),
    Route6(Route),
    AsSet(AsSet),
    RouteSet(RouteSet),
    Mntner(Mntner),
    Organisation(Organisation),
}

impl IrrObject {
    /// The `source:` attribute value (registry provenance).
    pub fn source(&self) -> &str {
        match self {
            IrrObject::AutNum(o) => &o.source,
            IrrObject::Route(o) | IrrObject::Route6(o) => &o.source,
            IrrObject::AsSet(o) => &o.source,
            IrrObject::RouteSet(o) => &o.source,
            IrrObject::Mntner(o) => &o.source,
            IrrObject::Organisation(o) => &o.source,
        }
    }
}
