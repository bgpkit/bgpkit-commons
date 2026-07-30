//! asinfo is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asinfo: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - RIR delegated stats (authoritative allocation records, attached to every ASN):
//!   <https://www.nro.net/about/rirs/statistics/>
//! - IRR `aut-num`, `route`/`route6` objects (per-source arrays for every ASN
//!   with IRR registrations): RIPE, APNIC, ARIN, LACNIC, AFRINIC, NTTCOM, RADB
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
//!     pub irr: Vec<IrrAsnInfo>,
//!     pub delegated: Option<DelegatedInfo>,
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
//! bgpkit.load_asinfo_with_profile(Default::default()).unwrap();
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
//! bgpkit.load_asinfo_with(bgpkit.asinfo_builder().with_as2org()).unwrap();
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

/// RIR delegated-stats data for a single ASN.
///
/// Sourced from the five RIR delegated stats files (NRO format). These are
/// authoritative allocation records, updated daily. For each ASN, the
/// registry that allocated it, the allocation date, status, and country
/// code are recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedInfo {
    /// The RIR that allocated/assigned this ASN (e.g. `"ripencc"`, `"arin"`).
    pub registry: String,
    /// The ISO 3166-1 alpha-2 country code (uppercased).
    pub country: String,
    /// The allocation/assignment date (as-is from the record, format: `YYYYMMDD`).
    pub date: String,
    /// The allocation status (`"allocated"` or `"assigned"`).
    pub status: String,
}

/// IRR data for a single ASN from a single registry source.
///
/// Each entry corresponds to one IRR registry's view of this ASN. An ASN may
/// have entries from multiple registries (e.g. both RIPE and RADB) — the
/// `irr` field on [`AsInfo`] is a `Vec<IrrAsnInfo>` so callers can pick which
/// source(s) to trust.
///
/// Provenance is preserved via `source` (the registry name from the RPSL
/// `source:` attribute). IRR data is self-registered; trust varies by
/// registry authorization model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrAsnInfo {
    /// The `as-name` attribute from the IRR `aut-num` object.
    pub as_name: String,
    /// The `descr` attribute(s), if any.
    pub descr: Vec<String>,
    /// The `source:` attribute — which IRR registry published this object.
    pub source: String,
    /// The `mnt-by` attribute(s) — maintainers controlling this object.
    pub mnt_by: Vec<String>,
    /// Registered IPv4 prefixes from `route` objects with this ASN as origin.
    pub route_prefixes: Vec<String>,
    /// Registered IPv6 prefixes from `route6` objects with this ASN as origin.
    pub route6_prefixes: Vec<String>,
    /// AS-set names that contain this ASN as a direct member.
    pub member_of_sets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsInfo {
    pub asn: u32,
    pub name: String,
    pub country: String,
    pub as2org: Option<As2orgInfo>,
    pub population: Option<AsnPopulationData>,
    pub hegemony: Option<HegemonyData>,
    pub peeringdb: Option<Network>,
    /// RIR delegated-stats allocation data. Present for every allocated ASN.
    pub delegated: Option<DelegatedInfo>,
    /// IRR data per registry source. Empty if the ASN has no IRR registrations.
    /// Multiple sources may have data; callers choose which to trust.
    pub irr: Vec<IrrAsnInfo>,
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

/// Configuration for which IRR sources to fetch.
///
/// By default, `with_irr()` uses the default source set (5 RIRs + NTTCOM + RADB).
/// For finer control, use `with_irr_sources()` to pick specific registries.
///
/// # Example
///
/// ```rust,no_run
/// use bgpkit_commons::asinfo::AsInfoBuilder;
/// use bgpkit_commons::irr::IrrSourceConfig;
///
/// // Only RIPE + RADB
/// let config = IrrSourceConfig::sources(&["RIPE", "RADB"]);
/// let asinfo = AsInfoBuilder::new()
///     .with_irr_sources(config)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct IrrSourceConfig {
    /// Registry names to fetch (e.g. `["RIPE", "RADB"]`).
    /// If empty, uses all default sources.
    pub sources: Vec<String>,
}

