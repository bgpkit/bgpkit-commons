# BGPKIT Commons

*This readme is generated from the library's doc comments using [cargo-readme](https://github.com/livioribeiro/cargo-readme). Please refer to the Rust docs website for the full documentation*

[![Crates.io](https://img.shields.io/crates/v/bgpkit-commons)](https://crates.io/crates/bgpkit-commons)
[![Docs.rs](https://docs.rs/bgpkit-commons/badge.svg)](https://docs.rs/bgpkit-commons)
[![License](https://img.shields.io/crates/l/bgpkit-commons)](https://raw.githubusercontent.com/bgpkit/bgpkit-commons/main/LICENSE)
[![Discord](https://img.shields.io/discord/919618842613927977?label=Discord&style=plastic)](https://discord.gg/XDaAtZsz6b)

## Overview

BGPKIT-Commons is a library for common BGP-related data and functions.

It provides the following modules:
- `mrt_collectors`: public RouteViews and RIPE RIS MRT mrt_collectors information extracted from their official APIs
- `asinfo`: Autonomous System (AS) information and country lookup
- `countries`: country code to name and other information lookup
- `rpki`: RPKI validation data. Historical data from RIPE NCC and real-time data from Cloudflare
- `bogons`: IP prefix and ASN bogon lookup
- `as2rel`: AS-level relationship data, generated by BGPKIT

### Basic Usage

Add `bgpkit-commons` to your `Cargo.toml`'s `dependencies` section:
```toml
bgpkit-commons = "0.8"
```

`bgpkit-commons` is designed to load only the data you need. Here is an example of checking if an ASN is a bogon ASN:

```rust
use bgpkit_commons::BgpkitCommons;

let mut bgpkit = BgpkitCommons::new();
bgpkit.load_bogons().unwrap();
assert!(bgpkit.bogons_match("23456").unwrap());
```

The common steps include:
1. create a mutable `BgpkitCommons` instance
2. load the data you need by calling `bgpkit.load_xxx()` functions
3. use the data by calling the corresponding functions, named as `bgpkit.xxx_yyy()`

For detailed usages, please refer to the module documentation.

### Feature Flags

- `rustls` (default): use rustls instead of native-tls for the underlying HTTPS requests
- `native-tls`: use native-tls as the backend

## License

MIT
