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
//! - [`stream`] — validated dump fetching plus source-faithful streaming
//!   parsing, with compatibility conversion to typed objects through
//!   [`rpsl-rs`](https://crates.io/crates/rpsl-rs).
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
//! use bgpkit_commons::irr::{IrrObjectType, fetch, parse_reader, source_by_name};
//!
//! let source = source_by_name("RIPE").unwrap();
//! let reader = fetch(&source, IrrObjectType::AutNum).unwrap();
//! let format = reader.dump_url.format;
//! for record in parse_reader(reader, format) {
//!     let record = record.unwrap();
//!     println!("{} with {} attributes", record.object_type, record.attributes.len());
//! }
//! ```

pub mod extract;
pub mod sources;
pub mod stream;
pub mod types;

pub use sources::{
    DumpFormat, IrrDumpUrl, IrrSource, Transport, all_sources, default_sources, source_by_name,
    sources_by_name,
};
pub use stream::{IrrReader, ParseStats, fetch, parse_dump, parse_dump_from_reader, parse_reader};
pub use types::{
    AsSet, AutNum, IrrAttribute, IrrObject, IrrObjectType, IrrRecord, Mntner, Organisation, Route,
    RouteSet,
};
