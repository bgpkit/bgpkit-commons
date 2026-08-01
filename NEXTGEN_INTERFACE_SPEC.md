# BGPKIT Commons Source and AsInfo Interface Specification

**Status:** Approved and implemented on this branch
**Baseline:** PR #39 head `e3f9857` (`feat/irr-data-support`)

## 1. Scope

This refactor has three goals:

1. Separate source fetching/parsing from data serving and enrichment. Source modules return source records without choosing preferred values or modifying another source's data.
2. Add two source-scoped fields, `delegated` and `irr`, to `AsInfo`. Do not otherwise redesign the `AsInfo` record, and omit enrichment fields from serialized output when they are absent.
3. Expose three AsInfo loading profiles: `Minimum`, `Default`, and `Full`.

This is a focused cleanup of PR #39. It is not a crate-wide snapshot, provenance, transport, caching, async, or error-model redesign.

## 2. Two layers

### 2.1 Source layer

A source module is responsible for:

- describing known sources and dump URLs;
- fetching a selected source artifact;
- parsing a caller-provided reader;
- returning records that preserve the source's values;
- reporting fetch and parse errors.

A source module must not:

- pick a preferred value across registries;
- merge one registry's object into another registry's object;
- replace an AsInfo name or country;
- silently ignore an explicitly requested source;
- serialize an application-specific output format.

Fetching and parsing are separate entry points. Parsing must work with a local reader and must not perform network access.

### 2.2 Serving/product layer

`asinfo` is the serving and enrichment layer. It chooses which source modules to load, indexes their records by ASN, and constructs `AsInfo` values according to a profile.

All cross-source policy stays in `asinfo`, including:

- which sources a profile enables;
- how IRR records are grouped by ASN and registry;
- how delegated-statistics records are projected onto an ASN;
- whether route and route6 objects are included;
- existing preferred-name behavior.

The existing `BgpkitCommons` façade and `AsInfoBuilder` remain. This refactor does not introduce a new universal `Snapshot` API.

## 3. IRR source API

PR #39 already provides the source catalog, dump fetching, RPSL parsing, and typed IRR objects. The refactor should preserve that work while making the source/product boundary explicit.

### 3.1 Parsing

The canonical parser accepts a reader and yields source records:

```rust
pub fn parse_reader<R: Read>(
    reader: R,
    format: DumpFormat,
) -> impl Iterator<Item = Result<IrrRecord, IrrError>>;
```

`IrrRecord` represents one RPSL object without AsInfo policy:

```rust
pub struct IrrRecord {
    pub object_type: String,
    pub attributes: Vec<IrrAttribute>,
}

pub struct IrrAttribute {
    pub name: String,
    pub value: String,
}
```

Requirements:

- preserve object order;
- preserve attribute order and repeated attributes;
- retain unsupported object classes instead of dropping them;
- return malformed objects as iterator errors instead of only logging and skipping them;
- preserve the RPSL `source:` attribute as an ordinary source value;
- do not attach AsInfo meaning to any attribute.

The existing PR #39 types (`AutNum`, `Route`, `AsSet`, `RouteSet`, `Mntner`, and `Organisation`) become typed conversions or views over `IrrRecord`. Existing callback APIs may remain as compatibility wrappers.

Exact original bytes and a crate-wide provenance framework are not required by this refactor.

### 3.2 Fetching

Fetching selects and opens a dump; parsing remains reusable independently:

```rust
pub fn fetch(source: &IrrSource, object_type: IrrObjectType) -> Result<IrrReader>;
```

Requirements:

- the caller explicitly selects the source and object type;
- an unknown requested source returns an error;
- fetch does not merge registries;
- fetch does not create or modify `AsInfo`;
- whole-database dumps are fetched once when several object types use the same URL.

The current source catalog and HTTP/FTP support from PR #39 remain in scope. No new async API or generic transport framework is required.

### 3.3 Source selection

Keep the useful presets:

- `all_sources()` — every known source and the default used by `with_irr()` and `Full`;
- `default_sources()` — an optional curated subset for callers that explicitly choose it;
- explicit source selection for callers that need a subset.

Explicit names must be validated. A typo must not silently produce an empty selection.

The AsInfo builder exposes both default-all and explicit-subset loading:

```rust
// All catalogued IRR sources.
AsInfoBuilder::new().with_irr();

// A caller-selected subset; invalid names return an error.
let sources = IrrSourceConfig::only(&["RIPE", "RADB"])?;
AsInfoBuilder::new().with_irr_sources(sources);
```

