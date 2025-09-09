//! asinfo is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asinfo: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - (Optional) CAIDA as-to-organization mapping: <https://www.caida.org/catalog/datasets/as-organizations/>
//! - (Optional) APNIC AS population data: <https://stats.labs.apnic.net/cgi-bin/aspop>
//! - (Optional) IIJ IHR Hegemony data: <https://ihr-archive.iijlab.net/>
//! - (Optional) PeeringDB data: <https://www.peeringdb.com>
//!
//! # Data structure
//!
//! ```rust,no_run
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct AsInfo {
//!     pub asn: u32,
//!     pub name: String,
//!     pub country: String,
//!     pub as2org: Option<As2orgInfo>,
//!     pub population: Option<AsnPopulationData>,
//!     pub hegemony: Option<HegemonyData>,
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
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct HegemonyData {
//!     pub asn: u32,
//!     pub ipv4: f64,
//!     pub ipv6: f64,
//! }
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct PeeringdbData {
//!     pub asn: u32,
//!     pub name: Option<String>,
//!     pub name_long: Option<String>,
//!     pub aka: Option<String>,
//!     pub irr_as_set: Option<String>,
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
//! bgpkit.load_asinfo(false, false, false, false).unwrap();
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
//! let asinfo: HashMap<u32, AsInfo> = get_asinfo_map(false, false, false, false).unwrap();
//! assert_eq!(asinfo.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asinfo.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asinfo.get(&400644).unwrap().country, "US");
//! ```
//!
//! Retrieve all previously generated and cached AS information:
//! ```rust,no_run
//! use std::collections::HashMap;
//! use bgpkit_commons::asinfo::{get_asinfo_map_cached, AsInfo};
//! let asinfo: HashMap<u32, AsInfo> = get_asinfo_map_cached().unwrap();
//! assert_eq!(asinfo.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asinfo.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asinfo.get(&400644).unwrap().country, "US");
//! ```
//!
//! Or with `BgpkitCommons` instance:
//! ```rust,no_run
//!
//! use std::collections::HashMap;
//! use bgpkit_commons::asinfo::AsInfo;
//! use bgpkit_commons::BgpkitCommons;
//!
//! let mut commons = BgpkitCommons::new();
//! commons.load_asinfo_cached().unwrap();
//! let asinfo: HashMap<u32, AsInfo> = commons.asinfo_all().unwrap();
//! assert_eq!(asinfo.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asinfo.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asinfo.get(&400644).unwrap().country, "US");
//! ```
//!
//! Check if two ASNs are siblings:
//!
//! ```rust,no_run
//! use bgpkit_commons::BgpkitCommons;
//!
//! let mut bgpkit = BgpkitCommons::new();
//! bgpkit.load_asinfo(true, false, false, false).unwrap();
//! let are_siblings = bgpkit.asinfo_are_siblings(3333, 3334).unwrap();
//! ```

mod hegemony;
mod peeringdb;
mod population;
mod sibling_orgs;

use crate::errors::{data_sources, load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
use peeringdb::PeeringdbData;
use serde::{Deserialize, Serialize};
use sibling_orgs::SiblingOrgsUtils;
use std::collections::HashMap;
use tracing::info;

pub use hegemony::HegemonyData;
pub use population::AsnPopulationData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsInfo {
    pub asn: u32,
    pub name: String,
    pub country: String,
    pub as2org: Option<As2orgInfo>,
    pub population: Option<AsnPopulationData>,
    pub hegemony: Option<HegemonyData>,
    pub peeringdb: Option<PeeringdbData>,
}

impl AsInfo {
    /// Returns the preferred name for the AS.
    ///
    /// The order of preference is:
    /// 1. `peeringdb.name` if available
    /// 2. `as2org.org_name` if available and not empty
    /// 3. The default `name` field
    /// ```
    pub fn get_preferred_name(&self) -> String {
        if let Some(peeringdb_data) = &self.peeringdb
            && let Some(name) = &peeringdb_data.name {
                return name.clone();
            }
        if let Some(as2org_info) = &self.as2org
            && !as2org_info.org_name.is_empty() {
                return as2org_info.org_name.clone();
            }
        self.name.clone()
    }
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
const BGPKIT_ASNINFO_URL: &str = "https://data.bgpkit.com/commons/asinfo.jsonl";

pub struct AsInfoUtils {
    pub asinfo_map: HashMap<u32, AsInfo>,
    pub sibling_orgs: Option<SiblingOrgsUtils>,
    pub load_as2org: bool,
    pub load_population: bool,
    pub load_hegemony: bool,
    pub load_peeringdb: bool,
}

impl AsInfoUtils {
    pub fn new(
        load_as2org: bool,
        load_population: bool,
        load_hegemony: bool,
        load_peeringdb: bool,
    ) -> Result<Self> {
        let asinfo_map =
            get_asinfo_map(load_as2org, load_population, load_hegemony, load_peeringdb)?;
        let sibling_orgs = if load_as2org {
            Some(SiblingOrgsUtils::new()?)
        } else {
            None
        };
        Ok(AsInfoUtils {
            asinfo_map,
            sibling_orgs,
            load_as2org,
            load_population,
            load_hegemony,
            load_peeringdb,
        })
    }

    pub fn new_from_cached() -> Result<Self> {
        let asinfo_map = get_asinfo_map_cached()?;
        let sibling_orgs = Some(SiblingOrgsUtils::new()?);
        Ok(AsInfoUtils {
            asinfo_map,
            sibling_orgs,
            load_as2org: true,
            load_population: true,
            load_hegemony: true,
            load_peeringdb: true,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.asinfo_map = get_asinfo_map(
            self.load_as2org,
            self.load_population,
            self.load_hegemony,
            self.load_peeringdb,
        )?;
        Ok(())
    }

    pub fn get(&self, asn: u32) -> Option<&AsInfo> {
        self.asinfo_map.get(&asn)
    }
}

impl LazyLoadable for AsInfoUtils {
    fn reload(&mut self) -> Result<()> {
        self.reload()
    }

    fn is_loaded(&self) -> bool {
        !self.asinfo_map.is_empty()
    }

    fn loading_status(&self) -> &'static str {
        if self.is_loaded() {
            "ASInfo data loaded"
        } else {
            "ASInfo data not loaded"
        }
    }
}

pub fn get_asinfo_map_cached() -> Result<HashMap<u32, AsInfo>> {
    info!("loading asinfo from previously generated BGPKIT cache file...");
    let mut asnames_map = HashMap::new();
    for line in oneio::read_lines(BGPKIT_ASNINFO_URL)? {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let asinfo: AsInfo = serde_json::from_str(&line)?;
        asnames_map.insert(asinfo.asn, asinfo);
    }
    Ok(asnames_map)
}

pub fn get_asinfo_map(
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
    load_peeringdb: bool,
) -> Result<HashMap<u32, AsInfo>> {
    info!("loading asinfo from RIPE NCC...");
    let text = match oneio::read_to_string(BGPKIT_ASN_TXT_MIRROR_URL) {
        Ok(t) => t,
        Err(_) => match oneio::read_to_string(RIPE_RIS_ASN_TXT_URL) {
            Ok(t) => t,
            Err(e) => {
                return Err(BgpkitCommonsError::data_source_error(
                    data_sources::BGPKIT,
                    format!(
                        "error reading asinfo (neither mirror or original works): {}",
                        e
                    ),
                ));
            }
        },
    };

    let as2org_utils = if load_as2org {
        info!("loading as2org data from CAIDA...");
        Some(as2org_rs::As2org::new(None).map_err(|e| {
            BgpkitCommonsError::data_source_error(data_sources::CAIDA, e.to_string())
        })?)
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
    let peeringdb_utils = if load_peeringdb {
        info!("loading peeringdb data...");
        Some(peeringdb::Peeringdb::new()?)
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
            let peeringdb = peeringdb_utils
                .as_ref()
                .and_then(|h| h.get_data(asn).cloned());
            Some(AsInfo {
                asn,
                name: name_str.to_string(),
                country: country_str.to_string(),
                as2org,
                population,
                hegemony,
                peeringdb,
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
    /// Returns a HashMap containing all AS information.
    ///
    /// # Returns
    ///
    /// - `Ok(HashMap<u32, AsInfo>)`: A HashMap where the key is the ASN and the value is the corresponding AsInfo.
    /// - `Err`: If the asinfo is not loaded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bgpkit_commons::BgpkitCommons;
    ///
    /// let mut bgpkit = BgpkitCommons::new();
    /// bgpkit.load_asinfo(false, false, false, false).unwrap();
    /// let all_asinfo = bgpkit.asinfo_all().unwrap();
    /// ```
    pub fn asinfo_all(&self) -> Result<HashMap<u32, AsInfo>> {
        if self.asinfo.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::ASINFO,
                load_methods::LOAD_ASINFO,
            ));
        }

        Ok(self.asinfo.as_ref().unwrap().asinfo_map.clone())
    }

    /// Retrieves AS information for a specific ASN.
    ///
    /// # Arguments
    ///
    /// * `asn` - The Autonomous System Number to look up.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(AsInfo))`: The AS information if found.
    /// - `Ok(None)`: If the ASN is not found in the database.
    /// - `Err`: If the asinfo is not loaded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bgpkit_commons::BgpkitCommons;
    ///
    /// let mut bgpkit = BgpkitCommons::new();
    /// bgpkit.load_asinfo(false, false, false, false).unwrap();
    /// let asinfo = bgpkit.asinfo_get(3333).unwrap();
    /// ```
    pub fn asinfo_get(&self, asn: u32) -> Result<Option<AsInfo>> {
        if self.asinfo.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::ASINFO,
                load_methods::LOAD_ASINFO,
            ));
        }

        Ok(self.asinfo.as_ref().unwrap().get(asn).cloned())
    }

    /// Checks if two ASNs are siblings (belong to the same organization).
    ///
    /// # Arguments
    ///
    /// * `asn1` - The first Autonomous System Number.
    /// * `asn2` - The second Autonomous System Number.
    ///
    /// # Returns
    ///
    /// - `Ok(bool)`: True if the ASNs are siblings, false otherwise.
    /// - `Err`: If the asinfo is not loaded or not loaded with as2org data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bgpkit_commons::BgpkitCommons;
    ///
    /// let mut bgpkit = BgpkitCommons::new();
    /// bgpkit.load_asinfo(true, false, false, false).unwrap();
    /// let are_siblings = bgpkit.asinfo_are_siblings(3333, 3334).unwrap();
    /// ```
    ///
    /// # Note
    ///
    /// This function requires the asinfo to be loaded with as2org data.
    pub fn asinfo_are_siblings(&self, asn1: u32, asn2: u32) -> Result<bool> {
        if self.asinfo.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::ASINFO,
                load_methods::LOAD_ASINFO,
            ));
        }
        if !self.asinfo.as_ref().unwrap().load_as2org {
            return Err(BgpkitCommonsError::module_not_configured(
                modules::ASINFO,
                "as2org data",
                "load_asinfo() with as2org=true",
            ));
        }

        let info_1_opt = self.asinfo_get(asn1)?;
        let info_2_opt = self.asinfo_get(asn2)?;

        if let (Some(info1), Some(info2)) = (info_1_opt, info_2_opt)
            && let (Some(org1), Some(org2)) = (info1.as2org, info2.as2org)
        {
            let org_id_1 = org1.org_id;
            let org_id_2 = org2.org_id;

            return Ok(org_id_1 == org_id_2
                || self
                    .asinfo
                    .as_ref()
                    .and_then(|a| a.sibling_orgs.as_ref())
                    .map(|s| s.are_sibling_orgs(org_id_1.as_str(), org_id_2.as_str()))
                    .unwrap_or(false));
        }
        Ok(false)
    }
}
