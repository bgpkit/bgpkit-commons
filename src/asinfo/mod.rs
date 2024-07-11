//! asinfo is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asinfo: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - CAIDA as-to-organization mapping: <https://www.caida.org/catalog/datasets/as-organizations/>
//!
//! # Data structure
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone)]
//! pub struct AsName {
//!     pub asn: u32,
//!     pub name: String,
//!     pub country: String,
//!     pub as2org: Option<As2orgInfo>,
//!     pub population: Option<AsnPopulationData>,
//! }
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct As2orgInfo {
//!     pub name: String,
//!     pub country: String,
//!     pub org_id: String,
//!     pub org_name: String,
//! }
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct AsnPopulationData {
//!     pub user_count: i64,
//!     pub percent_country: f64,
//!     pub percent_global: f64,
//!     pub sample_count: i64,
//! }
//! ```
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use bgpkit_commons::asinfo::{AsInfo, get_asnames};
//!
//! let asinfo: HashMap<u32, AsInfo> = get_asnames().unwrap();
//! assert_eq!(asinfo.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asinfo.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asinfo.get(&400644).unwrap().country, "US");
//! ```

mod hegemony;
mod population;

use crate::asinfo::hegemony::HegemonyData;
use crate::asinfo::population::AsnPopulationData;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsInfo {
    pub asn: u32,
    pub name: String,
    pub country: String,
    pub as2org: Option<As2orgInfo>,
    pub population: Option<AsnPopulationData>,
    pub hegemony: Option<HegemonyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct As2orgInfo {
    pub name: String,
    pub country: String,
    pub org_id: String,
    pub org_name: String,
}

const DATA_URL: &str = "https://ftp.ripe.net/ripe/asnames/asn.txt";

pub struct AsInfoUtils {
    pub asinfo_map: HashMap<u32, AsInfo>,
    pub load_as2org: bool,
    pub load_population: bool,
    pub load_hegemony: bool,
}

impl AsInfoUtils {
    pub fn new(load_as2org: bool, load_population: bool, load_hegemony: bool) -> Result<Self> {
        let asinfo_map = get_asnames(load_as2org, load_population, load_hegemony)?;
        Ok(AsInfoUtils {
            asinfo_map,
            load_as2org,
            load_population,
            load_hegemony,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.asinfo_map = get_asnames(self.load_as2org, self.load_population, self.load_hegemony)?;
        Ok(())
    }

    pub fn get(&self, asn: u32) -> Option<&AsInfo> {
        self.asinfo_map.get(&asn)
    }
}

pub fn get_asnames(
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
) -> Result<HashMap<u32, AsInfo>> {
    info!("loading asinfo from RIPE NCC...");
    let text = oneio::read_to_string(DATA_URL)?;
    let as2org_utils = if load_as2org {
        info!("loading as2org data from CAIDA...");
        Some(as2org_rs::As2org::new(None)?)
    } else {
        None
    };
    let population_utils = if load_population {
        info!("loading ASN population data from APNIC...");
        Some(population::AsnPopulation::new()?)
    } else {
        None
    };
    let hegemony_utils = if load_hegemony {
        info!("loading IIJ IHR hegemony score data from BGPKIT mirror...");
        Some(hegemony::Hegemony::new()?)
    } else {
        None
    };

    let asnames = text
        .lines()
        .filter_map(|line| {
            let (asn_str, name_country_str) = match line.split_once(' ') {
                Some((asn, name)) => (asn, name),
                None => return None,
            };
            let (name_str, country_str) = match name_country_str.rsplit_once(", ") {
                Some((name, country)) => (name, country),
                None => return None,
            };
            let asn = asn_str.parse::<u32>().unwrap();
            let as2org = as2org_utils.as_ref().and_then(|as2org_data| {
                as2org_data.get_as_info(asn).map(|info| As2orgInfo {
                    name: info.name.clone(),
                    country: info.country_code.clone(),
                    org_id: info.org_id.clone(),
                    org_name: info.org_name.clone(),
                })
            });
            let population = population_utils.as_ref().and_then(|p| p.get(asn));
            let hegemony = hegemony_utils
                .as_ref()
                .and_then(|h| h.get_score(asn).cloned());
            Some(AsInfo {
                asn,
                name: name_str.to_string(),
                country: country_str.to_string(),
                as2org,
                population,
                hegemony,
            })
        })
        .collect::<Vec<AsInfo>>();

    let mut asnames_map = HashMap::new();
    for asname in asnames {
        asnames_map.insert(asname.asn, asname);
    }
    Ok(asnames_map)
}