Source selection changes which registries are fetched; it does not establish a preference or merge records across registries.

## 4. Delegated-statistics source API

PR #39 currently fetches and parses RIR delegated-statistics files inside `asinfo`. Move that work into a small `delegated` source module so it can be used without AsInfo.

```rust
pub fn parse_reader<R: Read>(reader: R) -> impl Iterator<Item = Result<DelegatedRecord, DelegatedError>>;

pub struct DelegatedRecord {
    pub registry: String,
    pub country: String,
    pub record_type: String,
    pub start: String,
    pub value: String,
    pub date: String,
    pub status: String,
}
```

The source parser returns delegated records as published. It does not fill missing ASNs, replace names or countries, or decide which statuses belong in AsInfo. Fetching the five RIR files is also exposed by the `delegated` module.

Filtering to ASN allocation records and constructing `DelegatedInfo` happens in the AsInfo serving layer.

## 5. AsInfo changes

Keep the existing `AsInfo` fields and add `delegated` and `irr` at the same level:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsInfo {
    pub asn: u32,
    pub name: String,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as2org: Option<As2orgInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<AsnPopulationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hegemony: Option<HegemonyData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peeringdb: Option<Network>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegated: Option<DelegatedInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub irr: Vec<IrrAsnInfo>,
}
```

Serde defaults are required so existing cached JSON without `delegated` or `irr` remains readable. This fixes the current PR #39 failure: `missing field 'irr'`.

Serialization includes only data that exists for that ASN:

- `Minimum` records contain only `asn`, `name`, and `country`;
- optional enrichment fields are omitted when their value is `None`;
- `delegated` is omitted when it is `None`;
- `irr` is omitted when it is empty;
- `null` and empty placeholder fields are not emitted.

This rule applies per ASN. Even when a profile loads a source globally, an ASN with no record from that source omits the corresponding field.

`IrrAsnInfo` remains source-scoped:

```rust
pub struct IrrAsnInfo {
    pub as_name: String,
    pub descr: Vec<String>,
    pub source: String,
    pub mnt_by: Vec<String>,
    pub route_prefixes: Vec<Ipv4Net>,
    pub route6_prefixes: Vec<Ipv6Net>,
    pub member_of_sets: Vec<String>,
}
```

Prefixes are stored as `Ipv4Net` and `Ipv6Net`, not `String`. This avoids repeated heap-allocated CIDR strings, reduces in-memory size, and gives callers typed prefix values. With the existing `ipnet` Serde support, JSON output remains human-readable CIDR strings.

`DelegatedInfo` remains source-scoped:

```rust
pub struct DelegatedInfo {
    pub registry: String,
    pub country: String,
    pub date: String,
    pub status: String,
}
```

There may be multiple entries for an ASN because different IRR registries can publish different values. The library does not choose a trusted IRR registry or overwrite the existing `name`, `country`, or other source fields with IRR values.

Delegated data is likewise retained in `delegated`; it does not overwrite the base `name` or `country` fields.

## 6. AsInfo profiles

Keep the existing profile names; no profile-version framework is needed.

```rust
pub enum AsInfoProfile {
    Minimum,
    #[default]
    Default,
    Full,
}
```

The profiles are:

| Profile | Sources |
|---|---|
| `Minimum` | `asn.txt` only: ASN, name, and country |
| `Default` | `asn.txt` + CAIDA AS2Org + APNIC population + IHR hegemony + PeeringDB |
| `Full` | Everything in `Default` + RIR delegated statistics + IRR from every source returned by `all_sources()`, including route and route6 prefixes |

`Default` is grounded in the current normal, non-simplified `../asninfo` v1 path, which calls:

```rust
commons.load_asinfo(true, true, true, true)
```

The four arguments enable AS2Org, population, hegemony, and PeeringDB. The `asn.txt` base is always loaded.

The `asninfo --simplified` behavior is an application output mode and does not define an additional library profile.

Profile mapping remains straightforward:

```rust
match profile {
    AsInfoProfile::Minimum => AsInfoBuilder::new(),
    AsInfoProfile::Default => AsInfoBuilder::new()
        .with_as2org()
        .with_population()
        .with_hegemony()
        .with_peeringdb(),
    AsInfoProfile::Full => AsInfoBuilder::new()
        .with_as2org()
        .with_population()
        .with_hegemony()
        .with_peeringdb()
        .with_delegated()
        .with_irr()
        .with_irr_route_prefixes(),
}
```

`AsInfoBuilder` remains available for custom combinations and explicit IRR source selection.

`with_irr()` means all available IRR sources. `with_irr_sources(config)` uses exactly the caller's validated subset.

## 7. Cargo features

Keep the feature model simple:

- `irr` enables IRR records, parsing, source catalog, and fetching;
- `delegated` enables delegated-statistics records, parsing, and fetching;
- `asinfo` may depend on `irr` and `delegated` because it serves both enrichments;
- no `irr-parse`, `irr-fetch`, `asinfo-irr`, profile-specific, or legacy feature matrix is required for this refactor.

The architectural separation is between module responsibilities and APIs, not a requirement to create a Cargo feature for every boundary.

## 8. Compatibility

- Existing `AsInfo` JSON must deserialize with `delegated: None` and `irr: Vec::new()` when the fields are absent.
- Existing JSON containing `null` for optional enrichment fields must continue to deserialize.
- New serialization omits absent optional fields and empty `irr` arrays instead of writing `null` or `[]`.
- Existing `load_asinfo(bool, bool, bool, bool)` remains as a compatibility entry point and maps to the same behavior as before.
- Existing `AsInfoBuilder` methods remain.
- Existing IRR callback parsing APIs may remain as wrappers.
- `BgpkitCommons` remains supported; making it legacy or removing it is out of scope.
- No other module is redesigned in this change.

## 9. Tests

Required deterministic tests:

1. Parse a local reader without network access.
2. Preserve repeated attributes and their order.
3. Return an unsupported RPSL object as `IrrRecord`.
4. Return a malformed record as an error.
5. Reject an unknown explicitly requested IRR source.
6. `with_irr()` resolves to every source in `all_sources()`.
7. An explicit IRR subset fetches only the selected sources.
8. Store route and route6 prefixes as `Ipv4Net` and `Ipv6Net` and serialize them as CIDR strings.
9. Parse delegated statistics from a local reader without AsInfo.
10. Deserialize old `AsInfo` JSON without `delegated` or `irr` and obtain `None` and an empty vector.
11. Deserialize old `AsInfo` JSON containing `null` optional fields.
12. Serialize a `Minimum` record with only `asn`, `name`, and `country` keys.
13. Omit each `None` enrichment and omit an empty `irr` vector.
14. `Minimum` enables no optional enrichment.
15. `Default` enables exactly AS2Org, population, hegemony, and PeeringDB.
16. `Full` enables everything in `Default` plus delegated statistics, all IRR sources, and route prefixes.
17. Delegated and IRR enrichment never overwrite `AsInfo.name` or `AsInfo.country`.

Live IRR download tests remain ignored network tests; local fixtures are the normal merge gate.

## 10. Implementation sequence

1. Refactor PR #39's IRR parser to return unopinionated `IrrRecord` values while retaining typed compatibility conversions.
2. Make all catalogued IRR sources the default, validate explicit source subsets, and avoid duplicate whole-database fetches.
3. Move delegated-statistics fetching/parsing from `asinfo` into an unopinionated `delegated` module.
4. Keep `AsInfo.delegated` and `AsInfo.irr` as sibling source-scoped fields, add backward-compatible Serde defaults, and omit absent enrichment fields during serialization.
5. Make `Minimum`, `Default`, and `Full` match §6 exactly.
6. Add local parser, profile, and old-cache compatibility tests.
7. Update examples and documentation.

## 11. Acceptance criteria

The refactor is complete when:

- IRR fetching/parsing can be used directly without AsInfo;
- delegated-statistics fetching/parsing can be used directly without AsInfo;
- direct IRR APIs return source records without enrichment policy;
- direct delegated APIs return source records without enrichment policy;
- `AsInfo` adds backward-compatible sibling `delegated` and `irr` fields;
- IRR prefixes are stored as typed `Ipv4Net` and `Ipv6Net` values rather than strings;
- users can select an exact IRR source subset, while `with_irr()` and `Full` load all catalogued sources by default;
- serialized AsInfo records contain no `null` enrichment fields or empty `irr` arrays;
- `Default` reproduces the current normal `asninfo` v1 source selection;
- `Full` adds delegated and all supported IRR enrichment;
- old AsInfo cache data still loads;
- formatting, Clippy, deterministic tests, and all-feature tests pass.
