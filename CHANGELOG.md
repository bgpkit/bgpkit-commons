# Changelog

All notable changes to this project will be documented in this file.

## Unreleased changes

### Dependencies

* Made `serde` a required dependency (no longer optional) to ensure all public types can be serialized/deserialized
* Added `Serialize` and `Deserialize` derives to all public structs that were missing them

## v0.9.0 - 2025-07-29

### Breaking changes

* **Error handling**: All access methods now return `Result<T>` instead of mixed `Option<T>`/`Result<T>` patterns
* **Error types**: Changed from `anyhow::Error` to structured `BgpkitCommonsError` enum with specific error variants
* **Dependencies**: Removed `anyhow` dependency completely - now using only `thiserror 2.0` for error handling

### Feature flags

* Added individual feature flags for each module (`asinfo`, `as2rel`, `bogons`, `countries`, `mrt_collectors`, `rpki`)
* Made all dependencies optional except `thiserror` - dependencies are only compiled when their respective features are
  enabled
* Added `all` convenience feature that enables all modules (set as default for backwards compatibility)
* Removed `native-tls` and `rustls` feature flags - oneio now uses rustls by default
* Updated GitHub Actions workflow to test all feature combinations

### Bug fixes

* Fixed RPKI module to properly handle multiple ROAs for the same prefix
* Added duplicate prevention for ROAs based on (prefix, asn, max_length) triplet
* Enhanced RPKI module documentation with multiple ROAs support details

### Code improvements

* Cleaned up `lib.rs` by using full paths instead of feature-gated imports
* Enhanced CI testing with comprehensive feature combination validation
* Significantly improved lib.rs documentation with comprehensive usage examples
* Added feature flag documentation and minimal build examples
* Enhanced module descriptions with clear feature requirements
* Updated lib.rs with detailed functionality descriptions including load methods, access methods, data sources, and
  capabilities for each module
* Standardized error handling across all modules - all access methods now return `Result<T>` with clear error messages
  instead of mixed `Option<T>` and `Result<T>` patterns
* Unified error message format across all modules with consistent "Data not loaded. Call load_xxx() first." pattern
* Added LazyLoadable trait interface for consistent reloading and status checking across all data modules
* Added loading_status() method to BgpkitCommons for inspecting which modules are currently loaded
* Replaced anyhow with thiserror for structured error handling - introduced BgpkitCommonsError with specific error types
  for module loading failures, data source errors, and invalid formats
* Added comprehensive error constants and helper methods for consistent error creation across modules
* Eliminated all anyhow! macro calls in favor of structured error types with specific context and guidance
* Added From implementations for common parsing errors (ParseIntError, ParseFloatError) to support automatic error
  conversion
* Successfully removed anyhow dependency completely - now using only thiserror 2.0 for all error handling

## v0.8.2 - 2025-06-06

### Hot fix

* Update `oneio` to `0.18.2` to fix potential build issue related to `rustls_sys` crate

## v0.8.1 - 2025-06-01

### Highlights

* Added support for loading previously generated and cached AS information directly from BGPKIT cache files.
    - Introduced `get_asinfo_map_cached()` function and `BgpkitCommons::load_asinfo_cached()` method for fast, offline
      loading of AS info.
    - Added examples in the documentation for using the cached AS info.
* Improved AS name resolution:
    - New `get_preferred_name()` method for `AsInfo` struct, prioritizing PeeringDB, as2org, then default name.
* Enhanced PeeringDB integration:
    - Added `website` field to PeeringDB data.
* Dependency updates:
    - Bumped `peeringdb-rs` to `0.1.1` and `oneio` to `0.18.1` with new features.
    - Cleaned up unused dependencies.
* Documentation improvements for new features and updated code examples.
* Added and improved integration tests for cached AS info loading and preferred name resolution.
* Removed outdated or redundant test code.

### Other changes

* Updated `README.md` and crate docs to reference version 0.8.
* Minor internal refactoring and code cleanup.

## v0.8.0 - 2025-05-27

