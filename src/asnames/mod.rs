//! asnames is a module for simple Autonomous System (AS) names and country lookup
//!
//! # Data source
//!
//! - RIPE NCC asnames: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - CAIDA as-to-organization mapping: <https://www.caida.org/catalog/datasets/as-organizations/>
//!
//! # Data structure
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone)]
//! pub struct AsName {
//!     pub asn: u32,
//!     pub name: String,
//!     pub country: String,
//!     pub as2org: Option<As2orgInfo>,
//!     pub population: Option<AsnPopulationData>,
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

mod hegemony;
mod population;

use crate::asnames::hegemony::HegemonyData;
use crate::asnames::population::AsnPopulationData;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsName {
    pub asn: u32,
    pub name: String,
    pub country: String,
    pub as2org: Option<As2orgInfo>,
    pub population: Option<AsnPopulationData>,
    pub hegemony: Option<HegemonyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct As2orgInfo {
    pub name: String,
    pub country: String,
    pub org_id: String,
    pub org_name: String,
}

const DATA_URL: &str = "https://ftp.ripe.net/ripe/asnames/asn.txt";

pub fn get_asnames() -> Result<HashMap<u32, AsName>> {
    info!("loading asnames from RIPE NCC...");
    let text = oneio::read_to_string(DATA_URL)?;
    info!("loading as2org data from CAIDA...");
    let as2org = as2org_rs::As2org::new(None)?;
    info!("loading ASN population data from APNIC...");
    let population = population::AsnPopulation::new()?;
    info!("loading IIJ IHR hegemony score data from BGPKIT mirror...");
    let hegemony = hegemony::Hegemony::new()?;

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
            let as2org = as2org.get_as_info(asn).map(|info| As2orgInfo {
                name: info.name.clone(),
                country: info.country_code.clone(),
                org_id: info.org_id.clone(),
                org_name: info.org_name.clone(),
            });
            let population = population.get(asn);
            Some(AsName {
                asn,
                name: name_str.to_string(),
                country: country_str.to_string(),
                as2org,
                population,
                hegemony: hegemony.get_score(asn).cloned(),
            })
        })
        .collect::<Vec<AsName>>();

    let mut asnames_map = HashMap::new();
    for asname in asnames {
        asnames_map.insert(asname.asn, asname);
    }
    Ok(asnames_map)
}
