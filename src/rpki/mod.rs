//! RPKI (Resource Public Key Infrastructure) validation and data structures.
//!
//! This module provides functionality for loading and validating RPKI data from multiple sources,
//! including real-time data from Cloudflare and historical data from RIPE NCC or RPKIviews.
//!
//! # Overview
//!
//! RPKI is a cryptographic framework used to secure internet routing by providing a way to
//! validate the authenticity of BGP route announcements. This module implements RPKI validation
//! using Route Origin Authorizations (ROAs) that specify which Autonomous Systems (ASes) are
//! authorized to originate specific IP prefixes.
//!
//! # Data Sources
//!
//! ## Real-time Data (Cloudflare)
//! - **Source**: [Cloudflare RPKI Portal](https://rpki.cloudflare.com/rpki.json)
//! - **Format**: JSON with ROAs, ASPAs, and BGPsec keys
//! - **Update Frequency**: Real-time
//! - **Features**: Includes expiry timestamps for temporal validation
//!
//! ## Historical Data (RIPE NCC)
//! - **Source**: [RIPE NCC FTP archives](https://ftp.ripe.net/rpki/)
//! - **Format**: JSON files (output.json.xz) with ROAs, ASPAs
//! - **Use Case**: Historical analysis and research
//! - **Date Range**: Configurable historical date
//! - **TAL Sources**:
//!     - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
//!     - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
//!     - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
//!     - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
//!     - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>
//!
//! ## Historical Data (RPKIviews)
//! - **Source**: [RPKIviews](https://rpkiviews.org/)
//! - **Format**: Compressed tarballs (.tgz) containing rpki-client.json
//! - **Use Case**: Historical analysis from multiple vantage points
//! - **Default Collector**: Kerfuffle (rpkiviews.kerfuffle.net)
//! - **Collectors**:
//!     - Josephine: A2B Internet (AS51088), Amsterdam, Netherlands
//!     - Amber: Massar (AS57777), Lugano, Switzerland
//!     - Dango: Internet Initiative Japan (AS2497), Tokyo, Japan
//!     - Kerfuffle: Kerfuffle, LLC (AS35008), Fremont, California, United States
//!
//! # Core Data Structures
//!
//! ## RpkiTrie
//! The main data structure that stores RPKI data in a trie for efficient prefix lookups:
//! - **Trie**: `IpnetTrie<Vec<Roa>>` - Maps IP prefixes to lists of ROA entries
//! - **ASPAs**: `Vec<Aspa>` - AS Provider Authorization records
//! - **Date**: `Option<NaiveDate>` - Optional date for historical data
//!
//! ## Roa
//! Represents a Route Origin Authorization with the following fields:
//! - `prefix: IpNet` - The IP prefix (e.g., 192.0.2.0/24)
//! - `asn: u32` - The authorized ASN (e.g., 64496)
//! - `max_length: u8` - Maximum allowed prefix length for more specifics
//! - `rir: Option<Rir>` - Regional Internet Registry that issued the ROA
//! - `not_before: Option<NaiveDateTime>` - ROA validity start time
//! - `not_after: Option<NaiveDateTime>` - ROA validity end time (from expires field)
//!
//! ## Aspa
//! Represents an AS Provider Authorization with the following fields:
//! - `customer_asn: u32` - The customer AS number
//! - `providers: Vec<u32>` - List of provider AS numbers
//! - `expires: Option<NaiveDateTime>` - When this ASPA expires
//!
//! ## Validation Results
//! RPKI validation returns one of three states:
//! - **Valid**: The prefix-ASN pair is explicitly authorized by a valid ROA
//! - **Invalid**: The prefix has ROAs but none authorize the given ASN
//! - **Unknown**: No ROAs exist for the prefix, or all ROAs are outside their validity period
//!
//! # Usage Examples
//!
//! ## Loading Real-time Data (Cloudflare)
//! ```rust,no_run
//! use bgpkit_commons::BgpkitCommons;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut commons = BgpkitCommons::new();
//!
//! // Load current RPKI data from Cloudflare
//! commons.load_rpki(None)?;
//!
//! // Validate a prefix-ASN pair (standard validation)
//! let result = commons.rpki_validate(64496, "192.0.2.0/24")?;
//! match result {
//!     bgpkit_commons::rpki::RpkiValidation::Valid => println!("Route is RPKI valid"),
//!     bgpkit_commons::rpki::RpkiValidation::Invalid => println!("Route is RPKI invalid"),
//!     bgpkit_commons::rpki::RpkiValidation::Unknown => println!("No RPKI data for this prefix"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Loading Historical Data with Source Selection
//! ```rust,no_run
//! use bgpkit_commons::BgpkitCommons;
//! use bgpkit_commons::rpki::{HistoricalRpkiSource, RpkiViewsCollector};
//! use chrono::NaiveDate;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut commons = BgpkitCommons::new();
//! let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
//!
//! // Load from RIPE NCC
//! commons.load_rpki_historical(date, HistoricalRpkiSource::Ripe)?;
//!
//! // Or load from RPKIviews (uses Kerfuffle collector by default)
//! let source = HistoricalRpkiSource::RpkiViews(RpkiViewsCollector::default());
//! commons.load_rpki_historical(date, source)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Listing Available Files
//! ```rust,no_run
//! use bgpkit_commons::BgpkitCommons;
//! use bgpkit_commons::rpki::{HistoricalRpkiSource, RpkiViewsCollector};
//! use chrono::NaiveDate;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let commons = BgpkitCommons::new();
//! let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
//!
//! // List available files from RPKIviews (multiple snapshots per day)
//! let source = HistoricalRpkiSource::RpkiViews(RpkiViewsCollector::default());
//! let rpkiviews_files = commons.list_rpki_files(date, source)?;
//! for file in &rpkiviews_files {
//!     println!("RPKIviews file: {} (timestamp: {})", file.url, file.timestamp);
//! }
//! # Ok(())
//! # }
//! ```

