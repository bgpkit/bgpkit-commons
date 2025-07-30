//! rpki module maintains common functions for accessing RPKI information
//!
//! This module supports multiple Route Origin Authorizations (ROAs) for the same prefix,
//! which is common in real-world RPKI deployments where different ASNs may be authorized
//! to originate the same prefix with different maximum lengths.
//!
//! # Key Features
//!
//! - Multiple ROAs per prefix: A single prefix can have multiple valid ROAs with different ASNs
//! - Duplicate prevention: ROAs with identical (prefix, asn, max_length) are automatically deduplicated
//! - Efficient lookup: Fast prefix matching using a trie data structure
//! - Validation: Comprehensive RPKI validation against stored ROAs
//!
//! # Data sources
//!
//! - [Cloudflare RPKI JSON](https://rpki.cloudflare.com/rpki.json)
//! - [RIPE NCC RPKI historical data dump](https://ftp.ripe.net/rpki/)
//!     - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
//!     - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
//!     - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
//!     - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
//!     - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>
//! - [PeeringDB](https://www.peeringdb.com/apidocs/)

mod cloudflare;
mod ripe_historical;
// mod rpkiviews;

use chrono::{NaiveDate, NaiveDateTime};
use ipnet::IpNet;
use ipnet_trie::IpnetTrie;

use crate::errors::{load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
pub use cloudflare::*;
use std::fmt::Display;
use std::str::FromStr;

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

#[derive(Clone, Debug, Copy)]
pub struct RoaEntry {
    pub prefix: IpNet,
    pub asn: u32,
    pub max_length: u8,
    pub rir: Option<Rir>,
    pub not_before: Option<NaiveDateTime>,
    pub not_after: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
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
                        all_matches.push(*roa);
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
