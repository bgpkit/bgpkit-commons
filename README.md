# bgpkit-commons

[![crates.io](https://img.shields.io/crates/v/bgpkit-commons.svg)](https://crates.io/crates/bgpkit-commons)
[![docs.rs](https://docs.rs/bgpkit-commons/badge.svg)](https://docs.rs/bgpkit-commons)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for common BGP-related data and functions. Each data source is a
feature-gated module with lazy loading: call `load_xxx()` to fetch, then query.

## Quick start

```toml
[dependencies]
bgpkit-commons = "0.12"
```

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_bogons().unwrap();

assert!(commons.bogons_match("23456").unwrap()); // AS23456 is reserved
```

## Data sources

| Module | Feature | Source | Data |
|--------|---------|--------|------|
| `asinfo` | `asinfo` | RIPE NCC, CAIDA, APNIC, IIJ IHR, PeeringDB, RIR delegated stats, IRR | AS names, countries, org mappings, population, hegemony, IRR registrations |
| `as2rel` | `as2rel` | BGPKIT inference | AS-level provider/customer/peer relationships |
| `bogons` | `bogons` | IANA special registries | Reserved/bogon ASN and IP prefix detection |
| `countries` | `countries` | GeoNames | ISO country codes, capitals, continents, neighbors |
| `mrt_collectors` | `mrt_collectors` | RouteViews, RIPE RIS | BGP collector metadata (name, project, country, dates) |
| `peeringdb` | `peeringdb` | PeeringDB API | All 12 endpoints: networks, IXPs, facilities, orgs, carriers |
| `rpki` | `rpki` | Cloudflare, RIPE NCC, RPKIviews, RPKISPOOL | ROA and ASPA validation (real-time and historical) |
| `delegated` | `delegated` | NRO/RIR delegated stats | RIR allocation records (ASN, IPv4, IPv6) |
| `irr` | `irr` | RIPE, APNIC, ARIN, LACNIC, AFRINIC, NTTCOM, RADB, ... | RPSL aut-num, route, route6, as-set records |
| `export` | `export` | All of the above | Parquet export of every loaded data source |

## Usage

### AS information with enrichment

```rust
use bgpkit_commons::asinfo::AsInfoProfile;
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_asinfo_with_profile(AsInfoProfile::Default).unwrap();

if let Ok(Some(asinfo)) = commons.asinfo_get(13335) {
    println!("AS{}: {} ({})", asinfo.asn, asinfo.name, asinfo.country);
}
```

### Fine-grained source selection

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
let builder = commons.asinfo_builder()
    .with_as2org()
    .with_peeringdb();
commons.load_asinfo_with(builder).unwrap();

if let Ok(siblings) = commons.asinfo_are_siblings(13335, 132892) {
    println!("Sibling ASes: {siblings}");
}
```

### RPKI validation

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_rpki(None).unwrap(); // real-time Cloudflare data

let result = commons.rpki_validate(13335, "1.1.1.0/24").unwrap();
println!("RPKI: {result}"); // valid, invalid, or unknown
```

### IRR source selection

```rust
use bgpkit_commons::asinfo::{AsInfoBuilder, IrrSourceConfig};

let asinfo = AsInfoBuilder::new()
    .with_irr_sources(IrrSourceConfig::only(&["RIPE", "RADB"]).unwrap())
    .build()
    .unwrap();
```

### Bogon detection

```rust
use bgpkit_commons::BgpkitCommons;

let mut commons = BgpkitCommons::new();
commons.load_bogons().unwrap();

assert!(commons.bogons_match("10.0.0.0/8").unwrap());   // RFC 1918
assert!(commons.bogons_match_asn(65535).unwrap());       // reserved ASN
```

### PeeringDB

```rust
use bgpkit_commons::peeringdb::Peeringdb;

let pdb = Peeringdb::new().unwrap(); // all 12 endpoints
let cf = pdb.get_network(13335).unwrap();
println!("{}: {} IXP memberships", cf.name.unwrap(), cf.ix_count.unwrap());
```

### Direct module access

Modules work standalone without the `BgpkitCommons` facade:

```rust
use bgpkit_commons::bogons::Bogons;
use bgpkit_commons::countries::Countries;

let bogons = Bogons::new().unwrap();
let countries = Countries::new().unwrap();
```

## Parquet export

The `export` feature enables source-faithful Parquet export of all loaded data.
Each upstream source is written as its own file with full source fields preserved.

```toml
[dependencies]
bgpkit-commons = { version = "0.12", features = ["export"] }
```

```rust
use bgpkit_commons::BgpkitCommons;
use bgpkit_commons::export;

let mut commons = BgpkitCommons::new();
commons.load_countries().unwrap();

export::countries("./output", &commons).unwrap();
// writes ./output/countries.parquet
```

### CLI tool

The `export-cli` feature adds the `bgpkit-export` binary:

```sh
cargo install bgpkit-commons --features export-cli

bgpkit-export --output-dir ./commons-export
bgpkit-export --output-dir ./output --with-peeringdb --with-irr --with-rpki
bgpkit-export --output-dir ./output --with-asninfo-jsonl
```

Output layout (one file per source):

```
commons-export/
  manifest.json
  asn_names.parquet
  countries.parquet
  iana_bogons.parquet
  mrt_collectors.parquet
  as_relationships.parquet
  rir_delegated.parquet
  peeringdb/          # 12 endpoint tables (--with-peeringdb)
  irr/records.parquet  # RPSL records (--with-irr)
  rpki/               # ROAs + ASPAs (--with-rpki)
  asninfo.jsonl        # legacy merged output (--with-asninfo-jsonl)
```

## Feature flags

| Feature | Description |
|---------|-------------|
| `asinfo` | AS information with multi-source enrichment |
| `as2rel` | AS relationship inference data |
| `bogons` | Bogon prefix and ASN detection |
| `countries` | Country information lookup |
| `delegated` | RIR delegated-statistics parser |
| `irr` | IRR RPSL record parsing (aut-num, route, as-set) |
| `mrt_collectors` | MRT collector metadata |
| `peeringdb` | PeeringDB API data (all 12 endpoints) |
| `rpki` | RPKI validation (ROA and ASPA) |
| `all` *(default)* | Enables all data modules above |
| `export` | Parquet export of all loaded sources (adds `arrow` + `parquet`) |
| `export-cli` | Adds the `bgpkit-export` binary (adds `clap` + `tracing-subscriber`) |

Minimal build:

```toml
[dependencies]
bgpkit-commons = { version = "0.12", default-features = false, features = ["bogons", "countries"] }
```

## AsInfo loading profiles

| Profile | Sources | Use case |
|---------|---------|----------|
| `Minimum` | RIPE `asn.txt` only | Fast name/country lookup |
| `Default` | + as2org, population, hegemony, PeeringDB | Production enrichment |
| `Full` | + delegated stats, IRR (all sources, with route prefixes) | Complete dataset |

```rust
use bgpkit_commons::asinfo::AsInfoProfile;
// or build custom:
use bgpkit_commons::asinfo::AsInfoBuilder;

let builder = AsInfoBuilder::new()
    .with_as2org()
    .with_delegated()
    .with_irr();
```

## License

MIT
