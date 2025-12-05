# BGPKIT Commons

*This readme is generated from the library's doc comments using [cargo-readme](https://github.com/livioribeiro/cargo-readme). Please refer to the Rust docs website for the full documentation*

[![Crates.io](https://img.shields.io/crates/v/bgpkit-commons)](https://crates.io/crates/bgpkit-commons)
[![Docs.rs](https://docs.rs/bgpkit-commons/badge.svg)](https://docs.rs/bgpkit-commons)
[![License](https://img.shields.io/crates/l/bgpkit-commons)](https://raw.githubusercontent.com/bgpkit/bgpkit-commons/main/LICENSE)
[![Discord](https://img.shields.io/discord/919618842613927977?label=Discord&style=plastic)](https://discord.gg/XDaAtZsz6b)

## Overview

BGPKIT-Commons is a library for common BGP-related data and functions with a lazy-loading architecture.
Each module can be independently enabled via feature flags, allowing for minimal builds.

### Available Modules

#### [`asinfo`] - Autonomous System Information (requires `asinfo` feature)
**Load Methods**: `load_asinfo(as2org, population, hegemony, peeringdb)`, `load_asinfo_cached()`, or `load_asinfo_with(builder)`
**Access Methods**: `asinfo_get(asn)`, `asinfo_all()`, `asinfo_are_siblings(asn1, asn2)`
**Data Sources**: RIPE NCC, CAIDA as2org, APNIC population, IIJ IHR hegemony, PeeringDB
**Functionality**: AS name resolution, country mapping, organization data, population statistics, hegemony scores, sibling detection

**New in v0.10.0**: The `as2org-rs` and `peeringdb-rs` crates have been consolidated into this module.
Use [`AsInfoBuilder`](asinfo::AsInfoBuilder) for ergonomic configuration of data sources.

#### [`as2rel`] - AS Relationship Data (requires `as2rel` feature)
**Load Method**: `load_as2rel()`
**Access Methods**: `as2rel_lookup(asn1, asn2)`
**Data Sources**: BGPKIT AS relationship inference
**Functionality**: Provider-customer, peer-to-peer, and sibling relationships between ASes

#### [`bogons`] - Bogon Detection (requires `bogons` feature)
**Load Method**: `load_bogons()`
**Access Methods**: `bogons_match(input)`, `bogons_match_prefix(prefix)`, `bogons_match_asn(asn)`, `get_bogon_prefixes()`, `get_bogon_asns()`
**Data Sources**: IANA special registries (IPv4, IPv6, ASN)
**Functionality**: Detect invalid/reserved IP prefixes and ASNs that shouldn't appear in routing

#### [`countries`] - Country Information (requires `countries` feature)
**Load Method**: `load_countries()`
**Access Methods**: `country_by_code(code)`, `country_by_code3(code)`, `country_by_name(name)`, `country_all()`
**Data Sources**: GeoNames geographical database
**Functionality**: ISO country code to name mapping and geographical information

#### [`mrt_collectors`] - MRT Collector Metadata (requires `mrt_collectors` feature)
**Load Methods**: `load_mrt_collectors()`, `load_mrt_collector_peers()`
**Access Methods**: `mrt_collectors_all()`, `mrt_collectors_by_name(name)`, `mrt_collectors_by_country(country)`, `mrt_collector_peers_all()`, `mrt_collector_peers_full_feed()`
**Data Sources**: RouteViews and RIPE RIS official APIs
**Functionality**: BGP collector information, peer details, full-feed vs partial-feed classification

#### [`rpki`] - RPKI Validation (requires `rpki` feature)
**Load Methods**: `load_rpki(optional_date)`, `load_rpki_historical(date, source)`, `load_rpki_from_files(urls, source, date)`
**Access Methods**: `rpki_validate(asn, prefix)`, `rpki_validate_check_expiry(asn, prefix, timestamp)`, `rpki_lookup_by_prefix(prefix)`
**Data Sources**: Cloudflare real-time, RIPE NCC historical, RPKIviews historical
**Functionality**: Route Origin Authorization (ROA) and ASPA validation, supports multiple data sources

**New in v0.10.0**: Added RPKIviews as a historical RPKI data source with multiple collectors.
New public types [`Roa`](rpki::Roa) and [`Aspa`](rpki::Aspa) provide stable API for RPKI objects.

### Quick Start

Add `bgpkit-commons` to your `Cargo.toml`:
```toml
[dependencies]
bgpkit-commons = "0.10"
```

#### Basic Usage Pattern

All modules follow the same lazy-loading pattern:
1. Create a mutable `BgpkitCommons` instance
2. Load the data you need by calling `load_xxx()` methods
3. Access the data using the corresponding `xxx_yyy()` methods

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();

// Load bogon data
commons.load_bogons().unwrap();

// Use the data
if let Ok(is_bogon) = commons.bogons_match("23456") {
    println!("ASN 23456 is a bogon: {}", is_bogon);
}
```

#### Working with Multiple Modules

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();

// Load multiple data sources
commons.load_asinfo(false, false, false, false).unwrap();
commons.load_countries().unwrap();

// Use the data together
if let Ok(Some(asinfo)) = commons.asinfo_get(13335) {
    println!("AS13335: {} ({})", asinfo.name, asinfo.country);
}
```

#### Using the AsInfoBuilder (New in v0.10.0)

The builder pattern provides a clearer API for loading AS information:

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();

// Clear, self-documenting configuration
let builder = commons.asinfo_builder()
    .with_as2org()
    .with_peeringdb();
commons.load_asinfo_with(builder).unwrap();

// Check if two ASes are siblings (requires as2org data)
if let Ok(are_siblings) = commons.asinfo_are_siblings(13335, 132892) {
    println!("AS13335 and AS132892 are siblings: {}", are_siblings);
}
```

#### Loading Historical RPKI Data (New in v0.10.0)

Load RPKI data from multiple historical sources:

```rust
use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::rpki::{HistoricalRpkiSource, RpkiViewsCollector};
use chrono::NaiveDate;

let mut commons = BgpkitCommons::new();
let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();

// Load from RIPE NCC historical archives
commons.load_rpki_historical(date, HistoricalRpkiSource::Ripe).unwrap();

// Or load from RPKIviews collectors
let source = HistoricalRpkiSource::RpkiViews(RpkiViewsCollector::KerfuffleNet);
commons.load_rpki_historical(date, source).unwrap();

// List available files for a date
let files = commons.list_rpki_files(date, HistoricalRpkiSource::Ripe).unwrap();
```

### Feature Flags

#### Module Features
- `asinfo` - AS information with organization and population data (includes integrated as2org and peeringdb)
- `as2rel` - AS relationship data
- `bogons` - Bogon prefix and ASN detection
- `countries` - Country information lookup
- `mrt_collectors` - MRT collector metadata
- `rpki` - RPKI validation functionality (ROA and ASPA)

#### Convenience Features
- `all` (default) - Enables all modules for backwards compatibility

#### Minimal Build Example
```toml
[dependencies]
bgpkit-commons = { version = "0.10", default-features = false, features = ["bogons", "countries"] }
```

### Direct Module Access

All modules support both central access via `BgpkitCommons` and direct module access:

```rust
// Via BgpkitCommons (recommended for most use cases)
use bgpkit_commons::BgpkitCommons;
let mut commons = BgpkitCommons::new();
commons.load_bogons().unwrap();

// Direct module access (useful for standalone usage)
use bgpkit_commons::bogons::Bogons;
let bogons = Bogons::new().unwrap();
```

### Error Handling

All access methods return `Result<T>` and will return an error if the corresponding module
hasn't been loaded yet or if there are data validation issues. Error messages include guidance
on which `load_xxx()` method to call. Always call the appropriate `load_xxx()` method before accessing data.

### Data Persistence and Reloading

All loaded data is kept in memory for fast access. Use the `reload()` method to refresh
all currently loaded data sources:

```rust
let mut commons = BgpkitCommons::new();
commons.load_bogons().unwrap();

// Later, reload all loaded data
commons.reload().unwrap();
```

### What's New in v0.10.0

- **RPKIviews Historical Data**: Load historical RPKI data from RPKIviews collectors (SoborostNet, MassarsNet, AttnJp, KerfuffleNet) in addition to RIPE NCC archives
- **Crate Consolidation**: `as2org-rs` and `peeringdb-rs` functionality integrated directly into the `asinfo` module
- **AsInfoBuilder**: New builder pattern for ergonomic configuration of AS information data sources
- **Public RPKI Types**: New stable [`Roa`](rpki::Roa) and [`Aspa`](rpki::Aspa) structs for RPKI objects
- **Streaming Optimization**: RPKIviews archives are streamed efficiently without downloading entire files
- **RIPE Historical JSON**: RIPE historical data now uses JSON format for richer data including expiry timestamps

## License

MIT
