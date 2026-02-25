# AS Information Module

**Purpose:** Autonomous System (AS) name resolution, organization mapping, and metadata lookup.

## Structure

```
src/asinfo/
├── mod.rs              # AsInfo, AsInfoBuilder, AsInfoUtils
├── as2org.rs           # CAIDA AS-to-organization mapping
├── hegemony.rs         # IIJ IHR hegemony scores
├── peeringdb.rs        # PeeringDB integration
├── population.rs       # APNIC AS population data
└── sibling_orgs.rs     # Sibling organization detection
```

## Key Types

| Type | Purpose |
|------|---------|
| `AsInfo` | Main AS data struct with name, country, optional metadata |
| `AsInfoBuilder` | Configure which data sources to load |
| `AsInfoUtils` | Loaded data container with lookup methods |

## Data Sources

| Source | File | Data |
|--------|------|------|
| RIPE NCC | `mod.rs` | AS names and countries |
| CAIDA as2org | `as2org.rs` | Organization mappings |
| APNIC | `population.rs` | User population stats |
| IIJ IHR | `hegemony.rs` | Hegemony scores |
| PeeringDB | `peeringdb.rs` | IRR as-set, long names |

## Usage Pattern

```rust
// Via BgpkitCommons
let mut commons = BgpkitCommons::new();
commons.load_asinfo_with(
    commons.asinfo_builder().with_as2org().with_peeringdb()
)?;

// Direct module access
let asinfo = AsInfoBuilder::new()
    .with_as2org()
    .build()?;
```

## Notes

- `get_preferred_name()` prioritizes: PeeringDB name > as2org org_name > default name
- Sibling detection requires as2org data loaded
- Cached data available via `load_asinfo_cached()` (BGPKIT mirror)