mod cloudflare;
mod ripe_historical;
pub(crate) mod rpki_client;
mod rpkiviews;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ipnet::IpNet;
use ipnet_trie::IpnetTrie;

use crate::errors::{load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
pub use ripe_historical::list_ripe_files;
use rpki_client::RpkiClientData;
pub use rpkiviews::{RpkiViewsCollector, list_rpkiviews_files};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

// ============================================================================
// Public Data Structures
// ============================================================================

/// A validated Route Origin Authorization (ROA).
///
/// ROAs authorize specific Autonomous Systems to originate specific IP prefixes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Roa {
    /// The IP prefix (e.g., 192.0.2.0/24 or 2001:db8::/32)
    pub prefix: IpNet,
    /// The AS number authorized to originate this prefix
    pub asn: u32,
    /// Maximum prefix length allowed for announcements
    pub max_length: u8,
    /// Regional Internet Registry that issued this ROA
    pub rir: Option<Rir>,
    /// ROA validity start time (if available)
    pub not_before: Option<NaiveDateTime>,
    /// ROA validity end time (from expires field)
    pub not_after: Option<NaiveDateTime>,
}

/// A validated AS Provider Authorization (ASPA).
///
/// ASPAs specify which ASes are authorized providers for a customer AS.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aspa {
    /// The customer AS number
    pub customer_asn: u32,
    /// List of provider AS numbers
    pub providers: Vec<u32>,
    /// When this ASPA expires
    pub expires: Option<NaiveDateTime>,
}

/// Information about an available RPKI data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpkiFile {
    /// Full URL to download the file
    pub url: String,
    /// Timestamp when the file was created
    pub timestamp: DateTime<Utc>,
    /// Size of the file in bytes (if available)
    pub size: Option<u64>,
    /// RIR that this file is for (for RIPE files)
    pub rir: Option<Rir>,
    /// Collector that provides this file (for RPKIviews files)
    pub collector: Option<RpkiViewsCollector>,
}

