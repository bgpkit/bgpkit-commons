//! # Module: bogons
//!
//! This module provides functions to detect whether some given prefix or ASN is a bogon ASN.
//!
//! We obtain the bogon ASN and prefixes data from IANA's special registries:
//! * IPv4: <https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml>
//! * IPv6: <https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml>
//! * ASN: <https://www.iana.org/assignments/iana-as-numbers-special-registry/iana-as-numbers-special-registry.xhtml>
//!
//! The simplest way to check bogon is to provide a &str:
//! ```
//! let bogons = bgpkit_commons::bogons::Bogons::new().unwrap();
//! assert!(bogons.matches_str("10.0.0.0/9"));
//! assert!(bogons.matches_str("112"));
//! assert!(bogons.is_bogon_prefix(&"2001::/24".parse().unwrap()));
//! assert!(bogons.is_bogon_asn(65535));
//! ```
mod asn;
mod prefix;
mod utils;

use crate::bogons::asn::load_bogon_asns;
use crate::bogons::prefix::load_bogon_prefixes;
use crate::errors::{data_sources, load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
pub use asn::BogonAsn;
use ipnet::IpNet;
pub use prefix::BogonPrefix;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bogons {
    pub prefixes: Vec<BogonPrefix>,
    pub asns: Vec<BogonAsn>,
}

impl Bogons {
    pub fn new() -> Result<Self> {
        Ok(Bogons {
            prefixes: load_bogon_prefixes()?,
            asns: load_bogon_asns()?,
        })
    }

    /// Check if a given string matches a bogon prefix or ASN.
    pub fn matches_str(&self, s: &str) -> bool {
        match s.parse::<IpNet>() {
            Ok(ip) => self.is_bogon_prefix(&ip),
            Err(_) => match s.parse::<u32>() {
                Ok(asn) => self.is_bogon_asn(asn),
                Err(_) => false,
            },
        }
    }

    /// Check if a given IP prefix is a bogon prefix.
    pub fn is_bogon_prefix(&self, prefix: &IpNet) -> bool {
        self.prefixes
            .iter()
            .any(|bogon_prefix| bogon_prefix.matches(prefix))
    }

    /// Check if a given ASN is a bogon ASN.
    pub fn is_bogon_asn(&self, asn: u32) -> bool {
        self.asns.iter().any(|bogon_asn| bogon_asn.matches(asn))
    }
}

impl LazyLoadable for Bogons {
    fn reload(&mut self) -> Result<()> {
        *self = Bogons::new().map_err(|e| {
            BgpkitCommonsError::data_source_error(data_sources::IANA, e.to_string())
        })?;
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        !self.prefixes.is_empty() && !self.asns.is_empty()
    }

    fn loading_status(&self) -> &'static str {
        if self.is_loaded() {
            "Bogons data loaded"
        } else {
            "Bogons data not loaded"
        }
    }
}

impl BgpkitCommons {
    pub fn bogons_match(&self, s: &str) -> Result<bool> {
        match &self.bogons {
            Some(b) => Ok(b.matches_str(s)),
            None => Err(BgpkitCommonsError::module_not_loaded(
                modules::BOGONS,
                load_methods::LOAD_BOGONS,
            )),
        }
    }

    pub fn bogons_match_prefix(&self, prefix: &str) -> Result<bool> {
        let prefix: IpNet = prefix.parse().map_err(|e: ipnet::AddrParseError| {
            BgpkitCommonsError::invalid_format("IP prefix", prefix, e.to_string())
        })?;
        match &self.bogons {
            Some(b) => Ok(b.is_bogon_prefix(&prefix)),
            None => Err(BgpkitCommonsError::module_not_loaded(
                modules::BOGONS,
                load_methods::LOAD_BOGONS,
            )),
        }
    }

    pub fn bogons_match_asn(&self, asn: u32) -> Result<bool> {
        match &self.bogons {
            Some(b) => Ok(b.is_bogon_asn(asn)),
            None => Err(BgpkitCommonsError::module_not_loaded(
                modules::BOGONS,
                load_methods::LOAD_BOGONS,
            )),
        }
    }

    /// Get all bogon prefixes.
    pub fn get_bogon_prefixes(&self) -> Result<Vec<BogonPrefix>> {
        match &self.bogons {
            Some(b) => Ok(b.prefixes.clone()),
            None => Err(BgpkitCommonsError::module_not_loaded(
                modules::BOGONS,
                load_methods::LOAD_BOGONS,
            )),
        }
    }

    /// Get all bogon ASNs.
    pub fn get_bogon_asns(&self) -> Result<Vec<BogonAsn>> {
        match &self.bogons {
            Some(b) => Ok(b.asns.clone()),
            None => Err(BgpkitCommonsError::module_not_loaded(
                modules::BOGONS,
                load_methods::LOAD_BOGONS,
            )),
        }
    }
}
