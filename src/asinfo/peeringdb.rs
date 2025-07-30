use crate::{BgpkitCommonsError, Result};
use peeringdb_rs::{PeeringdbNet, load_peeringdb_net};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeeringdbData {
    pub asn: u32,
    pub name: Option<String>,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub irr_as_set: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peeringdb {
    peeringdb_map: HashMap<u32, PeeringdbData>,
}

impl Peeringdb {
    pub fn new() -> Result<Self> {
        let mut peeringdb_map = HashMap::new();
        let net_vec: Vec<PeeringdbNet> = load_peeringdb_net().map_err(|e| {
            BgpkitCommonsError::data_source_error(
                crate::errors::data_sources::PEERINGDB,
                e.to_string(),
            )
        })?;
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
            };
        }

        Ok(Self { peeringdb_map })
    }

    pub fn get_data(&self, asn: u32) -> Option<&PeeringdbData> {
        self.peeringdb_map.get(&asn)
    }
}
