//! asinfo is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asinfo: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - (Optional) CAIDA as-to-organization mapping: <https://www.caida.org/catalog/datasets/as-organizations/>
//! - (Optional) APNIC AS population data: <https://stats.labs.apnic.net/cgi-bin/aspop>
//! - (Optional) IIJ IHR Hegemony data: <https://ihr-archive.iijlab.net/>
//!
//! # Data structure
//!
//! ```rust,no_run
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone)]
//! pub struct AsInfo {
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
//! Call with `BgpkitCommons` instance:
//!
//! ```rust,no_run
//! use bgpkit_commons::BgpkitCommons;
//!
//! let mut bgpkit = BgpkitCommons::new();
//! bgpkit.load_asinfo(false, false, false).unwrap();
//! let asinfo = bgpkit.asinfo_get(3333).unwrap().unwrap();
//! assert_eq!(asinfo.name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! ```
//!
//! Directly call the module:
//!
//! ```rust,no_run
//! use std::collections::HashMap;
//! use bgpkit_commons::asinfo::{AsInfo, get_asinfo_map};
//!
//! let asinfo: HashMap<u32, AsInfo> = get_asinfo_map(false, false, false).unwrap();
//! assert_eq!(asinfo.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asinfo.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asinfo.get(&400644).unwrap().country, "US");
//! ```

mod hegemony;
mod population;

pub use crate::asinfo::hegemony::HegemonyData;
pub use crate::asinfo::population::AsnPopulationData;
use crate::BgpkitCommons;
use anyhow::{anyhow, Result};
use oneio::OneIoError;
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

const RIPE_RIS_ASN_TXT_URL: &str = "https://ftp.ripe.net/ripe/asnames/asn.txt";
const BGPKIT_ASN_TXT_MIRROR_URL: &str = "https://data.bgpkit.com/commons/asn.txt";

pub struct AsInfoUtils {
    pub asinfo_map: HashMap<u32, AsInfo>,
    pub load_as2org: bool,
    pub load_population: bool,
    pub load_hegemony: bool,
}

impl AsInfoUtils {
    pub fn new(load_as2org: bool, load_population: bool, load_hegemony: bool) -> Result<Self> {
        let asinfo_map = get_asinfo_map(load_as2org, load_population, load_hegemony)?;
        Ok(AsInfoUtils {
            asinfo_map,
            load_as2org,
            load_population,
            load_hegemony,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.asinfo_map =
            get_asinfo_map(self.load_as2org, self.load_population, self.load_hegemony)?;
        Ok(())
    }

    pub fn get(&self, asn: u32) -> Option<&AsInfo> {
        self.asinfo_map.get(&asn)
    }
}

pub fn get_asinfo_map(
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
) -> Result<HashMap<u32, AsInfo>> {
    info!("loading asinfo from RIPE NCC...");
    let text = match oneio::read_to_string(BGPKIT_ASN_TXT_MIRROR_URL) {
        Ok(t) => t,
        Err(_) => match oneio::read_to_string(RIPE_RIS_ASN_TXT_URL) {
            Ok(t) => t,
            Err(e) => {
                return Err(anyhow!(
                    "error reading asinfo (neither mirror or original works): {}",
                    e
                ));
            }
        },
    };

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

impl BgpkitCommons {
    pub fn asinfo_get(&self, asn: u32) -> Result<Option<AsInfo>> {
        if self.asinfo.is_none() {
            return Err(anyhow!("asinfo is not loaded"));
        }

        Ok(self.asinfo.as_ref().unwrap().get(asn).cloned())
    }

    pub fn asinfo_are_siblings(&self, asn1: u32, asn2: u32) -> Result<bool> {
        if self.asinfo.is_none() {
            return Err(anyhow!("asinfo is not loaded"));
        }
        if !self.asinfo.as_ref().unwrap().load_as2org {
            return Err(anyhow!("asinfo is not loaded with as2org data"));
        }

        let info_1_opt = self.asinfo_get(asn1)?;
        let info_2_opt = self.asinfo_get(asn2)?;
        if info_1_opt.is_some() && info_2_opt.is_some() {
            let org_1_opt = info_1_opt.unwrap().as2org;
            let org_2_opt = info_2_opt.unwrap().as2org;
            if org_1_opt.is_some() && org_2_opt.is_some() {
                return Ok(org_1_opt.unwrap().org_id == org_2_opt.unwrap().org_id);
            }
        }
        Ok(false)
    }
}
