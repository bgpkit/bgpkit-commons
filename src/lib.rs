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
//! ```no_run
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
//! ```no_run
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
//! fn main() {
//!     println!("get route views collectors");
//!     let mut routeviews_collectors = get_routeviews_collectors().unwrap();
//!     routeviews_collectors.sort();
//!     let earliest = routeviews_collectors.first().unwrap();
//!     let latest = routeviews_collectors.last().unwrap();
//!     println!("\t total of {} collectors", routeviews_collectors.len());
//!     println!(
//!         "\t earliest collector: {} (activated on {})",
//!         earliest.name, earliest.activated_on
//!     );
//!     println!(
//!         "\t latest collector: {} (activated on {})",
//!         latest.name, latest.activated_on
//!     );
//! }
//! ```
//!
//! ## AS name and country
//!
//! `asnames` is a module for Autonomous System (AS) names and country lookup
//!
//! Data source:
//! - <https://ftp.ripe.net/ripe/asnames/asn.txt>
//!
//! ### Data structure
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
//! ### Usage example
//!
//! ```rust
//! use std::collections::HashMap;
//! use bgpkit_commons::asnames::{AsName, get_asnames};
//!
//! fn main() {
//!     let asnames: HashMap<u32, AsName> = get_asnames().unwrap();
//!     assert_eq!(asnames.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
//!     assert_eq!(asnames.get(&400644).unwrap().name, "BGPKIT-LLC");
//!     assert_eq!(asnames.get(&400644).unwrap().country, "US");
//! }
//! ```
//! # Built with ❤️ by BGPKIT Team
//!
//! <a href="https://bgpkit.com"><img src="https://bgpkit.com/Original%20Logo%20Cropped.png" alt="https://bgpkit.com/favicon.ico" width="200"/></a>

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/icon-transparent.png",
    html_favicon_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/favicon.ico"
)]

pub mod asnames;
pub mod collectors;
