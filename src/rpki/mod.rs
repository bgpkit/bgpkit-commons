//! rpki module maintains common functions for accessing RPKI information
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

use crate::BgpkitCommons;
use anyhow::{Result, anyhow};
pub use cloudflare::*;
use std::fmt::Display;
use std::str::FromStr;

pub struct RpkiTrie {
    pub trie: IpnetTrie<RoaEntry>,
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

    /// insert an [RoaEntry]. If old value exists, it is returned.
    pub fn insert_roa(&mut self, roa: RoaEntry) -> Option<RoaEntry> {
        self.trie.insert(roa.prefix, roa)
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
        for (p, roa) in self.trie.matches(prefix) {
            if p.contains(prefix) && roa.max_length >= prefix.prefix_len() {
                all_matches.push(*roa);
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

impl BgpkitCommons {
    pub fn rpki_lookup_by_prefix(&self, prefix: &str) -> Result<Vec<RoaEntry>> {
        if self.rpki_trie.is_none() {
            return Err(anyhow!("rpki is not loaded"));
        }

        let prefix = prefix.parse()?;

        Ok(self.rpki_trie.as_ref().unwrap().lookup_by_prefix(&prefix))
    }

    pub fn rpki_validate(&self, asn: u32, prefix: &str) -> Result<RpkiValidation> {
        if self.rpki_trie.is_none() {
            return Err(anyhow!("rpki is not loaded"));
        }
        let prefix = prefix.parse()?;
        Ok(self.rpki_trie.as_ref().unwrap().validate(&prefix, asn))
    }
}