/// Historical RPKI data source.
///
/// Used to specify which data source to use when loading historical RPKI data.
#[derive(Debug, Clone, Default)]
pub enum HistoricalRpkiSource {
    /// RIPE NCC historical archives (data from all 5 RIRs)
    #[default]
    Ripe,
    /// RPKIviews collector
    RpkiViews(RpkiViewsCollector),
}

impl std::fmt::Display for HistoricalRpkiSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoricalRpkiSource::Ripe => write!(f, "RIPE NCC"),
            HistoricalRpkiSource::RpkiViews(collector) => write!(f, "RPKIviews ({})", collector),
        }
    }
}

/// Regional Internet Registry (RIR).
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rir {
    AFRINIC,
    APNIC,
    ARIN,
    LACNIC,
    RIPENCC,
}

impl FromStr for Rir {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "afrinic" => Ok(Rir::AFRINIC),
            "apnic" => Ok(Rir::APNIC),
            "arin" => Ok(Rir::ARIN),
            "lacnic" => Ok(Rir::LACNIC),
            "ripe" => Ok(Rir::RIPENCC),
            _ => Err(format!("unknown RIR: {}", s)),
        }
    }
}

impl Display for Rir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rir::AFRINIC => write!(f, "AFRINIC"),
            Rir::APNIC => write!(f, "APNIC"),
            Rir::ARIN => write!(f, "ARIN"),
            Rir::LACNIC => write!(f, "LACNIC"),
            Rir::RIPENCC => write!(f, "RIPENCC"),
        }
    }
}

impl Rir {
    pub fn to_ripe_ftp_root_url(&self) -> String {
        match self {
            Rir::AFRINIC => "https://ftp.ripe.net/rpki/afrinic.tal".to_string(),
            Rir::APNIC => "https://ftp.ripe.net/rpki/apnic.tal".to_string(),
            Rir::ARIN => "https://ftp.ripe.net/rpki/arin.tal".to_string(),
            Rir::LACNIC => "https://ftp.ripe.net/rpki/lacnic.tal".to_string(),
            Rir::RIPENCC => "https://ftp.ripe.net/rpki/ripencc.tal".to_string(),
        }
    }
}

/// RPKI validation result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RpkiValidation {
    /// The prefix-ASN pair is explicitly authorized by a valid ROA
    Valid,
    /// The prefix has ROAs but none authorize the given ASN
    Invalid,
    /// No ROAs exist for the prefix, or all ROAs are outside their validity period
    Unknown,
}

impl Display for RpkiValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpkiValidation::Valid => write!(f, "valid"),
            RpkiValidation::Invalid => write!(f, "invalid"),
            RpkiValidation::Unknown => write!(f, "unknown"),
        }
    }
}

// ============================================================================
// Backwards Compatibility Type Aliases
// ============================================================================

/// Type alias for backwards compatibility. Use [`Roa`] instead.
/// Deprecated since 0.10.0. This alias will be removed in version 0.12.0.
#[deprecated(since = "0.10.0", note = "Use Roa instead")]
pub type RoaEntry = Roa;

// ============================================================================
// RpkiTrie Implementation
// ============================================================================

/// The main RPKI data structure storing ROAs and ASPAs.
#[derive(Clone)]
pub struct RpkiTrie {
    /// Trie mapping IP prefixes to ROA entries
    pub trie: IpnetTrie<Vec<Roa>>,
    /// AS Provider Authorizations
    pub aspas: Vec<Aspa>,
    /// Date for historical data (None for real-time)
    date: Option<NaiveDate>,
}

impl Default for RpkiTrie {
    fn default() -> Self {
        Self {
            trie: IpnetTrie::new(),
            aspas: vec![],
            date: None,
        }
    }
}

impl RpkiTrie {
    /// Create a new empty RpkiTrie.
    pub fn new(date: Option<NaiveDate>) -> Self {
        Self {
            trie: IpnetTrie::new(),
            aspas: vec![],
            date,
        }
    }

