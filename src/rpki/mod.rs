//! RPKI (Resource Public Key Infrastructure) validation and data structures.
//!
//! This module provides functionality for loading and validating RPKI data from multiple sources,
//! including real-time data from Cloudflare and historical data from RIPE NCC.
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
//! - **Format**: CSV files with historical RPKI states
//! - **Use Case**: Historical analysis and research
//! - **Date Range**: Configurable historical date
//! - **TAL Sources**:
//!     - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
//!     - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
//!     - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
//!     - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
//!     - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>
//!
//! # Core Data Structures
//!
//! ## RpkiTrie
//! The main data structure that stores RPKI data in a trie for efficient prefix lookups:
//! - **Trie**: `IpnetTrie<Vec<RoaEntry>>` - Maps IP prefixes to lists of ROA entries
//! - **ASPAs**: `Vec<CfAspaEntry>` - AS Provider Authorization records
//! - **Date**: `Option<NaiveDate>` - Optional date for historical data
//!
//! ## RoaEntry
//! Represents a Route Origin Authorization with the following fields:
//! - `prefix: IpNet` - The IP prefix (e.g., 192.0.2.0/24)
//! - `asn: u32` - The authorized ASN (e.g., 64496)
//! - `max_length: u8` - Maximum allowed prefix length for more specifics
//! - `rir: Option<Rir>` - Regional Internet Registry that issued the ROA
//! - `not_before: Option<NaiveDateTime>` - ROA validity start time
//! - `not_after: Option<NaiveDateTime>` - ROA validity end time (from expires field)
//!
//! ## Validation Results
//! RPKI validation returns one of three states:
//! - **Valid**: The prefix-ASN pair is explicitly authorized by a valid ROA
//! - **Invalid**: The prefix has ROAs but none authorize the given ASN
//! - **Unknown**: No ROAs exist for the prefix, or all ROAs are outside their validity period
//!
//! # Validation Process
//!
//! ## Standard Validation (`validate`)
//! 1. Look up all ROAs that cover the given prefix
//! 2. Check if any ROA authorizes the given ASN with appropriate max_length
//! 3. Return Valid/Invalid/Unknown based on matches
//!
//! ## Expiry-Aware Validation (`validate_check_expiry`)
//! 1. Look up all ROAs that cover the given prefix
//! 2. Filter ROAs to only include those within their validity time window:
//!    - Check `not_before` ≤ check_time (if present)
//!    - Check `not_after` ≥ check_time (if present)
//! 3. Among time-valid ROAs, check for ASN authorization
//! 4. Return validation result:
//!    - **Valid**: Time-valid ROA found for the ASN
//!    - **Invalid**: Time-valid ROAs exist but none authorize the ASN
//!    - **Unknown**: No ROAs found, or all ROAs are outside validity period
//!
//! # Key Features
//!
//! - **Multiple ROAs per prefix**: A single prefix can have multiple valid ROAs with different ASNs
//! - **Duplicate prevention**: ROAs with identical (prefix, asn, max_length) are automatically deduplicated
//! - **Efficient lookup**: Fast prefix matching using a trie data structure
//! - **Temporal validation**: Support for time-aware validation with expiry checking
//! - **Comprehensive validation**: Full RPKI validation against stored ROAs
//!
//! # Usage Examples
//!
//! ## Loading Real-time Data (Cloudflare)
//! ```rust
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
//!
//! // Validate with expiry checking (current time)
//! let result = commons.rpki_validate_check_expiry(64496, "192.0.2.0/24", None)?;
//!
//! // Validate with expiry checking (specific time)
//! use chrono::{DateTime, Utc};
//! let check_time = DateTime::from_timestamp(1700000000, 0).unwrap().naive_utc();
//! let result = commons.rpki_validate_check_expiry(64496, "192.0.2.0/24", Some(check_time))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Loading Historical Data (RIPE)
//! ```rust
//! use bgpkit_commons::BgpkitCommons;
//! use chrono::NaiveDate;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut commons = BgpkitCommons::new();
//!
//! // Load RPKI data for a specific historical date
//! let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
//! commons.load_rpki(Some(date))?;
//!
//! // Validate using historical data
//! let result = commons.rpki_validate(64496, "192.0.2.0/24")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Direct Trie Usage
//! ```rust
//! use bgpkit_commons::rpki::{RpkiTrie, RpkiValidation};
//! use ipnet::IpNet;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load from Cloudflare directly
//! let trie = RpkiTrie::from_cloudflare()?;
//!
//! // Lookup all ROAs for a prefix
//! let prefix: IpNet = "192.0.2.0/24".parse()?;
//! let roas = trie.lookup_by_prefix(&prefix);
//! println!("Found {} ROAs for prefix", roas.len());
//!
//! // Validate with expiry checking
//! let result = trie.validate_check_expiry(&prefix, 64496, None);
//! # Ok(())
//! # }
//! ```
//!
//! ## Handling Multiple ROAs
//! A single prefix can have multiple ROAs with different ASNs and validity periods:
//! ```rust
//! use bgpkit_commons::BgpkitCommons;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut commons = BgpkitCommons::new();
//! commons.load_rpki(None)?;
//!
//! // Look up all ROAs for a prefix
//! let roas = commons.rpki_lookup_by_prefix("192.0.2.0/24")?;
//! for roa in roas {
//!     println!("ASN: {}, Max Length: {}, Expires: {:?}",
//!              roa.asn, roa.max_length, roa.not_after);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Performance Considerations
//!
//! - **Trie Structure**: Uses `ipnet-trie` for O(log n) prefix lookups
//! - **Memory Usage**: Stores all ROAs in memory for fast access
//! - **Loading Time**: Initial load from Cloudflare takes a few seconds
//! - **Caching**: No automatic caching - reload when fresh data is needed
//!
//! # Error Handling
//!
//! All validation methods return `Result<RpkiValidation>` and can fail due to:
//! - Network errors when loading data
//! - Invalid prefix format in input
//! - RPKI data not loaded (call `load_rpki_*` methods first)

