# BGPKIT Commons

*This readme is generated from the library's doc comments using [cargo-readme](https://github.com/livioribeiro/cargo-readme). Please refer to the Rust docs website for the full documentation*

[![Crates.io](https://img.shields.io/crates/v/bgpkit-commons)](https://crates.io/crates/bgpkit-commons)
[![Docs.rs](https://docs.rs/bgpkit-commons/badge.svg)](https://docs.rs/bgpkit-commons)
[![License](https://img.shields.io/crates/l/bgpkit-commons)](https://raw.githubusercontent.com/bgpkit/bgpkit-commons/main/LICENSE)
[![Discord](https://img.shields.io/discord/919618842613927977?label=Discord&style=plastic)](https://discord.gg/XDaAtZsz6b)


## Overview

BGPKIT-Commons is a library for common BGP-related data and functions.

## Categories

### MRT collectors

This crate provides three functions to retrieve the full list of MRT collectors from
RouteViews and RIPE RIS:
- `get_routeviews_collectors()`
- `get_riperis_collectors()`
- `get_all_collectors()`

#### Data structure

The collectors are abstract to the following struct:
```rust
use chrono::NaiveDateTime;
use bgpkit_commons::collectors::MrtCollectorProject;
 /// MRT collector meta information
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MrtCollector {
    /// name of the collector
    pub name: String,
    /// collector project
    pub project: MrtCollectorProject,
    /// MRT data files root URL
    pub data_url: String,
    /// collector activation timestamp
    pub activated_on: NaiveDateTime,
    /// collector deactivation timestamp (None for active collectors)
    pub deactivated_on: Option<NaiveDateTime>,
    /// country where the collect runs in
    pub country: String,
}
```
where `MrtCollectorProject` is defined as:
```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MrtCollectorProject {
    RouteViews,
    RipeRis,
}
```

#### Usage example

See the following example for usage:
```rust
use bgpkit_commons::collectors::get_routeviews_collectors;
fn main() {
    println!("get route views collectors");
    let mut routeviews_collectors = get_routeviews_collectors().unwrap();
    routeviews_collectors.sort();
    let earliest = routeviews_collectors.first().unwrap();
    let latest = routeviews_collectors.last().unwrap();
    println!("\t total of {} collectors", routeviews_collectors.len());
    println!(
        "\t earliest collector: {} (activated on {})",
        earliest.name, earliest.activated_on
    );
    println!(
        "\t latest collector: {} (activated on {})",
        latest.name, latest.activated_on
    );
}
```

### AS name and country

`asnames` is a module for Autonomous System (AS) names and country lookup

Data source:
- <https://ftp.ripe.net/ripe/asnames/asn.txt>

#### Data structure

```rust
#[derive(Debug, Clone)]
pub struct AsName {
    pub asn: u32,
    pub name: String,
    pub country: String,
}
```

#### Usage example

```rust
use std::collections::HashMap;
use bgpkit_commons::asnames::{AsName, get_asnames};

fn main() {
    let asnames: HashMap<u32, AsName> = get_asnames().unwrap();
    assert_eq!(asnames.get(&3333).unwrap().name, "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)");
    assert_eq!(asnames.get(&400644).unwrap().name, "BGPKIT-LLC");
    assert_eq!(asnames.get(&400644).unwrap().country, "US");
}
```

### RPKI utilities

#### Data sources

- [Cloudflare RPKI JSON](https://rpki.cloudflare.com/rpki.json)
- [RIPC NCC RPKI historical data dump](https://ftp.ripe.net/rpki/)
    - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
    - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
    - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
    - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
    - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>

#### Usage Examples

##### Check current RPKI validation using Cloudflare RPKI portal

```rust
use std::str::FromStr;
use ipnet::IpNet;
use bgpkit_commons::rpki::{RpkiTrie, RpkiValidation};

let trie = RpkiTrie::from_cloudflare().unwrap();
let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
assert_eq!(trie.validate(&prefix, 13335), RpkiValidation::Valid);
```


##### Check RPKI validation for a given date
```rust
use std::str::FromStr;
use chrono::NaiveDate;
use ipnet::IpNet;
use bgpkit_commons::rpki::{RpkiTrie, RpkiValidation};

let rpki = RpkiTrie::from_ripe_historical(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()).unwrap();
let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
assert_eq!(rpki.validate(&prefix, 13335), RpkiValidation::Valid);
```

## Feature Flags

- `rustls`: use rustls instead of native-tls for the underlying HTTPS requests

## Built with ❤️ by BGPKIT Team

<a href="https://bgpkit.com"><img src="https://bgpkit.com/Original%20Logo%20Cropped.png" alt="https://bgpkit.com/favicon.ico" width="200"/></a>

## License

MIT
