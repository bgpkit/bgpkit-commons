//!
//! # Overview
//!
//! BGPKIT-Commons is a library for common BGP-related data and functions.
//!
//! # Categories
//!
//! ## MRT collectors
//!
//! This crate provides three functions to retrieve the full list of MRT collectors from
//! RouteViews and RIPE RIS:
//! - `get_routeviews_collectors()`
//! - `get_riperis_collectors()`
//! - `get_all_collectors()`
//!
//! ### Data structure
//!
//! The collectors are abstract to the following struct:
//! ```rust,no_run
//! use chrono::NaiveDateTime;
//! use bgpkit_commons::collectors::MrtCollectorProject;
//!  /// MRT collector meta information
//! #[derive(Debug, Clone, Eq, PartialEq)]
//! pub struct MrtCollector {
//!     /// name of the collector
//!     pub name: String,
//!     /// collector project
//!     pub project: MrtCollectorProject,
//!     /// MRT data files root URL
//!     pub data_url: String,
//!     /// collector activation timestamp
//!     pub activated_on: NaiveDateTime,
//!     /// collector deactivation timestamp (None for active collectors)
//!     pub deactivated_on: Option<NaiveDateTime>,
//!     /// country where the collect runs in
//!     pub country: String,
//! }
//! ```
//! where `MrtCollectorProject` is defined as:
//! ```rust,no_run
//! #[derive(Debug, Copy, Clone, Eq, PartialEq)]
//! pub enum MrtCollectorProject {
//!     RouteViews,
//!     RipeRis,
//! }
//! ```
//!
//! ### Usage example
//!
//! See the following example for usage:
//! ```rust
//! use bgpkit_commons::collectors::get_routeviews_collectors;
//!
//! println!("get route views collectors");
//! let mut routeviews_collectors = get_routeviews_collectors().unwrap();
//! routeviews_collectors.sort();
//! let earliest = routeviews_collectors.first().unwrap();
//! let latest = routeviews_collectors.last().unwrap();
//! println!("\t total of {} collectors", routeviews_collectors.len());
//! println!(
//!     "\t earliest collector: {} (activated on {})",
//!     earliest.name, earliest.activated_on
//! );
//! println!(
//!     "\t latest collector: {} (activated on {})",
//!     latest.name, latest.activated_on
//! );
//! ```
//!
//! ## AS name and country
//!
//! `asnames` is a module for Autonomous System (AS) names and country lookup
//!
//! ### Data sources
//! - RIPE NCC asnames: <https://ftp.ripe.net/ripe/asnames/asn.txt>
//! - CAIDA as-to-organization mapping: <https://www.caida.org/catalog/datasets/as-organizations/>
//! - APNIC AS population data: <https://stats.labs.apnic.net/cgi-bin/aspop>
//!
//! ### Data structure
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone)]
//! pub struct AsName {
//!     pub asn: u32,
//!     pub name: String,
//!     pub country: String,
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
//! ### Usage example
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
//!
//! ## Countries detailed information
//!
//! ### Data Structure
//!
//! ```rust
//! pub struct Country {
//!     /// 2-letter country code
//!     pub code: String,
//!     /// 3-letter country code
//!     pub code3: String,
//!     /// Country name
//!     pub name: String,
//!     /// Capital city
//!     pub capital: String,
//!     /// Continent
//!     pub continent: String,
//!     /// Country's top-level domain
//!     pub ltd: Option<String>,
//!     /// Neighboring countries in 2-letter country code
//!     pub neighbors: Vec<String>,
//! }
//! ```
//!
//! ### Usage Examples
//!
//! ```
//! use bgpkit_commons::countries::Countries;
//!
//! let countries = Countries::new().unwrap();
//! assert_eq!(
//!     countries.lookup_by_code("US").unwrap().name,
//!     "United States"
//! );
//! assert_eq!(countries.lookup_by_name("united states").len(), 2);
//! assert_eq!(countries.lookup_by_name("united kingdom").len(), 1);
//! ```
//!
//! ## RPKI Utilities
//!
//! ### Data sources
//!
//! - [Cloudflare RPKI JSON](https://rpki.cloudflare.com/rpki.json)
//! - [RIPC NCC RPKI historical data dump](https://ftp.ripe.net/rpki/)
//!     - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
//!     - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
//!     - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
//!     - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
//!     - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>
//! - [rpkiviews historical data dump](https://rpkiviews.org/)
//!
//! ### Usage Examples
//!
//! #### Check current RPKI validation using Cloudflare RPKI portal
//!
//! ```
//! use std::str::FromStr;
//! use ipnet::IpNet;
//! use bgpkit_commons::rpki::{RpkiTrie, RpkiValidation};
//!
//! let trie = RpkiTrie::from_cloudflare().unwrap();
//! let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
//! assert_eq!(trie.validate(&prefix, 13335), RpkiValidation::Valid);
//! ```
//!
//!
//! #### Check RPKI validation for a given date
//! ```
//! use std::str::FromStr;
//! use chrono::NaiveDate;
//! use ipnet::IpNet;
//! use bgpkit_commons::rpki::{RpkiTrie, RpkiValidation};
//!
//! let rpki = RpkiTrie::from_ripe_historical(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()).unwrap();
//! // let rpki = RpkiTrie::from_rpkiviews_historical(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()).unwrap();
//! let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
//! assert_eq!(rpki.validate(&prefix, 13335), RpkiValidation::Valid);
//! ```
//!
//! ## Bogon utilities
//!
//! We provide a utility to check if an IP prefix or an ASN is a bogon.
//!
//! ### Data sources
//!
//! IANA special registries:
//! * IPv4: https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml
//! * IPv6: https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml
//! * ASN: https://www.iana.org/assignments/iana-as-numbers-special-registry/iana-as-numbers-special-registry.xhtml
//!
//! ### Usage Examples
//!
//! ```
//! use bgpkit_commons::bogons::Bogons;
//!
//! let bogons = Bogons::new().unwrap();
//! assert!(bogons.matches_str("10.0.0.0/9"));
//! assert!(bogons.matches_str("112"));
//! assert!(bogons.is_bogon_prefix(&"2001::/24".parse().unwrap()));
//! assert!(bogons.is_bogon_asn(65535));
//! ```
//!
//! ## AS-level relationship
//!
//! `bgpkit-commons` provides access to AS-level relationship data generated by BGPKIT.
//!
//! ### Data sources
//!
//! * Raw data files available at: <https://data.bgpkit.com/as2rel/>
//!
//! ### Data Structure
//!
//! ```rust
//! pub enum AsRelationship {
//!     ProviderCustomer,
//!     CustomerProvider,
//!     PeerPeer,
//! }
//!
//! pub struct As2relBgpkitData {
//!     pub rel: AsRelationship,
//!     pub peers_count: u32,
//!     pub max_peer_count: u32,
//!}
//! ```
//!
//! ### Usage Examples
//!
//!```rust
//! let bgpkit = bgpkit_commons::as2rel::As2relBgpkit::new().unwrap();
//! let (v4_data, v6_data) = bgpkit.lookup_pair(400644, 54825);
//! assert!(v4_data.is_none());
//! assert!(v6_data.is_some());
//! assert_eq!(v6_data.unwrap().rel, bgpkit_commons::as2rel::AsRelationship::CustomerProvider);
//! ```
//!
//! ## Feature Flags
//!
//! - `rustls`: use rustls instead of native-tls for the underlying HTTPS requests
//!
//! # Commandline tool
//!
//! This crate also provides a commandline tool `bgpkit-commons` for easy access to the data and utilities.
//!
//! ## Installation
//!
//! On macOS:
//! ```sh
//! brew install bgpkit/tap/bgpkit-commons
//! ```
//!
//! On other platforms:
//! ```sh
//! cargo binstall bgpkit-commons
//! ```
//!
//! ### Export all data to JSON
//!
//! ```sh
//! bgpkit-commons export --help
//! Export data to local files
//!  
//!  Usage: bgpkit-commons export [OPTIONS]
//!  
//!  Options:
//!    -o, --output-dir <OUTPUT_DIR>  output directory [default: .]
//!    -h, --help                     Print help
//!    -V, --version                  Print version
//! ```
//!
//! # Built with ❤️ by BGPKIT Team
//!
//! <a href="https://bgpkit.com"><img src="https://bgpkit.com/Original%20Logo%20Cropped.png" alt="https://bgpkit.com/favicon.ico" width="200"/></a>

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/icon-transparent.png",
    html_favicon_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/favicon.ico"
)]

pub mod as2rel;
pub mod asnames;
pub mod bogons;
pub mod collectors;
pub mod countries;
pub mod rpki;