mod cloudflare;
mod ripe_historical;
// mod rpkiviews;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use ipnet::IpNet;
use ipnet_trie::IpnetTrie;

use crate::errors::{load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
pub use cloudflare::*;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone)]
pub struct RpkiTrie {
    pub trie: IpnetTrie<Vec<RoaEntry>>,
    pub aspas: Vec<CfAspaEntry>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoaEntry {
    pub prefix: IpNet,
    pub asn: u32,
    pub max_length: u8,
    pub rir: Option<Rir>,
    pub not_before: Option<NaiveDateTime>,
    pub not_after: Option<NaiveDateTime>,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RpkiValidation {
    Valid,
    Invalid,
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

impl RpkiTrie {
    pub fn new(date: Option<NaiveDate>) -> Self {
        Self {
            trie: IpnetTrie::new(),
            aspas: vec![],
            date,
        }
    }

    /// insert an [RoaEntry]. Returns true if this is a new prefix, false if added to existing prefix.
    /// Duplicates are avoided - ROAs with same (prefix, asn, max_length) are considered identical.
    pub fn insert_roa(&mut self, roa: RoaEntry) -> bool {
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

    /// insert multiple [RoaEntry]s
    pub fn insert_roas(&mut self, roas: Vec<RoaEntry>) {
        for roa in roas {
            self.insert_roa(roa);
        }
    }

    /// Lookup all ROAs that match a given prefix, including invalid ones
    pub fn lookup_by_prefix(&self, prefix: &IpNet) -> Vec<RoaEntry> {
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

    /// Validate a prefix with an ASN
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

    /// Validate a prefix with an ASN, checking expiry dates
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

    pub fn reload(&mut self) -> Result<()> {
        match self.date {
            Some(date) => {
                let trie = RpkiTrie::from_ripe_historical(date)?;
                self.trie = trie.trie;
            }
            None => {
                let trie = RpkiTrie::from_cloudflare()?;
                self.trie = trie.trie;
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

impl BgpkitCommons {
    pub fn rpki_lookup_by_prefix(&self, prefix: &str) -> Result<Vec<RoaEntry>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn test_multiple_roas_same_prefix() {
        let mut trie = RpkiTrie::new(None);

        // Create a test prefix
        let prefix: IpNet = "10.0.0.0/8".parse().unwrap();

        // Create multiple ROAs for the same prefix with different ASNs
        let roa1 = RoaEntry {
            prefix,
            asn: 64496,
            max_length: 16,
            rir: Some(Rir::ARIN),
            not_before: None,
            not_after: None,
        };

        let roa2 = RoaEntry {
            prefix,
            asn: 64497,
            max_length: 24,
            rir: Some(Rir::ARIN),
            not_before: None,
            not_after: None,
        };

        // Create a duplicate ROA (same prefix, asn, max_length as roa1)
        let roa1_duplicate = RoaEntry {
            prefix,
            asn: 64496,
            max_length: 16,
            rir: Some(Rir::APNIC), // Different RIR but same (prefix, asn, max_length)
            not_before: None,
            not_after: None,
        };

        // Insert ROAs
        assert!(trie.insert_roa(roa1)); // Should return true for new prefix
        assert!(!trie.insert_roa(roa2)); // Should return false for existing prefix
        assert!(!trie.insert_roa(roa1_duplicate)); // Should return false and not add duplicate

        // Lookup should return only 2 ROAs (duplicate should be ignored)
        let matches = trie.lookup_by_prefix(&prefix);
        assert_eq!(matches.len(), 2);

        // Check that both ASNs are present
        let asns: std::collections::HashSet<u32> = matches.iter().map(|r| r.asn).collect();
        assert!(asns.contains(&64496));
        assert!(asns.contains(&64497));

        // Test validation - should be valid for both ASNs
        assert_eq!(trie.validate(&prefix, 64496), RpkiValidation::Valid);
        assert_eq!(trie.validate(&prefix, 64497), RpkiValidation::Valid);
        assert_eq!(trie.validate(&prefix, 64498), RpkiValidation::Invalid);
    }

    #[test]
    fn test_validate_check_expiry() {
        let mut trie = RpkiTrie::new(None);

        // Create a test prefix
        let prefix: IpNet = "10.0.0.0/8".parse().unwrap();

        // Create test dates
        let past = DateTime::from_timestamp(1000000000, 0).unwrap().naive_utc(); // 2001-09-09
        let present = DateTime::from_timestamp(1700000000, 0).unwrap().naive_utc(); // 2023-11-14
        let future = DateTime::from_timestamp(2000000000, 0).unwrap().naive_utc(); // 2033-05-18

        // Test 1: ROA with no time constraints
        let roa_no_time = RoaEntry {
            prefix,
            asn: 64496,
            max_length: 16,
            rir: Some(Rir::ARIN),
            not_before: None,
            not_after: None,
        };
        trie.insert_roa(roa_no_time.clone());

        // Should be valid at any time
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64496, Some(past)),
            RpkiValidation::Valid
        );
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64496, Some(present)),
            RpkiValidation::Valid
        );
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64496, Some(future)),
            RpkiValidation::Valid
        );
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64496, None),
            RpkiValidation::Valid
        );

        // Test 2: ROA that's expired
        let expired_roa = RoaEntry {
            prefix,
            asn: 64497,
            max_length: 16,
            rir: Some(Rir::ARIN),
            not_before: None,
            not_after: Some(past),
        };
        trie.insert_roa(expired_roa);

        // Should be unknown after expiry (ROA exists but is outside valid time range)
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64497, Some(present)),
            RpkiValidation::Unknown
        );
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64497, None),
            RpkiValidation::Unknown
        );

        // Test 3: ROA that's not yet valid
        let future_roa = RoaEntry {
            prefix,
            asn: 64498,
            max_length: 16,
            rir: Some(Rir::ARIN),
            not_before: Some(future),
            not_after: None,
        };
        trie.insert_roa(future_roa);

        // Should be unknown before validity period (ROA exists but is outside valid time range)
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64498, Some(present)),
            RpkiValidation::Unknown
        );
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64498, None),
            RpkiValidation::Unknown
        );

        // Test 4: ROA with valid time window
        let windowed_roa = RoaEntry {
            prefix,
            asn: 64499,
            max_length: 16,
            rir: Some(Rir::ARIN),
            not_before: Some(past),
            not_after: Some(future),
        };
        trie.insert_roa(windowed_roa);

        // Should be valid within window
        assert_eq!(
            trie.validate_check_expiry(&prefix, 64499, Some(present)),
            RpkiValidation::Valid
        );
        // Should be unknown outside window (ROA exists but is outside valid time range)
        assert_eq!(
            trie.validate_check_expiry(
                &prefix,
                64499,
                Some(DateTime::from_timestamp(900000000, 0).unwrap().naive_utc())
            ),
            RpkiValidation::Unknown
        );
        assert_eq!(
            trie.validate_check_expiry(
                &prefix,
                64499,
                Some(DateTime::from_timestamp(2100000000, 0).unwrap().naive_utc())
            ),
            RpkiValidation::Unknown
        );

        // Test 5: Ensure Invalid is still returned for wrong ASN
        assert_eq!(
            trie.validate_check_expiry(&prefix, 99999, Some(present)),
            RpkiValidation::Invalid
        );
    }
}
