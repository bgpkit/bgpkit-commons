//! asinfo is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asinfo: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - RIR delegated stats (fallback fill for ASNs missing from `asn.txt`):
//!   <https://www.nro.net/about/rirs/statistics/>
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
//! ```
//!
//! The `peeringdb` field of `AsInfo` uses [`crate::peeringdb::Network`], which
//! mirrors the full PeeringDB `/net` API record.
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

mod as2org;
mod hegemony;
mod population;
mod sibling_orgs;

use crate::errors::{data_sources, load_methods, modules};
use crate::peeringdb::{Network, Peeringdb};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
use serde::{Deserialize, Serialize};
use sibling_orgs::SiblingOrgsUtils;
use std::collections::HashMap;
use std::io::{BufRead, Read};
use tracing::{info, warn};

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
    pub peeringdb: Option<Network>,
}

impl AsInfo {
    /// Returns the preferred name for the AS.
    ///
    /// The order of preference is:
    /// 1. `peeringdb.name` if available
    /// 2. `as2org.org_name` if available and not empty
    /// 3. The default `name` field
    ///
    /// This method does not perform any network access.
    pub fn get_preferred_name(&self) -> String {
        if let Some(peeringdb_data) = &self.peeringdb {
            if let Some(name) = &peeringdb_data.name {
                if !name.is_empty() {
                    return name.clone();
                }
            }
        }
        if let Some(as2org_info) = &self.as2org {
            if !as2org_info.org_name.is_empty() {
                return as2org_info.org_name.clone();
            }
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

/// RIR delegated stats files (NRO format), used to fill ASNs missing from
/// RIPE NCC `asn.txt`. These files are authoritative allocation records,
/// updated daily, and include newly-allocated ASNs that `asn.txt` lags on
/// by days to weeks.
const RIR_DELEGATED_STATS_URLS: &[&str] = &[
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-latest",
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-latest",
];

/// Builder for configuring which data sources to load for AS information.
///
/// # Example
///
/// ```rust,no_run
/// use bgpkit_commons::asinfo::AsInfoBuilder;
///
/// let asinfo = AsInfoBuilder::new()
///     .with_as2org()
///     .with_peeringdb()
///     .build()
///     .unwrap();
/// ```
#[derive(Default)]
pub struct AsInfoBuilder {
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
    load_peeringdb: bool,
}

impl AsInfoBuilder {
    /// Create a new builder with all data sources disabled by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable loading CAIDA AS-to-Organization mapping data.
    pub fn with_as2org(mut self) -> Self {
        self.load_as2org = true;
        self
    }

    /// Enable loading APNIC AS population data.
    pub fn with_population(mut self) -> Self {
        self.load_population = true;
        self
    }

    /// Enable loading IIJ IHR hegemony score data.
    pub fn with_hegemony(mut self) -> Self {
        self.load_hegemony = true;
        self
    }

    /// Enable loading PeeringDB data.
    pub fn with_peeringdb(mut self) -> Self {
        self.load_peeringdb = true;
        self
    }

    /// Enable all optional data sources.
    pub fn with_all(mut self) -> Self {
        self.load_as2org = true;
        self.load_population = true;
        self.load_hegemony = true;
        self.load_peeringdb = true;
        self
    }

    /// Build the AsInfoUtils with the configured data sources.
    pub fn build(self) -> Result<AsInfoUtils> {
        AsInfoUtils::new(
            self.load_as2org,
            self.load_population,
            self.load_hegemony,
            self.load_peeringdb,
        )
    }
}

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
    let reader = oneio::get_reader(BGPKIT_ASNINFO_URL)?;
    for line in std::io::BufReader::new(reader).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let asinfo: AsInfo = serde_json::from_str(&line)?;
        asnames_map.insert(asinfo.asn, asinfo);
    }
    Ok(asnames_map)
}

/// Parse RIR delegated stats text (NRO format) into an ASN -> country code map.
///
/// Record format: `registry|CC|type|start|value|date|status[|extensions]`.
/// Only `asn` records with a real (non-empty, non-`*`) country code are kept;
/// private-use ASN ranges (RFC 6996: 64512-65534 and 4200000000+) are excluded.
/// Ranges are expanded per-ASN (`value` is a count).
fn parse_delegated_stats(text: &str, map: &mut HashMap<u32, String>) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 7 || parts[2] != "asn" {
            continue;
        }
        let cc = parts[1].trim();
        if cc.is_empty() || cc == "*" {
            continue;
        }
        let (Ok(start), Ok(count)) = (parts[3].parse::<u64>(), parts[4].parse::<u64>()) else {
            continue;
        };
        for asn in start..start.saturating_add(count) {
            if asn > u32::MAX as u64 {
                break;
            }
            let asn = asn as u32;
            if (64512..=65534).contains(&asn) || asn >= 4_200_000_000 {
                continue;
            }
            map.insert(asn, cc.to_uppercase());
        }
    }
}

