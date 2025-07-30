//! # Overview
//!
//! BGPKIT-Commons is a library for common BGP-related data and functions with a lazy-loading architecture.
//! Each module can be independently enabled via feature flags, allowing for minimal builds.
//!
//! ## Available Modules
//!
//! ### [`asinfo`] - Autonomous System Information (requires `asinfo` feature)
//! **Load Method**: `load_asinfo(as2org, population, hegemony, peeringdb)` or `load_asinfo_cached()`  
//! **Access Methods**: `asinfo_get(asn)`, `asinfo_all()`  
//! **Data Sources**: RIPE NCC, CAIDA as2org, APNIC population, IIJ IHR hegemony, PeeringDB  
//! **Functionality**: AS name resolution, country mapping, organization data, population statistics, hegemony scores
//!
//! ### [`as2rel`] - AS Relationship Data (requires `as2rel` feature)
//! **Load Method**: `load_as2rel()`  
//! **Access Methods**: `as2rel_lookup(asn1, asn2)`  
//! **Data Sources**: BGPKIT AS relationship inference  
//! **Functionality**: Provider-customer, peer-to-peer, and sibling relationships between ASes
//!
//! ### [`bogons`] - Bogon Detection (requires `bogons` feature)
//! **Load Method**: `load_bogons()`  
//! **Access Methods**: `bogons_match(input)`, `bogons_match_prefix(prefix)`, `bogons_match_asn(asn)`, `get_bogon_prefixes()`, `get_bogon_asns()`  
//! **Data Sources**: IANA special registries (IPv4, IPv6, ASN)  
//! **Functionality**: Detect invalid/reserved IP prefixes and ASNs that shouldn't appear in routing
//!
//! ### [`countries`] - Country Information (requires `countries` feature)
//! **Load Method**: `load_countries()`  
//! **Access Methods**: `country_by_code(code)`, country lookup by name  
//! **Data Sources**: GeoNames geographical database  
//! **Functionality**: ISO country code to name mapping and geographical information
//!
//! ### [`mrt_collectors`] - MRT Collector Metadata (requires `mrt_collectors` feature)
//! **Load Methods**: `load_mrt_collectors()`, `load_mrt_collector_peers()`  
//! **Access Methods**: `mrt_collectors_all()`, `mrt_collector_peers()`, `mrt_collector_peers_full_feed()`  
//! **Data Sources**: RouteViews and RIPE RIS official APIs  
//! **Functionality**: BGP collector information, peer details, full-feed vs partial-feed classification
//!
//! ### [`rpki`] - RPKI Validation (requires `rpki` feature)
//! **Load Method**: `load_rpki(optional_date)`  
//! **Access Methods**: `rpki_validate(prefix, asn)`  
//! **Data Sources**: RIPE NCC historical data, Cloudflare real-time data  
//! **Functionality**: Route Origin Authorization (ROA) validation, supports multiple ROAs per prefix
//!
//! ## Quick Start
//!
//! Add `bgpkit-commons` to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! bgpkit-commons = "0.8"
//! ```
//!
//! ### Basic Usage Pattern
//!
//! All modules follow the same lazy-loading pattern:
//! 1. Create a mutable `BgpkitCommons` instance
//! 2. Load the data you need by calling `load_xxx()` methods
//! 3. Access the data using the corresponding `xxx_yyy()` methods
//!
//! ```rust
//! use bgpkit_commons::BgpkitCommons;
//!
//! let mut commons = BgpkitCommons::new();
//!
//! // Load bogon data
//! commons.load_bogons().unwrap();
//!
//! // Use the data
//! if let Some(is_bogon) = commons.bogons_match("23456") {
//!     println!("ASN 23456 is a bogon: {}", is_bogon);
//! }
//! ```
//!
//! ### Working with Multiple Modules
//!
//! ```rust
//! use bgpkit_commons::BgpkitCommons;
//!
//! let mut commons = BgpkitCommons::new();
//!
//! // Load multiple data sources
//! commons.load_asinfo(false, false, false, false).unwrap();
//! commons.load_countries().unwrap();
//!
//! // Use the data together
//! if let Ok(Some(asinfo)) = commons.asinfo_get(13335) {
//!     println!("AS13335: {} ({})", asinfo.name, asinfo.country);
//! }
//! ```
//!
//! ## Feature Flags
//!
//! ### Module Features
//! - `asinfo` - AS information with organization and population data
//! - `as2rel` - AS relationship data
//! - `bogons` - Bogon prefix and ASN detection  
//! - `countries` - Country information lookup
//! - `mrt_collectors` - MRT collector metadata
//! - `rpki` - RPKI validation functionality
//!
//! ### Convenience Features  
//! - `all` (default) - Enables all modules for backwards compatibility
//!
//! ### Minimal Build Example
//! ```toml
//! [dependencies]
//! bgpkit-commons = { version = "0.8", default-features = false, features = ["bogons", "countries"] }
//! ```
//!
//! ## Error Handling
//!
//! All access methods return `Result<T>` and will return an error if the corresponding module
//! hasn't been loaded yet or if there are data validation issues. Error messages include guidance
//! on which `load_xxx()` method to call. Always call the appropriate `load_xxx()` method before accessing data.
//!
//! ## Data Persistence and Reloading
//!
//! All loaded data is kept in memory for fast access. Use the `reload()` method to refresh
//! all currently loaded data sources:
//!
//! ```rust
//! # use bgpkit_commons::BgpkitCommons;
//! let mut commons = BgpkitCommons::new();
//! commons.load_bogons().unwrap();
//!
//! // Later, reload all loaded data
//! commons.reload().unwrap();
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/icon-transparent.png",
    html_favicon_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/favicon.ico"
)]