    /// Insert a ROA. Returns true if this is a new prefix, false if added to existing prefix.
    /// Duplicates are avoided - ROAs with same (prefix, asn, max_length) are considered identical.
    pub fn insert_roa(&mut self, roa: Roa) -> bool {
        match self.trie.exact_match_mut(roa.prefix) {
            Some(existing_roas) => {
                // Check if this ROA already exists (same prefix, asn, max_length)
                if !existing_roas.iter().any(|existing| {
                    existing.asn == roa.asn && existing.max_length == roa.max_length
                }) {
                    existing_roas.push(roa);
                }
                false
            }
            None => {
                self.trie.insert(roa.prefix, vec![roa]);
                true
            }
        }
    }

    /// Insert multiple ROAs.
    pub fn insert_roas(&mut self, roas: Vec<Roa>) {
        for roa in roas {
            self.insert_roa(roa);
        }
    }

    /// Convert rpki-client data into an RpkiTrie.
    ///
    /// This is a shared conversion function used by all data sources
    /// (Cloudflare, RIPE, RPKIviews) since they all produce the same
    /// rpki-client JSON format.
    pub(crate) fn from_rpki_client_data(
        data: RpkiClientData,
        date: Option<NaiveDate>,
    ) -> Result<Self> {
        let mut trie = RpkiTrie::new(date);
        trie.merge_rpki_client_data(data);
        Ok(trie)
    }

    /// Merge rpki-client data into this trie.
    ///
    /// This converts ROAs and ASPAs from rpki-client format and inserts them,
    /// avoiding duplicates for ASPAs based on customer_asn.
    pub(crate) fn merge_rpki_client_data(&mut self, data: RpkiClientData) {
        // Convert and insert ROAs
        for roa in data.roas {
            let prefix = match roa.prefix.parse::<IpNet>() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let rir = Rir::from_str(&roa.ta).ok();
            let not_after =
                DateTime::from_timestamp(roa.expires as i64, 0).map(|dt| dt.naive_utc());

            self.insert_roa(Roa {
                prefix,
                asn: roa.asn,
                max_length: roa.max_length,
                rir,
                not_before: None,
                not_after,
            });
        }

        // Convert and merge ASPAs (avoiding duplicates based on customer_asn)
        for aspa in data.aspas {
            if !self
                .aspas
                .iter()
                .any(|a| a.customer_asn == aspa.customer_asid)
            {
                let expires = DateTime::from_timestamp(aspa.expires, 0).map(|dt| dt.naive_utc());
                self.aspas.push(Aspa {
                    customer_asn: aspa.customer_asid,
                    providers: aspa.providers,
                    expires,
                });
            }
        }
    }

    /// Lookup all ROAs that match a given prefix, including invalid ones.
    pub fn lookup_by_prefix(&self, prefix: &IpNet) -> Vec<Roa> {
        let mut all_matches = vec![];
        for (p, roas) in self.trie.matches(prefix) {
            if p.contains(prefix) {
                for roa in roas {
                    if roa.max_length >= prefix.prefix_len() {
                        all_matches.push(roa.clone());
                    }
                }
            }
        }
        all_matches
    }

    /// Validate a prefix with an ASN.
    ///
    /// Return values:
    /// - `RpkiValidation::Valid` if the prefix-asn pair is valid
    /// - `RpkiValidation::Invalid` if the prefix-asn pair is invalid
    /// - `RpkiValidation::Unknown` if the prefix-asn pair is not found in RPKI
    pub fn validate(&self, prefix: &IpNet, asn: u32) -> RpkiValidation {
        let matches = self.lookup_by_prefix(prefix);
        if matches.is_empty() {
            return RpkiValidation::Unknown;
        }

        for roa in matches {
            if roa.asn == asn && roa.max_length >= prefix.prefix_len() {
                return RpkiValidation::Valid;
            }
        }
        // there are matches but none of them is valid
        RpkiValidation::Invalid
    }

