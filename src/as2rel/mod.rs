//! AS-level relationship generated by BGPKIT
//!
//! Raw data files available at: <https://data.bgpkit.com/as2rel/>
//! * [as2rel-latest.json.bz2](https://data.bgpkit.com/as2rel/as2rel-latest.json.bz2): latest combined
//! * [as2rel-v4-latest.json.bz2](https://data.bgpkit.com/as2rel/as2rel-v4-latest.json.bz2): latest IPv4 relationship
//! * [as2rel-v6-latest.json.bz2](https://data.bgpkit.com/as2rel/as2rel-v6-latest.json.bz2): latest IPv6 relationship

use crate::BgpkitCommons;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use tracing::info;

#[allow(dead_code)]
const AS2REL_LATEST_COMBINED: &str = "https://data.bgpkit.com/as2rel/as2rel-latest.json.bz2";

const AS2REL_LATEST_V4: &str = "https://data.bgpkit.com/as2rel/as2rel-v4-latest.json.bz2";
const AS2REL_LATEST_V6: &str = "https://data.bgpkit.com/as2rel/as2rel-v6-latest.json.bz2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsRelationship {
    ProviderCustomer,
    CustomerProvider,
    PeerPeer,
}

impl Serialize for AsRelationship {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AsRelationship::ProviderCustomer => serializer.serialize_str("pc"),
            AsRelationship::CustomerProvider => serializer.serialize_str("cp"),
            AsRelationship::PeerPeer => serializer.serialize_str("pp"),
        }
    }
}

impl<'de> Deserialize<'de> for AsRelationship {
    fn deserialize<D>(deserializer: D) -> Result<AsRelationship, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = i8::deserialize(deserializer)?;
        match s {
            -1 | 1 => Ok(AsRelationship::ProviderCustomer),
            0 => Ok(AsRelationship::PeerPeer),
            _ => Err(serde::de::Error::custom("invalid relationship")),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct As2relEntry {
    asn1: u32,
    asn2: u32,
    paths_count: u32,
    peers_count: u32,
    rel: AsRelationship,
}

impl PartialEq for As2relEntry {
    fn eq(&self, other: &Self) -> bool {
        self.asn1 == other.asn1 && self.asn2 == other.asn2 && self.rel == other.rel
    }
}
impl Eq for As2relEntry {}

impl Hash for As2relEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.asn1.hash(state);
        self.asn2.hash(state);
        self.rel.hash(state);
    }
}

impl As2relEntry {
    fn reverse(&self) -> Self {
        Self {
            asn1: self.asn2,
            asn2: self.asn1,
            paths_count: self.paths_count,
            peers_count: self.peers_count,
            rel: match self.rel {
                AsRelationship::ProviderCustomer => AsRelationship::CustomerProvider,
                AsRelationship::CustomerProvider => AsRelationship::ProviderCustomer,
                AsRelationship::PeerPeer => AsRelationship::PeerPeer,
            },
        }
    }
}

pub struct As2relBgpkit {
    v4_rels_map: HashMap<(u32, u32), HashSet<As2relEntry>>,
    v6_rels_map: HashMap<(u32, u32), HashSet<As2relEntry>>,
    v4_max_peer_count: u32,
    v6_max_peer_count: u32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct As2relBgpkitData {
    pub rel: AsRelationship,
    pub peers_count: u32,
    pub max_peer_count: u32,
}

impl As2relBgpkit {
    pub fn new() -> Result<Self> {
        let v4_rels = parse_as2rel_data(AS2REL_LATEST_V4)?;
        let v6_rels = parse_as2rel_data(AS2REL_LATEST_V6)?;
        let mut v4_rels_map = HashMap::new();
        let mut v6_rels_map = HashMap::new();
        let mut v4_max_peer_count = 0;
        let mut v6_max_peer_count = 0;
        for entry in v4_rels {
            v4_rels_map
                .entry((entry.asn1, entry.asn2))
                .or_insert_with(HashSet::new)
                .insert(entry);
            v4_rels_map
                .entry((entry.asn2, entry.asn1))
                .or_insert_with(HashSet::new)
                .insert(entry.reverse());

            v4_max_peer_count = v4_max_peer_count.max(entry.peers_count);
        }
        for entry in v6_rels {
            v6_rels_map
                .entry((entry.asn1, entry.asn2))
                .or_insert_with(HashSet::new)
                .insert(entry);
            v6_rels_map
                .entry((entry.asn2, entry.asn1))
                .or_insert_with(HashSet::new)
                .insert(entry.reverse());

            v6_max_peer_count = v6_max_peer_count.max(entry.peers_count);
        }
        Ok(Self {
            v4_rels_map,
            v6_rels_map,
            v4_max_peer_count,
            v6_max_peer_count,
        })
    }

    pub fn lookup_pair(
        &self,
        asn1: u32,
        asn2: u32,
    ) -> (Vec<As2relBgpkitData>, Vec<As2relBgpkitData>) {
        let v4_entry_set = self.v4_rels_map.get(&(asn1, asn2));
        let v6_entry_set = self.v6_rels_map.get(&(asn1, asn2));

        let v4_entries = v4_entry_set
            .map(|set| {
                set.iter()
                    .map(|entry| As2relBgpkitData {
                        rel: entry.rel,
                        peers_count: entry.peers_count,
                        max_peer_count: self.v4_max_peer_count,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let v6_entries = v6_entry_set
            .map(|set| {
                set.iter()
                    .map(|entry| As2relBgpkitData {
                        rel: entry.rel,
                        peers_count: entry.peers_count,
                        max_peer_count: self.v6_max_peer_count,
                    })
                    .collect()
            })
            .unwrap_or_default();

        (v4_entries, v6_entries)
    }
}

fn parse_as2rel_data(url: &str) -> Result<Vec<As2relEntry>> {
    info!("loading AS2REL data from {}", url);
    let data: Vec<As2relEntry> = oneio::read_json_struct(url)?;
    Ok(data)
}

impl BgpkitCommons {
    pub fn as2rel_lookup(
        &self,
        asn1: u32,
        asn2: u32,
    ) -> Result<(Vec<As2relBgpkitData>, Vec<As2relBgpkitData>)> {
        if self.as2rel.is_none() {
            return Err(anyhow!("as2rel is not loaded"));
        }

        Ok(self.as2rel.as_ref().unwrap().lookup_pair(asn1, asn2))
    }
}