### Highlights

* add support for loading PeeringDB (https://www.peeringdb.com) data for `asinfo` module
    * it uses the [`peeringdb-rs`][peeringdb-rs] module for loading the data
    * users should supply a `PEERINGDB_API_KEY` when using this feature to avoid frequent rate limiting

The PeeringDB data for the `asinfo` module is simplified to contain only the following fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeeringdbData {
    pub asn: u32,
    pub name: Option<String>,
    pub name_long: Option<String>,
    pub aka: Option<String>,
    pub irr_as_set: Option<String>,
}
```

### Breaking change

The `commons.load_asinfo()` now takes four arguments, adding a new `load_peeringdb` boolean argument at the end.

For getting all other data, including organization and IX information, please check out the [
`peeringdb-rs`][peeringdb-rs] crate.

[peerindb-rs][https://github.com/bgpkit/peeringdb-rs]

## v0.7.4 - 2025-04-04

### Highlights

* Update `as2org-rs` crate to `v1.0.0` which fixes an issue of latin-1 encoding for org names

## v0.7.3 - 2024-10-31

### Highlights

* add MRT collector peers information
    * struct exposed as `crate::mrt_collectors::MrtCollectorPeer`
    * fetch data by calling `commons.load_mrt_collector_peers()` first
    * access all peers by calling `commons.mrt_collector_peers()`
    * access full-feed peers only by calling `commons.mrt_collector_peers_full_feed()`

Example usage:

```rust
use bgpkit_commons::BgpkitCommons;
fn main() {
    let mut commons = BgpkitCommons::new();
    commons.load_mrt_collector_peers().unwrap();
    let peers = commons.mrt_collector_peers();
    for peer in peers {
        println!("{:?}", peer);
    }
    let full_feed_peers = commons.mrt_collector_peers_full_feed();
    for peer in full_feed_peers {
        println!("{:?}", peer);
    }
}
```

## v0.7.2 - 2024-10-11

### Highlights

* allow exporting all bogon prefixes and asns
* update cloudflare RPKI data parsing, also added ASPA data
    * added examples/list_aspas.rs to demonstrate how to list all ASPAs

## v0.7.1 - 2024-10-03

### Highlights

* add new asinfo_all() function to return information for all ASNs in a single call

### Other changes

* improve documentation
* improve ci testing workflow
* add integration tests
* update dependencies

## v0.7.0 -2024-07-11

* consolidate all functionalities into a single `BgpkitCommons` instance

Example usage:

```rust
use bgpkit_commons::BgpkitCommons;
let mut bgpkit = BgpkitCommons::new();
bgpkit.load_bogons().unwrap();
assert!(bgpkit.bogons_match("23456").unwrap());
```

## v0.6.0 - 2024-06-26

* [Added `bogons` module](https://github.com/bgpkit/bgpkit-commons/pull/12) to check if an IP prefix or an ASN is a
  bogon
* [Added `as2rel` module](https://github.com/bgpkit/bgpkit-commons/pull/17) to provide access to AS-level relationship
  data generated by BGPKIT
* [Added APNIC population data](https://github.com/bgpkit/bgpkit-commons/pull/14) to `asnames` module
* [Added CAIDA `as2org` data](https://github.com/bgpkit/bgpkit-commons/pull/13) to `asnames` module
* [Added IIJ IHR Hegemony score](https://github.com/bgpkit/bgpkit-commons/pull/15) to `asnames` module

## v0.5.2 - 2024-03-20

### Highlights

* update `oneio` to `0.16.5` to fix route-views collector API issue

## v0.5.1 - 2024-03-20

### Highlights

* add new `bgpkit-commons` binary with `export` subcommand to export all data to JSON files
* replace `reqwest` with `oneio` as the default HTTP client

## v0.5.0 - 2024-01-30

### Breaking changes

- switch to `rustls` as the default TLS backend
    - users can still opt-in to use `native-tls` by specifying `default-features = false` and use `native-tls` feature
      flag