impl IrrSourceConfig {
    /// Create a config that fetches only the named sources.
    pub fn sources(names: &[&str]) -> Self {
        Self {
            sources: names.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a config that fetches all default sources.
    pub fn all_defaults() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Resolve to the actual list of `IrrSource` structs to fetch.
    fn resolve(&self) -> Vec<crate::irr::IrrSource> {
        if self.sources.is_empty() {
            crate::irr::default_sources()
        } else {
            crate::irr::all_sources()
                .into_iter()
                .filter(|s| self.sources.iter().any(|n| n == s.name))
                .collect()
        }
    }
}

/// Loading profile for AS information data sources.
///
/// Controls which data sources are loaded. Each profile is a curated preset;
/// use [`AsInfoBuilder`] directly for fine-grained control beyond these.
///
/// # Profiles
///
/// | Profile | Sources | Load time | Output size |
/// |---------|---------|-----------|-------------|
/// | [`Minimum`](AsInfoProfile::Minimum) | `asn.txt` only | ~1s | ~37 MB JSONL |
/// | [`Default`](AsInfoProfile::Default) | asn.txt + as2org + population + hegemony + peeringdb | ~30s | ~50 MB JSONL |
/// | [`Full`](AsInfoProfile::Full) | everything: + delegated stats + IRR (all sources) + route prefixes | ~75s | ~210 MB JSONL |
///
/// # Example
///
/// ```rust,no_run
/// use bgpkit_commons::asinfo::AsInfoProfile;
/// use bgpkit_commons::BgpkitCommons;
///
/// let mut commons = BgpkitCommons::new();
/// commons.load_asinfo_with_profile(AsInfoProfile::Full).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AsInfoProfile {
    /// Core `asn.txt` only: AS names + countries. Fast (~1s), minimal data.
    Minimum,

    /// Production default: asn.txt + as2org + population + hegemony + peeringdb.
    /// Matches the current asninfo generator output.
    #[default]
    Default,

    /// Everything: all of Default + delegated stats + IRR data from all default
    /// sources (RIPE, APNIC, ARIN, LACNIC, AFRINIC, NTTCOM, RADB) including
    /// route prefix lists.
    Full,
}

impl AsInfoProfile {
    /// Convert this profile into a builder configuration.
    pub fn builder(self) -> AsInfoBuilder {
        match self {
            AsInfoProfile::Minimum => AsInfoBuilder::new(),
            AsInfoProfile::Default => AsInfoBuilder::new()
                .with_as2org()
                .with_population()
                .with_hegemony()
                .with_peeringdb(),
            AsInfoProfile::Full => AsInfoBuilder::new()
                .with_as2org()
                .with_population()
                .with_hegemony()
                .with_peeringdb()
                .with_delegated()
                .with_irr()
                .with_irr_route_prefixes(),
        }
    }
}

/// Builder for configuring which data sources to load for AS information.
///
/// This is the canonical way to configure AS info loading. All data sources
/// are opt-in — the core `asn.txt` name/country data always loads; everything
/// else is gated behind a builder method.
///
/// # Example
///
/// ```rust,no_run
/// use bgpkit_commons::asinfo::AsInfoBuilder;
///
/// let asinfo = AsInfoBuilder::new()
///     .with_delegated()
///     .with_irr()
///     .with_as2org()
///     .with_peeringdb()
///     .build()
///     .unwrap();
/// ```
///
/// Selecting specific IRR sources only:
///
/// ```rust,no_run
/// use bgpkit_commons::asinfo::AsInfoBuilder;
/// use bgpkit_commons::asinfo::IrrSourceConfig;
///
/// let asinfo = AsInfoBuilder::new()
///     .with_irr_sources(IrrSourceConfig::sources(&["RIPE", "RADB"]))
///     .build()
///     .unwrap();
/// ```
#[derive(Default)]
pub struct AsInfoBuilder {
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
    load_peeringdb: bool,
    load_delegated: bool,
    load_irr: bool,
    irr_config: IrrSourceConfig,
    irr_route_prefixes: bool,
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

    /// Enable loading RIR delegated-stats data (registry, country, date, status
    /// per ASN from five RIR delegated stats files).
    pub fn with_delegated(mut self) -> Self {
        self.load_delegated = true;
        self
    }

    /// Enable loading IRR data using all default sources (RIPE, APNIC, ARIN,
    /// LACNIC, AFRINIC, NTTCOM, RADB).
    pub fn with_irr(mut self) -> Self {
        self.load_irr = true;
        self
    }

