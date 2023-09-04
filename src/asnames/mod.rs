//! asnames is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - <https://ftp.ripe.net/ripe/asnames/asn.txt>
//!
//! # Data structure
//!
//! ```rust
//! #[derive(Debug, Clone)]
//! pub struct AsName {
//!     pub asn: u32,
//!     pub name: String,
//!     pub country: String,
//! }
//! ```
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use bgpkit_commons::asnames::{AsName, get_asnames};
//!
//! let asnames: HashMap<u32, AsName> = get_asnames().unwrap();
//! assert_eq!(asnames.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//! assert_eq!(asnames.get(&400644).unwrap().name, "BGPKIT-LLC");
//! assert_eq!(asnames.get(&400644).unwrap().country, "US");
//! ```

use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AsName {
    pub asn: u32,
    pub name: String,
    pub country: String,
}

const DATA_URL: &str = "https://ftp.ripe.net/ripe/asnames/asn.txt";

pub fn get_asnames() -> Result<HashMap<u32, AsName>> {
    let text = reqwest::blocking::get(DATA_URL)?.text()?;
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
            Some(AsName {
                asn: asn_str.parse::<u32>().unwrap(),
                name: name_str.to_string(),
                country: country_str.to_string(),
            })
        })
        .collect::<Vec<AsName>>();

    let mut asnames_map = HashMap::new();
    for asname in asnames {
        asnames_map.insert(asname.asn, asname);
    }
    Ok(asnames_map)
}