    /// Validate a prefix with an ASN, checking expiry dates.
    ///
    /// Return values:
    /// - `RpkiValidation::Valid` if the prefix-asn pair is valid and not expired
    /// - `RpkiValidation::Invalid` if the prefix-asn pair is invalid (wrong ASN)
    /// - `RpkiValidation::Unknown` if the prefix-asn pair is not found in RPKI or all matching ROAs are outside their valid time range
    pub fn validate_check_expiry(
        &self,
        prefix: &IpNet,
        asn: u32,
        check_time: Option<NaiveDateTime>,
    ) -> RpkiValidation {
        let matches = self.lookup_by_prefix(prefix);
        if matches.is_empty() {
            return RpkiValidation::Unknown;
        }

        let check_time = check_time.unwrap_or_else(|| Utc::now().naive_utc());

        let mut found_matching_asn = false;

        for roa in matches {
            if roa.asn == asn && roa.max_length >= prefix.prefix_len() {
                found_matching_asn = true;

                // Check if ROA is within valid time period
                let is_valid_time = {
                    if let Some(not_before) = roa.not_before {
                        if check_time < not_before {
                            false // ROA not yet valid
                        } else {
                            true
                        }
                    } else {
                        true // no not_before constraint
                    }
                } && {
                    if let Some(not_after) = roa.not_after {
                        if check_time > not_after {
                            false // ROA expired
                        } else {
                            true
                        }
                    } else {
                        true // no not_after constraint
                    }
                };

                if is_valid_time {
                    return RpkiValidation::Valid;
                }
            }
        }

        // If we found matching ASN but all ROAs are outside valid time range, return Unknown
        if found_matching_asn {
            return RpkiValidation::Unknown;
        }

        // There are matches but none of them match the ASN
        RpkiValidation::Invalid
    }

    /// Reload the RPKI data from its original source.
    pub fn reload(&mut self) -> Result<()> {
        match self.date {
            Some(date) => {
                let trie = RpkiTrie::from_ripe_historical(date)?;
                self.trie = trie.trie;
                self.aspas = trie.aspas;
            }
            None => {
                let trie = RpkiTrie::from_cloudflare()?;
                self.trie = trie.trie;
                self.aspas = trie.aspas;
            }
        }

        Ok(())
    }
}

impl LazyLoadable for RpkiTrie {
    fn reload(&mut self) -> Result<()> {
        self.reload()
    }

    fn is_loaded(&self) -> bool {
        !self.trie.is_empty()
    }

    fn loading_status(&self) -> &'static str {
        if self.is_loaded() {
            "RPKI data loaded"
        } else {
            "RPKI data not loaded"
        }
    }
}

// ============================================================================
// BgpkitCommons Integration
// ============================================================================