    /// Enable loading IRR data with a custom set of sources.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use bgpkit_commons::asinfo::{AsInfoBuilder, IrrSourceConfig};
    ///
    /// let asinfo = AsInfoBuilder::new()
    ///     .with_irr_sources(IrrSourceConfig::sources(&["RIPE", "RADB"]))
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_irr_sources(mut self, config: IrrSourceConfig) -> Self {
        self.load_irr = true;
        self.irr_config = config;
        self
    }

    /// Enable collecting IRR route/route6 prefix lists per ASN.
    ///
    /// Off by default — prefix lists are the largest data component
    /// (~90MB JSON for all sources). Only enable when you need the
    /// actual registered prefixes, not just AS names/metadata.
    pub fn with_irr_route_prefixes(mut self) -> Self {
        self.irr_route_prefixes = true;
        self
    }

    /// Enable all optional data sources with default IRR source set.
    pub fn with_all(mut self) -> Self {
        self.load_as2org = true;
        self.load_population = true;
        self.load_hegemony = true;
        self.load_peeringdb = true;
        self.load_delegated = true;
        self.load_irr = true;
        self
    }

    /// Build the AsInfoUtils with the configured data sources.
    pub fn build(self) -> Result<AsInfoUtils> {
        AsInfoUtils::from_builder(&self)
    }

    /// Internal: expose config for AsInfoUtils construction.
    fn config(&self) -> AsInfoLoadConfig {
        AsInfoLoadConfig {
            load_as2org: self.load_as2org,
            load_population: self.load_population,
            load_hegemony: self.load_hegemony,
            load_peeringdb: self.load_peeringdb,
            load_delegated: self.load_delegated,
            load_irr: self.load_irr,
            irr_sources: self.irr_config.resolve(),
            irr_route_prefixes: self.irr_route_prefixes,
        }
    }
}

/// Internal configuration extracted from the builder.
#[derive(Debug, Clone)]
struct AsInfoLoadConfig {
    load_as2org: bool,
    load_population: bool,
    load_hegemony: bool,
    load_peeringdb: bool,
    load_delegated: bool,
    load_irr: bool,
    irr_sources: Vec<crate::irr::IrrSource>,
    irr_route_prefixes: bool,
}

pub struct AsInfoUtils {
    pub asinfo_map: HashMap<u32, AsInfo>,
    pub sibling_orgs: Option<SiblingOrgsUtils>,
    config: AsInfoLoadConfig,
}

impl AsInfoUtils {
    /// Build from a builder (canonical path).
    fn from_builder(builder: &AsInfoBuilder) -> Result<Self> {
        let config = builder.config();
        let asinfo_map = get_asinfo_map(&config)?;
        let sibling_orgs = if config.load_as2org {
            Some(SiblingOrgsUtils::new()?)
        } else {
            None
        };
        Ok(AsInfoUtils {
            asinfo_map,
            sibling_orgs,
            config,
        })
    }

