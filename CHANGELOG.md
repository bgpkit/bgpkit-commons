# Changelog

All notable changes to this project will be documented in this file.

## v0.10.0 - 2025-12-05

### RPKIviews Historical Data Support

* **Added RPKIviews as a historical RPKI data source**: Users can now load historical RPKI data from RPKIviews collectors in addition to RIPE NCC archives
    - New `RpkiViewsCollector` enum with four collectors: SoborostNet (default), MassarsNet, AttnJp, and KerfuffleNet
    - Added `RpkiTrie::from_rpkiviews(collector, date)` method for loading from a specific collector
    - Added `RpkiTrie::from_rpkiviews_file(url, date)` and `from_rpkiviews_files(urls, date)` for loading from specific archive URLs
    - Added `list_rpkiviews_files(collector, date)` function to discover available archives for a given date
    - New `HistoricalRpkiSource` enum to explicitly select between RIPE and RPKIviews sources

* **Streaming optimization for .tgz archives**: RPKIviews archives are streamed efficiently without downloading the entire file
    - `rpki-client.json` is located at position 3-4 in the archive, allowing early termination after ~80MB instead of downloading 300+ MB
    - New `extract_file_from_tgz(url, target_path)` function for streaming extraction of specific files
    - New `list_files_in_tgz(url, max_entries)` function for listing archive contents with early termination
    - New `tgz_contains_file(url, target_path)` function for checking file existence
    - Uses `reqwest` for HTTP streaming and external `gunzip` for decompression
    - Test completion time reduced from several minutes to ~8 seconds

* **Unified rpki-client JSON parsing**: Extracted shared parsing logic for rpki-client JSON format
    - New internal `rpki_client.rs` module with `RpkiClientData` struct and robust deserializers
    - Handles variations in ASN formats (numeric `12345` vs string `"AS12345"`)
    - Handles variations in ASPA field names (`customer_asid` vs `customer`)
    - Handles provider arrays as both numbers and strings
    - Used by Cloudflare, RIPE historical, and RPKIviews sources

* **Public ROA and ASPA structs**: Added stable public API types
    - New `Roa` struct with fields: `prefix`, `asn`, `max_length`, `not_before`, `not_after`
    - New `Aspa` struct with fields: `customer_asn`, `providers`
    - Internal rpki-client format structs are now `pub(crate)` only

* **Updated RIPE historical to use JSON format**: Changed from CSV to `output.json.xz` for consistency
    - Requires `xz` feature in oneio (now enabled by default for rpki feature)
    - Provides richer data including expiry timestamps

* **New BgpkitCommons methods**:
    - `load_rpki_historical(source, date)` - Load historical RPKI data from specified source
    - `list_rpki_files(source, date)` - List available RPKI files for a date from specified source
    - `load_rpki_from_files(urls, date)` - Load and merge RPKI data from multiple file URLs

* **New example**: Added `examples/rpki_historical.rs` demonstrating historical RPKI data loading

* **Updated example**: `examples/list_aspas.rs` now counts ASPA objects for first day of years 2020-2025

### Dependencies

* Added `reqwest` (with blocking feature) for HTTP streaming
* Added `tar` crate for reading tar archives
* Enabled `xz` feature in `oneio` for RIPE historical JSON support

### Crate Consolidation

* **Migrated `as2org-rs` into bgpkit-commons**: The CAIDA AS-to-Organization mapping functionality previously provided by the external `as2org-rs` crate has been fully integrated into the `asinfo` module
    - New `src/asinfo/as2org.rs` module provides `As2org` struct with `new()`, `get_as_info()`, `get_siblings()`, and `are_siblings()` methods
    - Removed external `as2org-rs` dependency from Cargo.toml
    - Single codebase simplifies maintenance and patch application

* **Migrated `peeringdb-rs` into bgpkit-commons**: The PeeringDB API access functionality previously provided by the external `peeringdb-rs` crate has been fully integrated into the `asinfo` module
    - Updated `src/asinfo/peeringdb.rs` with full PeeringDB API client implementation
    - Includes `PeeringdbNet` struct and `load_peeringdb_net()` function for direct API access
    - Removed external `peeringdb-rs` dependency from Cargo.toml

* **Updated feature flags**: The `asinfo` feature now uses `regex` instead of external crate dependencies
    - Before: `asinfo = ["as2org-rs", "peeringdb-rs", "oneio", "serde_json", "tracing", "chrono"]`
    - After: `asinfo = ["oneio", "serde_json", "tracing", "chrono", "regex"]`

### API Improvements

