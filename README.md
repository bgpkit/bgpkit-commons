# bgpkit-commons

## Overview

`bgpkit-commons` is a library for common BGP-related data and functions with a lazy-loading
architecture. Each module can be independently enabled via feature flags, allowing for minimal builds.

### Quick Start

Add `bgpkit-commons` to your `Cargo.toml`:

```toml
[dependencies]
bgpkit-commons = "0.10"
```

All modules follow the same pattern: create a [`BgpkitCommons`] instance, call a `load_xxx()`
method to fetch data, then use `xxx_yyy()` methods to access it.

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_bogons().unwrap();

if let Ok(is_bogon) = commons.bogons_match("23456") {
    println!("ASN 23456 is a bogon: {}", is_bogon);
}
```

### Modules

#### [`asinfo`] — Autonomous System Information

Feature: `asinfo` | Sources: RIPE NCC, CAIDA as2org, APNIC population, IIJ IHR hegemony, PeeringDB

- Load: `load_asinfo(as2org, population, hegemony, peeringdb)`, `load_asinfo_cached()`, `load_asinfo_with(builder)`
- Access: `asinfo_get(asn)`, `asinfo_all()`, `asinfo_are_siblings(asn1, asn2)`
- AS name resolution, country mapping, organization data, population statistics, hegemony scores

#### [`as2rel`] — AS Relationship Data

Feature: `as2rel` | Source: BGPKIT AS relationship inference

- Load: `load_as2rel()`
- Access: `as2rel_lookup(asn1, asn2)`
- Provider-customer, peer-to-peer, and sibling relationships between ASes

#### [`bogons`] — Bogon Detection

Feature: `bogons` | Source: IANA special registries (IPv4, IPv6, ASN)

- Load: `load_bogons()`
- Access: `bogons_match(input)`, `bogons_match_prefix(prefix)`, `bogons_match_asn(asn)`, `get_bogon_prefixes()`, `get_bogon_asns()`
- Detect invalid/reserved IP prefixes and ASNs that shouldn't appear in routing

#### [`countries`] — Country Information

Feature: `countries` | Source: GeoNames geographical database

- Load: `load_countries()`
- Access: `country_by_code(code)`, `country_by_code3(code)`, `country_by_name(name)`, `country_all()`
- ISO country code to name mapping and geographical information

#### [`mrt_collectors`] — MRT Collector Metadata

Feature: `mrt_collectors` | Sources: RouteViews and RIPE RIS official APIs

- Load: `load_mrt_collectors()`, `load_mrt_collector_peers()`
- Access: `mrt_collectors_all()`, `mrt_collectors_by_name(name)`, `mrt_collectors_by_country(country)`, `mrt_collector_peers_all()`, `mrt_collector_peers_full_feed()`
- BGP collector information, peer details, full-feed vs partial-feed classification

#### [`peeringdb`] — PeeringDB Data

Feature: `peeringdb` | Source: [PeeringDB API](https://www.peeringdb.com/api/)

- Load: `Peeringdb::new()` (all tables), `Peeringdb::new_networks_only()` (lightweight)
- Access: `get_network(asn)`, `get_ixp(ix_id)`, `get_ixp_memberships(asn)`, `lookup_ixp_prefix(prefix)`, `get_facility(fac_id)`
- Typed structs mirroring all 12 PeeringDB API endpoints: networks, internet exchanges,
  IXP prefixes, IXP membership, facilities, organizations, carriers, and more
- `PeeringdbData` is a type alias for the full `Network` struct (backward compatible)

#### [`rpki`] — RPKI Validation

Feature: `rpki` | Sources: Cloudflare (real-time), RIPE NCC historical, RPKIviews historical, RPKISPOOL historical

- Load: `load_rpki(optional_date)`, `load_rpki_historical(date, source)`, `load_rpki_from_files(urls, source, date)`
- Poll: `RpkiTrie::from_cloudflare_conditional(etag, last_modified)` returns `Ok(None)` on `304 Not Modified`
- Access: `rpki_validate(asn, prefix)`, `rpki_validate_check_expiry(asn, prefix, timestamp)`, `rpki_lookup_by_prefix(prefix)`, `rpki_lookup_aspa(customer_asn)`
- Route Origin Authorization (ROA) and ASPA validation, supports real-time and historical sources
- Poll current Cloudflare data with `RpkiTrie::from_cloudflare_conditional`, retaining the returned
  [`rpki::RpkiLoad`] validators and keeping the existing trie when the result is `Ok(None)`.
- `BgpkitCommons::reload()` performs a full reload; it does not use validators or provide an atomic
  poll-and-swap operation.

### Examples

#### Loading multiple modules

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_asinfo(false, false, false, false).unwrap();
commons.load_countries().unwrap();

if let Ok(Some(asinfo)) = commons.asinfo_get(13335) {
    println!("AS13335: {} ({})", asinfo.name, asinfo.country);
}
```

#### Using AsInfoBuilder

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
let builder = commons.asinfo_builder()
    .with_as2org()
    .with_peeringdb();
commons.load_asinfo_with(builder).unwrap();

if let Ok(are_siblings) = commons.asinfo_are_siblings(13335, 132892) {
    println!("AS13335 and AS132892 are siblings: {}", are_siblings);
}
```

#### Loading historical RPKI data

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

#### Direct module access

Modules can also be used directly without `BgpkitCommons`:

```rust
use bgpkit_commons::bogons::Bogons;
let bogons = Bogons::new().unwrap();
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `asinfo` | AS information: names, countries, organizations, population, hegemony |
| `as2rel` | AS relationship data |
| `bogons` | Bogon prefix and ASN detection |
| `countries` | Country information lookup |
| `mrt_collectors` | MRT collector metadata |
| `peeringdb` | PeeringDB API data (networks, IXPs, facilities, organizations) |
| `rpki` | RPKI validation (ROA and ASPA) |
| `all` *(default)* | Enables all modules |

For a minimal build:

```toml
[dependencies]
bgpkit-commons = { version = "0.10", default-features = false, features = ["bogons", "countries"] }
```

License: MIT