#[cfg(feature = "as2rel")]
pub mod as2rel;
#[cfg(feature = "asinfo")]
pub mod asinfo;
#[cfg(feature = "bogons")]
pub mod bogons;
#[cfg(feature = "countries")]
pub mod countries;
#[cfg(feature = "mrt_collectors")]
pub mod mrt_collectors;
#[cfg(feature = "rpki")]
pub mod rpki;

pub mod errors;

// Re-export error types for convenience
pub use errors::{BgpkitCommonsError, Result};

/// Trait for modules that support lazy loading and reloading of data
pub trait LazyLoadable {
    /// Reload the module's data from its external sources
    fn reload(&mut self) -> Result<()>;

    /// Check if the module's data is currently loaded
    fn is_loaded(&self) -> bool;

    /// Get a description of the module's current loading status
    fn loading_status(&self) -> &'static str;
}

#[derive(Default)]
pub struct BgpkitCommons {
    #[cfg(feature = "countries")]
    countries: Option<crate::countries::Countries>,
    #[cfg(feature = "rpki")]
    rpki_trie: Option<crate::rpki::RpkiTrie>,
    #[cfg(feature = "mrt_collectors")]
    mrt_collectors: Option<Vec<crate::mrt_collectors::MrtCollector>>,
    #[cfg(feature = "mrt_collectors")]
    mrt_collector_peers: Option<Vec<crate::mrt_collectors::MrtCollectorPeer>>,
    #[cfg(feature = "bogons")]
    bogons: Option<crate::bogons::Bogons>,
    #[cfg(feature = "asinfo")]
    asinfo: Option<crate::asinfo::AsInfoUtils>,
    #[cfg(feature = "as2rel")]
    as2rel: Option<crate::as2rel::As2relBgpkit>,
}

impl BgpkitCommons {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reload all data sources that are already loaded
    pub fn reload(&mut self) -> Result<()> {
        #[cfg(feature = "countries")]
        if self.countries.is_some() {
            self.load_countries()?;
        }
        #[cfg(feature = "rpki")]
        if let Some(rpki) = self.rpki_trie.as_mut() {
            rpki.reload()?;
        }
        #[cfg(feature = "mrt_collectors")]
        if self.mrt_collectors.is_some() {
            self.load_mrt_collectors()?;
        }
        #[cfg(feature = "mrt_collectors")]
        if self.mrt_collector_peers.is_some() {
            self.load_mrt_collector_peers()?;
        }
        #[cfg(feature = "bogons")]
        if self.bogons.is_some() {
            self.load_bogons()?;
        }
        #[cfg(feature = "asinfo")]
        if let Some(asinfo) = self.asinfo.as_mut() {
            asinfo.reload()?;
        }
        #[cfg(feature = "as2rel")]
        if self.as2rel.is_some() {
            self.load_as2rel()?;
        }

        Ok(())
    }