    pub fn new_from_cached() -> Result<Self> {
        let asinfo_map = get_asinfo_map_cached()?;
        let sibling_orgs = Some(SiblingOrgsUtils::new()?);
        Ok(AsInfoUtils {
            asinfo_map,
            sibling_orgs,
            config: AsInfoLoadConfig {
                load_as2org: true,
                load_population: true,
                load_hegemony: true,
                load_peeringdb: true,
                load_delegated: true,
                load_irr: true,
                irr_sources: crate::irr::default_sources(),
                irr_route_prefixes: false,
            },
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.asinfo_map = get_asinfo_map(&self.config)?;
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

/// Parse RIR delegated stats text (NRO format) into an ASN -> DelegatedInfo map.
///
/// Record format: `registry|CC|type|start|value|date|status[|extensions]`.
/// Only `asn` records with `allocated`/`assigned` status and a real (non-empty,
/// non-`*`) country code are kept; private-use ASN ranges (RFC 6996:
/// 64512-65534 and 4200000000+) are excluded. Ranges are expanded per-ASN
/// (`value` is a count).
fn parse_delegated_stats(text: &str, map: &mut HashMap<u32, DelegatedInfo>) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 7 || parts[2] != "asn" {
            continue;
        }
        let status = parts[6].trim();
        if status != "allocated" && status != "assigned" {
            continue;
        }
        let cc = parts[1].trim();
        if cc.is_empty() || cc == "*" {
            continue;
        }
        let (Ok(start), Ok(count)) = (parts[3].parse::<u64>(), parts[4].parse::<u64>()) else {
            continue;
        };
        let registry = parts[0].trim().to_lowercase();
        let country = cc.to_uppercase();
        let date = parts.get(5).unwrap_or(&"").trim().to_string();
        for asn in start..start.saturating_add(count) {
            if asn > u32::MAX as u64 {
                break;
            }
            let asn = asn as u32;
            if (64512..=65534).contains(&asn) || asn >= 4_200_000_000 {
                continue;
            }
            map.entry(asn).or_insert(DelegatedInfo {
                registry: registry.clone(),
                country: country.clone(),
                date: date.clone(),
                status: status.to_string(),
            });
        }
    }
}

/// Look up optional enrichment data (as2org, population, hegemony, peeringdb)
/// for an ASN from already-loaded datasets. Shared by the main `asn.txt` parse
/// loop and the delegated-stats fill so both paths behave identically.
#[allow(clippy::type_complexity)]
fn lookup_enrichment(
    asn: u32,
    as2org_utils: Option<&as2org::As2org>,
    population_utils: Option<&population::AsnPopulation>,
    hegemony_utils: Option<&hegemony::Hegemony>,
    peeringdb_utils: Option<&Peeringdb>,
) -> (
    Option<As2orgInfo>,
    Option<AsnPopulationData>,
    Option<HegemonyData>,
    Option<Network>,
) {
    let as2org = as2org_utils.and_then(|as2org_data| {
        as2org_data.get_as_info(asn).map(|info| As2orgInfo {
            name: info.name.clone(),
            country: info.country_code.clone(),
            org_id: info.org_id.clone(),
            org_name: info.org_name.clone(),
        })
    });
    let population = population_utils.and_then(|p| p.get(asn));
    let hegemony = hegemony_utils.and_then(|h| h.get_score(asn).cloned());
    let peeringdb = peeringdb_utils.and_then(|h| h.get_network(asn).cloned());
    (as2org, population, hegemony, peeringdb)
}

/// Load RIR delegated stats and attach [`DelegatedInfo`] to every ASN in the
/// map. For ASNs missing from `asn.txt`, new entries are created with
/// `name: "UNKNOWN"` and the delegated country code.
///
/// Delegated stats are authoritative allocation records updated daily, covering
/// newly-allocated ASNs that `asn.txt` lags on by days to weeks. Every ASN
/// (not just gap ASNs) gets structured delegated data attached.
///
/// Best-effort: failures fetching individual files are logged and skipped.
fn fill_delegated_data(
    asnames_map: &mut HashMap<u32, AsInfo>,
    as2org_utils: Option<&as2org::As2org>,
    population_utils: Option<&population::AsnPopulation>,
    hegemony_utils: Option<&hegemony::Hegemony>,
    peeringdb_utils: Option<&Peeringdb>,
) {
    let read_text = |url: &str| -> Result<String> {
        let mut text = String::new();
        oneio::get_reader(url)?.read_to_string(&mut text)?;
        Ok(text)
    };

    let mut delegated: HashMap<u32, DelegatedInfo> = HashMap::new();
    for url in RIR_DELEGATED_STATS_URLS {
        match read_text(url) {
            Ok(text) => parse_delegated_stats(&text, &mut delegated),
            Err(e) => warn!("failed to load delegated stats from {}: {}", url, e),
        }
    }

    // Attach delegated data to existing entries, and create new entries for
    // ASNs missing from asn.txt.
    let mut new_entries = 0usize;
    let mut attached = 0usize;
    for (asn, delegated_info) in delegated {
        asnames_map
            .entry(asn)
            .and_modify(|info| {
                info.delegated = Some(delegated_info.clone());
                attached += 1;
            })
            .or_insert_with(|| {
                new_entries += 1;
                let (as2org, population, hegemony, peeringdb) = lookup_enrichment(
                    asn,
                    as2org_utils,
                    population_utils,
                    hegemony_utils,
                    peeringdb_utils,
                );
                AsInfo {
                    asn,
                    name: "UNKNOWN".to_string(),
                    country: delegated_info.country.clone(),
                    as2org,
                    population,
                    hegemony,
                    peeringdb,
                    delegated: Some(delegated_info.clone()),
                    irr: Vec::new(),
                }
            });
    }
    info!(
        "delegated stats: {attached} existing entries enriched, {new_entries} new entries created"
    );
}
/// Enrich ASInfo entries with structured IRR data from all default sources.
///
/// For each IRR source (RIPE, APNIC, ARIN, LACNIC, AFRINIC, NTTCOM, RADB),
/// collects:
/// - `aut-num` objects → `as-name`, `descr`, `mnt-by`
/// - `route` objects → registered IPv4 prefixes per ASN
/// - `route6` objects → registered IPv6 prefixes per ASN
/// - `as-set` objects → reverse membership (which sets contain this ASN)
///
/// Each source produces an [`IrrAsnInfo`] entry in the `irr` Vec, so callers
/// can pick which source(s) to trust. Per-source failures are logged and
/// skipped.
///
/// Additionally, for entries whose `name` is `"UNKNOWN"`, the `as-name` from
/// the first IRR source (by default priority order) that has a non-empty name
/// replaces the placeholder.
fn enrich_from_irr(
    asnames_map: &mut HashMap<u32, AsInfo>,
    irr_sources: &[crate::irr::IrrSource],
    collect_route_prefixes: bool,
) {
    use crate::irr::sources::DumpFormat;
    use crate::irr::types::{IrrObject, IrrObjectType};
    use std::collections::HashMap as StdMap;

    // Per-source accumulator: source_name -> (asn -> IrrAsnInfo builder)
    let mut per_source: StdMap<String, StdMap<u32, IrrAsnInfoBuilder>> = StdMap::new();

    // Track which dump URLs we've already parsed (whole-DB files serve all types).
    let mut parsed_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    let wanted_types: Vec<IrrObjectType> = if collect_route_prefixes {
        vec![
            IrrObjectType::AutNum,
            IrrObjectType::Route,
            IrrObjectType::Route6,
            IrrObjectType::AsSet,
        ]
    } else {
        // Without route prefixes: only aut-num + as-set.
        // For WholeDb sources this means we still download once but skip route objects.
        // For SplitFile sources we skip the route/route6 files entirely.
        vec![IrrObjectType::AutNum, IrrObjectType::AsSet]
    };

    for source in irr_sources.iter().cloned() {
        let source_name = source.name.to_string();

        // For each source, figure out the unique URLs to download.
        // SplitFile sources have one URL per type; WholeDb has a single URL
        // that we parse once and extract all types.
        let mut urls_to_parse: Vec<(String, Vec<IrrObjectType>)> = Vec::new();

        if source.format == DumpFormat::WholeDb {
            // Single URL, parse once for all types
            let url = source.dump_urls(IrrObjectType::AutNum);
            if let Some(dump) = url.first() {
                urls_to_parse.push((dump.url.clone(), wanted_types.to_vec()));
            }
        } else {
            // Split files: one URL per type
            for obj_type in &wanted_types {
                for dump in source.dump_urls(*obj_type) {
                    urls_to_parse.push((dump.url.clone(), vec![*obj_type]));
                }
            }
        }

        for (url, _types_for_url) in urls_to_parse {
            if parsed_urls.contains(&url) {
                continue;
            }
            parsed_urls.insert(url.clone());

            let sn = source_name.clone();

            match crate::irr::parse_dump(
                &crate::irr::IrrDumpUrl {
                    url: url.clone(),
                    transport: source.transport,
                    format: source.format,
                },
                |obj| {
                    let source_map = per_source.entry(sn.clone()).or_default();
                    match &obj {
                        IrrObject::AutNum(a) => {
                            let entry = source_map.entry(a.asn).or_default();
                            entry.source = a.source.clone();
                            entry.as_name = a.as_name.clone();
                            entry.descr = a.descr.clone();
                            if let Some(mnt) = a.extra.get("mnt-by") {
                                entry.mnt_by = mnt.clone();
                            }
                        }
                        IrrObject::Route(r) => {
                            let entry = source_map.entry(r.origin).or_default();
                            if entry.source.is_empty() {
                                entry.source = r.source.clone();
                            }
                            entry.route_prefixes.push(r.prefix.to_string());
                        }
                        IrrObject::Route6(r) => {
                            let entry = source_map.entry(r.origin).or_default();
                            if entry.source.is_empty() {
                                entry.source = r.source.clone();
                            }
                            entry.route6_prefixes.push(r.prefix.to_string());
                        }
                        IrrObject::AsSet(s) => {
                            let set_name = s.name.clone();
                            for &member_asn in &s.members {
                                let entry = source_map.entry(member_asn).or_default();
                                if entry.source.is_empty() {
                                    entry.source = sn.clone();
                                }
                                entry.member_of_sets.push(set_name.clone());
                            }
                        }
                        _ => {}
                    }
                },
            ) {
                Ok(stats) => info!(
                    "IRR from {source_name} ({url}): {} objects extracted",
                    stats.extracted
                ),
                Err(e) => warn!("failed to load IRR from {source_name} ({url}): {e}"),
            }
        }
    }

    // Attach per-source IrrAsnInfo to each ASN
    let mut irr_attached = 0usize;
    let mut names_filled = 0usize;

    for (asn, info) in asnames_map.iter_mut() {
        let mut irr_entries: Vec<IrrAsnInfo> = Vec::new();

        for source in irr_sources.iter().cloned() {
            if let Some(source_map) = per_source.get(source.name) {
                if let Some(builder) = source_map.get(asn) {
                    irr_entries.push(builder.clone().build());
                }
            }
        }

        // Fill UNKNOWN name from first IRR source with a non-empty as_name
        if info.name == "UNKNOWN" {
            for irr_info in &irr_entries {
                if !irr_info.as_name.is_empty() {
                    info.name = irr_info.as_name.clone();
                    names_filled += 1;
                    break;
                }
            }
        }

        if !irr_entries.is_empty() {
            info.irr = irr_entries;
            irr_attached += 1;
        }
    }

    info!("IRR data attached to {irr_attached} ASNs, {names_filled} UNKNOWN names filled from IRR");
}

/// Builder for IrrAsnInfo — accumulates data from multiple object types
/// (aut-num, route, route6, as-set) before producing the final struct.
#[derive(Debug, Clone, Default)]
struct IrrAsnInfoBuilder {
    as_name: String,
    descr: Vec<String>,
    source: String,
    mnt_by: Vec<String>,
    route_prefixes: Vec<String>,
    route6_prefixes: Vec<String>,
    member_of_sets: Vec<String>,
}

impl IrrAsnInfoBuilder {
    fn build(self) -> IrrAsnInfo {
        IrrAsnInfo {
            as_name: self.as_name,
            descr: self.descr,
            source: self.source,
            mnt_by: self.mnt_by,
            route_prefixes: self.route_prefixes,
            route6_prefixes: self.route6_prefixes,
            member_of_sets: self.member_of_sets,
        }
    }
}

/// Loads the ASN information map and returns it.
///
/// The core RIPE NCC `asn.txt` data (plus the RIR delegated-stats fill) is
/// required: load failures propagate as `Err`. Optional enrichment datasets
/// (as2org, population, hegemony, peeringdb) fail soft — a failed download or
/// API error (e.g., PeeringDB rate limiting without `PEERINGDB_API_KEY`)
/// logs a warning and proceeds with that dataset's fields left as `None`.
/// Loads the ASN information map and returns it.
///
/// The core RIPE NCC `asn.txt` data (plus the RIR delegated-stats fill) is
/// required: load failures propagate as `Err`. Optional enrichment datasets
/// (as2org, population, hegemony, peeringdb) fail soft — a failed download or
/// API error (e.g., PeeringDB rate limiting without `PEERINGDB_API_KEY`)
/// logs a warning and proceeds with that dataset's fields left as `None`.
fn get_asinfo_map(config: &AsInfoLoadConfig) -> Result<HashMap<u32, AsInfo>> {
    let load_as2org = config.load_as2org;
    let load_population = config.load_population;
    let load_hegemony = config.load_hegemony;
    let load_peeringdb = config.load_peeringdb;
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
        match as2org::As2org::new(None) {
            Ok(data) => Some(data),
            Err(e) => {
                warn!("failed to load as2org data, proceeding without it: {e}");
                None
            }
        }
    } else {
        None
    };
    let population_utils = if load_population {
        info!("loading ASN population data from APNIC...");
        match population::AsnPopulation::new() {
            Ok(data) => Some(data),
            Err(e) => {
                warn!("failed to load population data, proceeding without it: {e}");
                None
            }
        }
    } else {
        None
    };
    let hegemony_utils = if load_hegemony {
        info!("loading IIJ IHR hegemony score data from BGPKIT mirror...");
        match hegemony::Hegemony::new() {
            Ok(data) => Some(data),
            Err(e) => {
                warn!("failed to load hegemony data, proceeding without it: {e}");
                None
            }
        }
    } else {
        None
    };
    let peeringdb_utils = if load_peeringdb {
        info!("loading peeringdb data...");
        match Peeringdb::new_networks_only() {
            Ok(data) => Some(data),
            Err(e) => {
                warn!(
                    "failed to load peeringdb data, proceeding without it: {e} \
                     (hint: set PEERINGDB_API_KEY to avoid rate limiting)"
                );
                None
            }
        }
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
            let (as2org, population, hegemony, peeringdb) = lookup_enrichment(
                asn,
                as2org_utils.as_ref(),
                population_utils.as_ref(),
                hegemony_utils.as_ref(),
                peeringdb_utils.as_ref(),
            );
            Some(AsInfo {
                asn,
                name: name_str.to_string(),
                country: country_str.to_string(),
                as2org,
                population,
                hegemony,
                peeringdb,
                delegated: None,
                irr: Vec::new(),
            })
        })
        .collect::<Vec<AsInfo>>();