* **AsInfoBuilder**: Added a new builder pattern for loading AS information with specific data sources
    - New `AsInfoBuilder` struct with fluent API methods: `with_as2org()`, `with_population()`, `with_hegemony()`, `with_peeringdb()`, `with_all()`
    - Added `asinfo_builder()` method to `BgpkitCommons` for creating builders
    - Added `load_asinfo_with(builder)` method to `BgpkitCommons` for loading with builder configuration
    - The existing `load_asinfo(bool, bool, bool, bool)` method is preserved for backward compatibility

**Before (confusing boolean parameters):**
```rust
commons.load_asinfo(true, false, true, false)?;
```

**After (clear builder pattern):**
```rust
let builder = commons.asinfo_builder()
    .with_as2org()
    .with_hegemony();
commons.load_asinfo_with(builder)?;
```

### Public API Enhancements

* **asinfo module**: Added `PeeringdbData` to public exports for direct module access
* All modules now consistently support both:
    - Central access via `BgpkitCommons` instance
    - Direct module access (e.g., `bgpkit_commons::bogons::Bogons::new()`)

### Testing Improvements

* **Comprehensive as2org module tests**: Added extensive unit tests for the migrated CAIDA AS-to-Organization functionality
    - JSON deserialization tests for `As2orgJsonOrg` and `As2orgJsonAs` structures
    - Tests for optional fields and default values
    - `As2orgAsInfo` struct creation and serialization round-trip tests
    - `fix_latin1_misinterpretation` function tests for edge cases
    - Integration tests (ignored by default) for `As2org::new()`, `get_as_info()`, `get_siblings()`, and `are_siblings()` methods

* **Comprehensive peeringdb module tests**: Added extensive unit tests for the migrated PeeringDB functionality
    - `PeeringdbData` struct creation, serialization, and deserialization tests
    - `PeeringdbNet` struct tests with all optional fields
    - `PeeringdbNetResponse` API response deserialization tests
    - `Peeringdb` struct tests for `get_data()`, `contains()`, `len()`, `is_empty()`, and `get_all_asns()` methods
    - Empty database edge case tests
    - Integration tests (ignored by default) for live API access

* **New Peeringdb helper methods**: Added utility methods to the `Peeringdb` struct for better usability
    - `len()`: Get the number of networks in the database
    - `is_empty()`: Check if the database is empty
    - `contains(asn)`: Check if an ASN exists in PeeringDB
    - `get_all_asns()`: Get all ASNs in the database

## v0.9.6 - 2025-10-29

### Maintenance

* Update dependencies to better handle rustls crypto providers:
    - peeringdb-rs to 0.1.3
    - oneio to 0.20.0
    - as2org-rs to 1.1.1

## v0.9.5 - 2025-10-29

### Fix CAIDA as2org data loading issue

* Update `as2org-rs` to `1.1.0` to fix CAIDA as2org data loading issue.

### Maintenance

* Update dependencies

## v0.9.4 - 2025-09-09

### Hot-fix

* Remove `let-chain` coding style to maintain compatibility with older Rust versions.

## v0.9.3 - 2025-09-09

### Maintenance

* Updated `peeringdb-rs` to `0.1.2` to address 403 error from PeeringDB API
* Updated `oneio` to `0.19.0`

### Code quality

* Addressed clippy warnings across the codebase with focus on `asinfo` module.
* Refactored `asinfo_are_siblings` to use if-let chains and Option combinators, removing unnecessary `unwrap()` calls.
* Simplified conditional logic (collapsible if) for better readability and safety.

## v0.9.2 - 2025-07-31

### Features

* **RPKI expiry support**: Added support for Cloudflare RPKI ROA expiry timestamps
    - Added `expires` field to `CfRoaEntry` structure for Cloudflare RPKI data
    - ROA expiry timestamps are now mapped to `not_after` field in `RoaEntry`
    - Added `validate_check_expiry()` method to `RpkiTrie` for time-aware validation
    - Added `rpki_validate_check_expiry()` method to `BgpkitCommons` for expiry-aware validation
    - Expired or not-yet-valid ROAs now return `Unknown` instead of `Invalid` (correct RPKI behavior)

### Bug fixes

* Fixed typo in `rpki_validate()` method (`vapidate` → `validate`)

### Documentation

* **RPKI documentation**: Added module documentation covering:
    - Data structures and validation process explanation
    - Usage examples for both real-time (Cloudflare) and historical (RIPE) data sources
    - Performance considerations and error handling guidance
    - Multiple ROAs per prefix handling examples
* Added `no_run` attribute to RPKI documentation examples to prevent timeouts during doc tests

### Testing

* Added unit tests for expiry checking functionality
* Added manual integration test for Cloudflare RPKI data loading with expiry validation
    - Run with: `cargo test --release --features rpki test_cloudflare_rpki_expiry_loading -- --ignored --nocapture`

## v0.9.1 - 2025-07-31

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
