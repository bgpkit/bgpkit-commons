//!
//! # Overview
//!
//! BGPKIT-Commons is a library for common BGP-related data and functions.
//!
//! # Categories
//!
//! ## MRT Collectors
//!
//! This crate provides three functions to retrive the full list of MRT collectors from
//! RouteViews and RIPE RIS:
//! - `get_routeviews_collectors()`
//! - `get_riperis_collectors()`
//! - `get_all_collectors()`
//!
//! The collectors is abstract to the following struct:
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
//! # Built with ❤️ by BGPKIT Team
//!
//! <a href="https://bgpkit.com"><img src="https://bgpkit.com/Original%20Logo%20Cropped.png" alt="https://bgpkit.com/favicon.ico" width="200"/></a>

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/icon-transparent.png",
    html_favicon_url = "https://raw.githubusercontent.com/bgpkit/assets/main/logos/favicon.ico"
)]

pub mod collectors;