impl BgpkitCommons {
    pub fn rpki_lookup_by_prefix(&self, prefix: &str) -> Result<Vec<Roa>> {
        if self.rpki_trie.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::RPKI,
                load_methods::LOAD_RPKI,
            ));
        }

        let prefix = prefix.parse()?;

        Ok(self.rpki_trie.as_ref().unwrap().lookup_by_prefix(&prefix))
    }

    pub fn rpki_validate(&self, asn: u32, prefix: &str) -> Result<RpkiValidation> {
        if self.rpki_trie.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::RPKI,
                load_methods::LOAD_RPKI,
            ));
        }
        let prefix = prefix.parse()?;
        Ok(self.rpki_trie.as_ref().unwrap().validate(&prefix, asn))
    }

    pub fn rpki_validate_check_expiry(
        &self,
        asn: u32,
        prefix: &str,
        check_time: Option<NaiveDateTime>,
    ) -> Result<RpkiValidation> {
        if self.rpki_trie.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::RPKI,
                load_methods::LOAD_RPKI,
            ));
        }
        let prefix = prefix.parse()?;
        Ok(self
            .rpki_trie
            .as_ref()
            .unwrap()
            .validate_check_expiry(&prefix, asn, check_time))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn test_multiple_roas_same_prefix() {
        let mut trie = RpkiTrie::new(None);

        // Insert first ROA
        let roa1 = Roa {
            prefix: "192.0.2.0/24".parse().unwrap(),
            asn: 64496,
            max_length: 24,
            rir: Some(Rir::APNIC),
            not_before: None,
            not_after: None,
        };
        assert!(trie.insert_roa(roa1.clone()));

        // Insert second ROA with different ASN for same prefix
        let roa2 = Roa {
            prefix: "192.0.2.0/24".parse().unwrap(),
            asn: 64497,
            max_length: 24,
            rir: Some(Rir::APNIC),
            not_before: None,
            not_after: None,
        };
        assert!(!trie.insert_roa(roa2.clone()));

        // Insert duplicate ROA (same prefix, asn, max_length) - should be ignored
        let roa_dup = Roa {
            prefix: "192.0.2.0/24".parse().unwrap(),
            asn: 64496,
            max_length: 24,
            rir: Some(Rir::ARIN), // Different RIR shouldn't matter
            not_before: None,
            not_after: None,
        };
        assert!(!trie.insert_roa(roa_dup));

        // Insert ROA with different max_length - should be added
        let roa3 = Roa {
            prefix: "192.0.2.0/24".parse().unwrap(),
            asn: 64496,
            max_length: 28,
            rir: Some(Rir::APNIC),
            not_before: None,
            not_after: None,
        };
        assert!(!trie.insert_roa(roa3.clone()));

        // Lookup should return 3 ROAs (roa1, roa2, roa3)
        let prefix: IpNet = "192.0.2.0/24".parse().unwrap();
        let roas = trie.lookup_by_prefix(&prefix);
        assert_eq!(roas.len(), 3);

        // Validate AS 64496 - should be valid
        assert_eq!(trie.validate(&prefix, 64496), RpkiValidation::Valid);

        // Validate AS 64497 - should be valid
        assert_eq!(trie.validate(&prefix, 64497), RpkiValidation::Valid);

        // Validate AS 64498 - should be invalid (prefix has ROAs but not for this ASN)
        assert_eq!(trie.validate(&prefix, 64498), RpkiValidation::Invalid);

        // Validate unknown prefix - should be unknown
        let unknown_prefix: IpNet = "10.0.0.0/8".parse().unwrap();
        assert_eq!(
            trie.validate(&unknown_prefix, 64496),
            RpkiValidation::Unknown
        );
    }

    #[test]
    fn test_validate_check_expiry_with_time_constraints() {
        let mut trie = RpkiTrie::new(None);

        // Time references
        let past_time = DateTime::from_timestamp(1600000000, 0)
            .map(|dt| dt.naive_utc())
            .unwrap();
        let current_time = DateTime::from_timestamp(1700000000, 0)
            .map(|dt| dt.naive_utc())
            .unwrap();
        let future_time = DateTime::from_timestamp(1800000000, 0)
            .map(|dt| dt.naive_utc())
            .unwrap();

        // Insert ROA that's currently valid (not_before in past, not_after in future)
        let roa_valid = Roa {
            prefix: "192.0.2.0/24".parse().unwrap(),
            asn: 64496,
            max_length: 24,
            rir: Some(Rir::APNIC),
            not_before: Some(past_time),
            not_after: Some(future_time),
        };
        trie.insert_roa(roa_valid);

        // Insert ROA that's expired
        let roa_expired = Roa {
            prefix: "198.51.100.0/24".parse().unwrap(),
            asn: 64497,
            max_length: 24,
            rir: Some(Rir::APNIC),
            not_before: Some(past_time),
            not_after: Some(past_time), // Expired in the past
        };
        trie.insert_roa(roa_expired);

        // Insert ROA that's not yet valid
        let roa_future = Roa {
            prefix: "203.0.113.0/24".parse().unwrap(),
            asn: 64498,
            max_length: 24,
            rir: Some(Rir::APNIC),
            not_before: Some(future_time), // Not valid yet
            not_after: None,
        };
        trie.insert_roa(roa_future);

        // Test valid ROA at current time
        let prefix_valid: IpNet = "192.0.2.0/24".parse().unwrap();
        assert_eq!(
            trie.validate_check_expiry(&prefix_valid, 64496, Some(current_time)),
            RpkiValidation::Valid
        );

        // Test expired ROA at current time - should return Unknown (was valid but expired)
        let prefix_expired: IpNet = "198.51.100.0/24".parse().unwrap();
        assert_eq!(
            trie.validate_check_expiry(&prefix_expired, 64497, Some(current_time)),
            RpkiValidation::Unknown
        );

        // Test not-yet-valid ROA at current time - should return Unknown
        let prefix_future: IpNet = "203.0.113.0/24".parse().unwrap();
        assert_eq!(
            trie.validate_check_expiry(&prefix_future, 64498, Some(current_time)),
            RpkiValidation::Unknown
        );

        // Test not-yet-valid ROA at future time - should return Valid
        let far_future = DateTime::from_timestamp(1900000000, 0)
            .map(|dt| dt.naive_utc())
            .unwrap();
        assert_eq!(
            trie.validate_check_expiry(&prefix_future, 64498, Some(far_future)),
            RpkiValidation::Valid
        );

        // Test wrong ASN - should return Invalid
        assert_eq!(
            trie.validate_check_expiry(&prefix_valid, 64499, Some(current_time)),
            RpkiValidation::Invalid
        );
    }

    #[test]
    #[ignore] // Requires network access
    fn test_load_from_ripe_historical() {
        // Use a recent date that should have data available
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let trie = RpkiTrie::from_ripe_historical(date).expect("Failed to load RIPE data");

        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!(
            "Loaded {} ROAs from RIPE historical for {}",
            total_roas, date
        );
        println!("Loaded {} ASPAs", trie.aspas.len());

        assert!(total_roas > 0, "Should have loaded some ROAs");
    }

    #[test]
    #[ignore] // Requires network access
    fn test_load_from_rpkiviews() {
        // Note: This test streams from a remote tgz file but stops early
        // once rpki-client.json is found (typically at position 3-4 in the archive).
        // Due to streaming optimization, this typically completes in ~8 seconds.
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let trie = RpkiTrie::from_rpkiviews(RpkiViewsCollector::default(), date)
            .expect("Failed to load RPKIviews data");

        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!("Loaded {} ROAs from RPKIviews for {}", total_roas, date);
        println!("Loaded {} ASPAs", trie.aspas.len());

        assert!(total_roas > 0, "Should have loaded some ROAs");
    }

    #[test]
    #[ignore] // Requires network access
    fn test_rpkiviews_file_position() {
        // Verify that rpki-client.json appears early in the archive
        // This confirms our early-termination optimization works
        use crate::rpki::rpkiviews::list_files_in_tgz;

        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let files = list_rpkiviews_files(RpkiViewsCollector::default(), date)
            .expect("Failed to list files");

        assert!(!files.is_empty(), "Should have found some files");

        let tgz_url = &files[0].url;
        println!("Checking file positions in: {}", tgz_url);

        // List first 50 entries to see where rpki-client.json appears
        let entries = list_files_in_tgz(tgz_url, Some(50)).expect("Failed to list tgz entries");

        let json_position = entries
            .iter()
            .position(|e| e.path.ends_with("rpki-client.json"));

        println!("First {} entries:", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            println!("  [{}] {} ({} bytes)", i, entry.path, entry.size);
        }

        if let Some(pos) = json_position {
            println!(
                "\nrpki-client.json found at position {} (early in archive)",
                pos
            );
            assert!(
                pos < 50,
                "rpki-client.json should appear early in the archive"
            );
        } else {
            println!("\nrpki-client.json not in first 50 entries - may need to stream more");
        }
    }

    #[test]
    #[ignore] // Requires network access
    fn test_list_rpkiviews_files() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let files = list_rpkiviews_files(RpkiViewsCollector::default(), date)
            .expect("Failed to list files");

        println!("Found {} files for {} from Kerfuffle", files.len(), date);
        for file in files.iter().take(3) {
            println!(
                "  {} ({} bytes, {})",
                file.url,
                file.size.unwrap_or(0),
                file.timestamp
            );
        }

        assert!(!files.is_empty(), "Should have found some files");
    }
}