    /// Get loading status for all available modules
    pub fn loading_status(&self) -> Vec<(&'static str, &'static str)> {
        let mut status = Vec::new();

        #[cfg(feature = "countries")]
        if let Some(ref countries) = self.countries {
            status.push(("countries", countries.loading_status()));
        } else {
            status.push(("countries", "Countries data not loaded"));
        }

        #[cfg(feature = "bogons")]
        if let Some(ref bogons) = self.bogons {
            status.push(("bogons", bogons.loading_status()));
        } else {
            status.push(("bogons", "Bogons data not loaded"));
        }

        #[cfg(feature = "rpki")]
        if let Some(ref rpki) = self.rpki_trie {
            status.push(("rpki", rpki.loading_status()));
        } else {
            status.push(("rpki", "RPKI data not loaded"));
        }

        #[cfg(feature = "asinfo")]
        if let Some(ref asinfo) = self.asinfo {
            status.push(("asinfo", asinfo.loading_status()));
        } else {
            status.push(("asinfo", "ASInfo data not loaded"));
        }

        #[cfg(feature = "as2rel")]
        if let Some(ref as2rel) = self.as2rel {
            status.push(("as2rel", as2rel.loading_status()));
        } else {
            status.push(("as2rel", "AS2Rel data not loaded"));
        }

        #[cfg(feature = "mrt_collectors")]
        {
            if self.mrt_collectors.is_some() {
                status.push(("mrt_collectors", "MRT collectors data loaded"));
            } else {
                status.push(("mrt_collectors", "MRT collectors data not loaded"));
            }

            if self.mrt_collector_peers.is_some() {
                status.push(("mrt_collector_peers", "MRT collector peers data loaded"));
            } else {
                status.push(("mrt_collector_peers", "MRT collector peers data not loaded"));
            }
        }

        status
    }

    /// Load countries data
    #[cfg(feature = "countries")]
    pub fn load_countries(&mut self) -> Result<()> {
        self.countries = Some(crate::countries::Countries::new()?);
        Ok(())
    }

    /// Load RPKI data
    #[cfg(feature = "rpki")]
    pub fn load_rpki(&mut self, date_opt: Option<chrono::NaiveDate>) -> Result<()> {
        if let Some(date) = date_opt {
            self.rpki_trie = Some(crate::rpki::RpkiTrie::from_ripe_historical(date)?);
        } else {
            self.rpki_trie = Some(crate::rpki::RpkiTrie::from_cloudflare()?);
        }
        Ok(())
    }

    /// Load MRT mrt_collectors data
    #[cfg(feature = "mrt_collectors")]
    pub fn load_mrt_collectors(&mut self) -> Result<()> {
        self.mrt_collectors = Some(crate::mrt_collectors::get_all_collectors()?);
        Ok(())
    }

    /// Load MRT mrt_collectors data
    #[cfg(feature = "mrt_collectors")]
    pub fn load_mrt_collector_peers(&mut self) -> Result<()> {
        self.mrt_collector_peers = Some(crate::mrt_collectors::get_mrt_collector_peers()?);
        Ok(())
    }

    /// Load bogons data
    #[cfg(feature = "bogons")]
    pub fn load_bogons(&mut self) -> Result<()> {
        self.bogons = Some(crate::bogons::Bogons::new()?);
        Ok(())
    }

    /// Load AS name and country data
    #[cfg(feature = "asinfo")]
    pub fn load_asinfo(
        &mut self,
        load_as2org: bool,
        load_population: bool,
        load_hegemony: bool,
        load_peeringdb: bool,
    ) -> Result<()> {
        self.asinfo = Some(crate::asinfo::AsInfoUtils::new(
            load_as2org,
            load_population,
            load_hegemony,
            load_peeringdb,
        )?);
        Ok(())
    }

    #[cfg(feature = "asinfo")]
    pub fn load_asinfo_cached(&mut self) -> Result<()> {
        self.asinfo = Some(crate::asinfo::AsInfoUtils::new_from_cached()?);
        Ok(())
    }

    /// Load AS-level relationship data
    #[cfg(feature = "as2rel")]
    pub fn load_as2rel(&mut self) -> Result<()> {
        self.as2rel = Some(crate::as2rel::As2relBgpkit::new()?);
        Ok(())
    }
}
