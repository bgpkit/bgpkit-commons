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
**Load Method**: `load_asinfo(as2org, population, hegemony, peeringdb)` or `load_asinfo_cached()`
**Access Methods**: `asinfo_get(asn)`, `asinfo_all()`
**Data Sources**: RIPE NCC, CAIDA as2org, APNIC population, IIJ IHR hegemony, PeeringDB
**Functionality**: AS name resolution, country mapping, organization data, population statistics, hegemony scores

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
**Access Methods**: `country_by_code(code)`, country lookup by name
**Data Sources**: GeoNames geographical database
**Functionality**: ISO country code to name mapping and geographical information

#### [`mrt_collectors`] - MRT Collector Metadata (requires `mrt_collectors` feature)
**Load Methods**: `load_mrt_collectors()`, `load_mrt_collector_peers()`
**Access Methods**: `mrt_collectors_all()`, `mrt_collector_peers()`, `mrt_collector_peers_full_feed()`
**Data Sources**: RouteViews and RIPE RIS official APIs
**Functionality**: BGP collector information, peer details, full-feed vs partial-feed classification

#### [`rpki`] - RPKI Validation (requires `rpki` feature)
**Load Method**: `load_rpki(optional_date)`
**Access Methods**: `rpki_validate(prefix, asn)`
**Data Sources**: RIPE NCC historical data, Cloudflare real-time data
**Functionality**: Route Origin Authorization (ROA) validation, supports multiple ROAs per prefix

### Quick Start

Add `bgpkit-commons` to your `Cargo.toml`:
```toml
[dependencies]
bgpkit-commons = "0.8"
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
if let Some(is_bogon) = commons.bogons_match("23456") {
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

### Feature Flags

#### Module Features
- `asinfo` - AS information with organization and population data
- `as2rel` - AS relationship data
- `bogons` - Bogon prefix and ASN detection
- `countries` - Country information lookup
- `mrt_collectors` - MRT collector metadata
- `rpki` - RPKI validation functionality

#### Convenience Features
- `all` (default) - Enables all modules for backwards compatibility

#### Minimal Build Example
```toml
[dependencies]
bgpkit-commons = { version = "0.8", default-features = false, features = ["bogons", "countries"] }
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

## License

MIT