/// Fill ASNs missing from the RIPE NCC `asn.txt`-derived map using the five
/// RIR delegated stats files (authoritative allocation data, updated daily).
///
/// `asn.txt` lags behind new allocations by days to weeks; the delegated
/// stats cover them. Synthesized entries carry the allocated country code and
/// an `"UNKNOWN"` name (delegated stats do not include names), with all
/// optional enrichment fields left as `None`.
///
/// Best-effort: failures fetching individual files are logged and skipped.
fn fill_missing_asns_from_delegated_stats(asnames_map: &mut HashMap<u32, AsInfo>) {
    let read_text = |url: &str| -> Result<String> {
        let mut text = String::new();
        oneio::get_reader(url)?.read_to_string(&mut text)?;
        Ok(text)
    };

    let mut delegated: HashMap<u32, String> = HashMap::new();
    for url in RIR_DELEGATED_STATS_URLS {
        match read_text(url) {
            Ok(text) => parse_delegated_stats(&text, &mut delegated),
            Err(e) => warn!("failed to load delegated stats from {}: {}", url, e),
        }
    }

    let mut filled = 0usize;
    for (asn, country) in delegated {
        asnames_map.entry(asn).or_insert_with(|| {
            filled += 1;
            AsInfo {
                asn,
                name: "UNKNOWN".to_string(),
                country,
                as2org: None,
                population: None,
                hegemony: None,
                peeringdb: None,
            }
        });
    }
    info!(
        "filled {} ASNs missing from RIPE NCC asn.txt via RIR delegated stats",
        filled
    );
}

pub fn get_asinfo_map(
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
    load_peeringdb: bool,
) -> Result<HashMap<u32, AsInfo>> {
    info!("loading asinfo from RIPE NCC...");
    let read_text = |url: &str| -> Result<String> {
        let mut text = String::new();
        oneio::get_reader(url)?.read_to_string(&mut text)?;
        Ok(text)
    };
    let text = match read_text(BGPKIT_ASN_TXT_MIRROR_URL) {
        Ok(t) => t,
        Err(_) => match read_text(RIPE_RIS_ASN_TXT_URL) {
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
        Some(as2org::As2org::new(None)?)
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
        Some(Peeringdb::new_networks_only()?)
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
                .and_then(|h| h.get_network(asn).cloned());
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

    info!("filling ASNs missing from RIPE NCC asn.txt via RIR delegated stats...");
    fill_missing_asns_from_delegated_stats(&mut asnames_map);

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

        if let (Some(info1), Some(info2)) = (info_1_opt, info_2_opt) {
            if let (Some(org1), Some(org2)) = (info1.as2org, info2.as2org) {
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
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delegated_stats_basic() {
        let text = "\
2|ripencc|ZZ|209|20250704|00000000+000000+00000000|UTF-8
ripencc|*|asn|*|39634|summary
ripencc|GB|asn|219157|1|20260722|allocated
ripencc|DE|asn|219125|1|20260728|allocated
arin||asn|212|1||reserved|
arin|*|asn|*|32843|summary
arin|US|asn|402598|1|20260604|assigned|
apnic|BD|asn|154708|1|20260609|allocated
ripencc|NL|asn|1000|4|19970901|allocated
ripencc|NL|ipv4|185.0.0.0|65536|20000101|allocated
";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert_eq!(map.get(&219157).map(String::as_str), Some("GB"));
        assert_eq!(map.get(&219125).map(String::as_str), Some("DE"));
        assert_eq!(map.get(&402598).map(String::as_str), Some("US"));
        assert_eq!(map.get(&154708).map(String::as_str), Some("BD"));
        // range expansion: AS1000..=AS1003 (value is a count of 4)
        assert_eq!(map.get(&1000).map(String::as_str), Some("NL"));
        assert_eq!(map.get(&1003).map(String::as_str), Some("NL"));
        assert!(!map.contains_key(&1004));
        // reserved entries with empty CC are skipped
        assert!(!map.contains_key(&212));
        // non-asn records are skipped
        assert_eq!(map.len(), 8);
    }

    #[test]
    fn test_parse_delegated_stats_skips_private_and_invalid() {
        let text = "\
arin|US|asn|64512|1023|19891201|reserved
arin|US|asn|4200000000|9999|19891201|reserved
arin|US|asn|notanumber|1|20200101|allocated
arin|US|asn|123|notacount|20200101|allocated
";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_delegated_stats_private_boundary() {
        // AS65535 (last private 16-bit ASN, not in 64512..=65534) is kept;
        // AS65534 is dropped. RFC 6996 documentation ASN 64496 is public but
        // unused; it is kept since only the exact private ranges are filtered.
        let text = "\
arin|US|asn|65535|1|19891201|allocated
arin|US|asn|65534|1|19891201|allocated
arin|US|asn|64496|1|19891201|allocated
arin|US|asn|4199999999|1|19891201|allocated
arin|US|asn|4200000000|1|19891201|allocated
";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert_eq!(map.get(&65535).map(String::as_str), Some("US"));
        assert!(!map.contains_key(&65534));
        assert_eq!(map.get(&64496).map(String::as_str), Some("US"));
        assert_eq!(map.get(&4199999999).map(String::as_str), Some("US"));
        assert!(!map.contains_key(&4200000000));
    }

    #[test]
    fn test_parse_delegated_stats_case_normalization() {
        let text = "lacnic|br|asn|269000|1|20150101|allocated\n";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert_eq!(map.get(&269000).map(String::as_str), Some("BR"));
    }

    #[test]
    fn test_parse_delegated_stats_malformed_lines() {
        let text = "\
# comment line

ripencc|GB|asn
ripencc|GB|ipv6|2001:db8::|32|20200101|allocated
some garbage line with no pipes at all
|GB|asn|100|1|20200101|allocated
ripencc|GB|asn|100|1|20200101
ripencc|GB|asn|100|1|20200101|allocated|extra|fields|ok
";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        // empty registry is kept (only CC matters), short lines dropped,
        // extended lines with >7 fields still parsed
        assert_eq!(map.get(&100).map(String::as_str), Some("GB"));
        assert_eq!(map.len(), 1);
    }
}