    let mut asnames_map = HashMap::new();
    for asname in asnames {
        asnames_map.insert(asname.asn, asname);
    }

    if config.load_delegated {
        info!("loading delegated stats data...");
        fill_delegated_data(
            &mut asnames_map,
            as2org_utils.as_ref(),
            population_utils.as_ref(),
            hegemony_utils.as_ref(),
            peeringdb_utils.as_ref(),
        );
    }

    if config.load_irr {
        info!("enriching from IRR data...");
        enrich_from_irr(
            &mut asnames_map,
            &config.irr_sources,
            config.irr_route_prefixes,
        );
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
    /// bgpkit.load_asinfo_with_profile(Default::default()).unwrap();
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
    /// bgpkit.load_asinfo_with_profile(Default::default()).unwrap();
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
    /// bgpkit.load_asinfo_with(bgpkit.asinfo_builder().with_as2org()).unwrap();
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
        if !self.asinfo.as_ref().unwrap().config.load_as2org {
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

    /// Helper: check country from DelegatedInfo in map.
    fn cc(map: &HashMap<u32, DelegatedInfo>, asn: u32) -> Option<&str> {
        map.get(&asn).map(|d| d.country.as_str())
    }

    #[test]
    fn test_parse_delegated_stats_basic() {
        let text = "\
2|ripencc|ZZ|209|20250704|00000000+00000000+00000000|UTF-8
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
        assert_eq!(cc(&map, 219157), Some("GB"));
        assert_eq!(cc(&map, 219125), Some("DE"));
        assert_eq!(cc(&map, 402598), Some("US"));
        assert_eq!(cc(&map, 154708), Some("BD"));
        // range expansion: AS1000..=AS1003 (value is a count of 4)
        assert_eq!(cc(&map, 1000), Some("NL"));
        assert_eq!(cc(&map, 1003), Some("NL"));
        assert!(!map.contains_key(&1004));
        // reserved entries with empty CC are skipped
        assert!(!map.contains_key(&212));
        // non-asn records are skipped
        assert_eq!(map.len(), 8);
        // Verify structured fields
        let info = &map[&219157];
        assert_eq!(info.registry, "ripencc");
        assert_eq!(info.status, "allocated");
        assert_eq!(info.date, "20260722");
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
    fn test_parse_delegated_stats_status_filter() {
        // reserved/available records are dropped even when they carry a
        // real-looking country code; only allocated/assigned are kept
        let text = "\
arin|US|asn|300000|1|20200101|reserved
arin|US|asn|300001|1|20200101|available
arin|US|asn|300002|1|20200101|allocated
arin|US|asn|300003|1|20200101|assigned
";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert!(!map.contains_key(&300000));
        assert!(!map.contains_key(&300001));
        assert_eq!(cc(&map, 300002), Some("US"));
        assert_eq!(cc(&map, 300003), Some("US"));
        assert_eq!(map.len(), 2);
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
        assert_eq!(cc(&map, 65535), Some("US"));
        assert!(!map.contains_key(&65534));
        assert_eq!(cc(&map, 64496), Some("US"));
        assert_eq!(cc(&map, 4199999999), Some("US"));
        assert!(!map.contains_key(&4200000000));
    }

    #[test]
    fn test_parse_delegated_stats_case_normalization() {
        let text = "lacnic|br|asn|269000|1|20150101|allocated\n";
        let mut map = HashMap::new();
        parse_delegated_stats(text, &mut map);
        assert_eq!(cc(&map, 269000), Some("BR"));
        assert_eq!(map[&269000].registry, "lacnic");
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
        assert_eq!(cc(&map, 100), Some("GB"));
        assert_eq!(map.len(), 1);
    }
}
