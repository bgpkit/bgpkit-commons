//! IRR (Internet Routing Registry) data support.
//!
//! This module provides tools for loading and parsing IRR data in RPSL format
//! from bulk database dumps published by IRR registries worldwide.
//!
//! # Design
//!
//! The module is a **library-level toolkit**, not an application. It provides:
//!
//! - [`sources`] — a registry of known IRR databases with their dump URLs,
//!   transports (HTTPS/FTP), and publication formats (split files vs whole-DB).
//!   Use [`sources::all_sources`] to discover everything, or
//!   [`sources::default_sources`] for a curated default set.
//! - [`types`] — typed Rust structs for the supported RPSL object types
//!   ([`AutNum`], [`Route`], [`AsSet`], [`RouteSet`], [`Mntner`], [`Organisation`]).
//! - [`stream`] — a streaming parser that reads gzipped RPSL dump files via
//!   oneio, splits text into per-object chunks, and extracts typed objects
//!   using the [`rpsl-rs`](https://crates.io/crates/rpsl-rs) parser.
//!
//! Downstream applications (e.g. `asninfo`) use these building blocks to build
//! indexes, resolve as-sets, or merge data across registries.
//!
//! # Supported Object Types
//!
//! | RPSL type | Struct | Key fields |
//! |-----------|--------|------------|
//! | `aut-num` | [`AutNum`] | ASN, as-name, descr, source |
//! | `route` / `route6` | [`Route`] | prefix, origin ASN, source |
//! | `as-set` | [`AsSet`] | name, AS members, set members (recursive) |
//! | `route-set` | [`RouteSet`] | name, prefix members, set members (recursive) |
//! | `mntner` | [`Mntner`] | name, auth methods, notification emails |
//! | `organisation` | [`Organisation`] | ID, name, address, country, abuse contact |
//!
//! # Example
//!
//! ```rust,no_run
//! use bgpkit_commons::irr;
//! use bgpkit_commons::irr::sources::default_sources;
//! use bgpkit_commons::irr::types::IrrObjectType;
//!
//! // Parse aut-num objects from all default sources
//! for source in default_sources() {
//!     for dump_url in source.dump_urls(IrrObjectType::AutNum) {
//!         println!("Loading {} from {} ({})",
//!             source.name, dump_url.url, dump_url.transport);
//!
//!         irr::stream::parse_dump(&dump_url, |obj| {
//!             if let irr::types::IrrObject::AutNum(a) = obj {
//!                 println!("AS{}: {} ({})", a.asn, a.as_name, a.source);
//!             }
//!         }).unwrap();
//!     }
//! }
//! ```

pub mod extract;
pub mod sources;
pub mod stream;
pub mod types;

pub use sources::{
    DumpFormat, IrrDumpUrl, IrrSource, Transport, all_sources, default_sources, source_by_name,
};
pub use stream::{ParseStats, parse_dump, parse_dump_from_reader};
pub use types::{AsSet, AutNum, IrrObject, IrrObjectType, Mntner, Organisation, Route, RouteSet};
