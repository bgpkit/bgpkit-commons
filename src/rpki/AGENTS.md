# RPKI Module

**Purpose:** Resource Public Key Infrastructure validation and historical data access.

## Structure

```
src/rpki/
├── mod.rs              # RpkiTrie, Roa, Aspa, validation logic
├── cloudflare.rs       # Real-time RPKI data from Cloudflare
├── ripe_historical.rs  # RIPE NCC historical archives
├── rpkiviews.rs        # RPKIviews historical data
└── rpki_client.rs      # rpki-client JSON parser
```

## Key Types

| Type | Purpose |
|------|---------|
| `RpkiTrie` | Main storage: IpnetTrie of ROAs + ASPAs |
| `Roa` | Route Origin Authorization (prefix, ASN, max_length, validity) |
| `Aspa` | AS Provider Authorization |
| `RpkiValidation` | Valid / Invalid / Unknown result |
| `HistoricalRpkiSource` | Ripe or RpkiViews enum |

## Data Sources

| Source | Type | Use Case |
|--------|------|----------|
| Cloudflare | Real-time JSON | Current validation |
| RIPE NCC | Historical JSON | All 5 RIRs, archival |
| RPKIviews | Historical .tgz | Multiple vantage points |

## Usage Pattern

```rust
// Real-time validation
commons.load_rpki(None)?;
let result = commons.rpki_validate(64496, "192.0.2.0/24")?;

// Historical data
let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
commons.load_rpki_historical(date, HistoricalRpkiSource::Ripe)?;
```

## Validation Logic

- **Valid:** Prefix-ASN match with valid max_length
- **Invalid:** Prefix has ROAs but not for this ASN
- **Unknown:** No ROAs exist (or all expired if check_expiry enabled)

## Notes

- RPKIviews streams tarballs efficiently (stops early once rpki-client.json found)
- `RoaEntry` type alias is deprecated - use `Roa`
- Multiple ROAs per prefix supported (avoids duplicates by (prefix, asn, max_length))
