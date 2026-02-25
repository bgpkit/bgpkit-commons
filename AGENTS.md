# BGPKIT-Commons Knowledge Base

**Generated:** 2026-02-25  
**Commit:** 575fe13  
**Branch:** main

## Overview

BGPKIT-Commons is a Rust library for common BGP-related data and functions with a lazy-loading architecture. Each module can be independently enabled via feature flags, allowing for minimal builds.

## Structure

```
.
├── src/
│   ├── lib.rs              # BgpkitCommons central struct
│   ├── errors.rs           # Error types and Result alias
│   ├── asinfo/             # AS information with multi-source data
│   ├── as2rel/             # AS relationship data
│   ├── bogons/             # Bogon detection (reserved IPs/ASNs)
│   ├── countries/          # Country code mappings
│   ├── mrt_collectors/     # BGP collector metadata
│   └── rpki/               # RPKI validation (ROA/ASPA)
├── examples/               # Feature-gated usage examples
└── tests/                  # Integration tests
```

## Where to Look

| Task | Location | Notes |
|------|----------|-------|
| Add AS info data source | `src/asinfo/` | See `mod.rs` for builder pattern |
| Add RPKI data source | `src/rpki/` | Implements `from_rpki_client_data` |
| Update error messages | `src/errors.rs` | Use constant modules for consistency |
| Add example | `examples/` | Add required-features in Cargo.toml |
| CI checks | `.github/workflows/rust.yaml` | Tests all feature combinations |

## Code Map

| Symbol | Type | Location | Purpose |
|--------|------|----------|---------|
| `BgpkitCommons` | struct | `lib.rs` | Central handle for all modules |
| `LazyLoadable` | trait | `lib.rs` | Reload trait for all modules |
| `AsInfoBuilder` | struct | `asinfo/mod.rs` | Configure AS info data sources |
| `RpkiTrie` | struct | `rpki/mod.rs` | RPKI data storage and validation |

## Conventions

- **Rust 2024 edition** with relaxed clippy rules (`uninlined_format_args`, `collapsible_if` allowed)
- **Feature-gated modules** - All modules behind feature flags (`asinfo`, `rpki`, etc.)
- **Lazy-loading pattern** - Always call `load_xxx()` before accessing data
- **Builder pattern** - `AsInfoBuilder` for ergonomic configuration
- **README generation** - Run `cargo readme > README.md` after lib.rs doc changes

## Anti-Patterns

- `RoaEntry` type alias is **deprecated** (use `Roa` instead) - will be removed in v0.12.0
- Must call `load_xxx()` before accessing module data - access methods return errors if not loaded
- Never edit `README.md` directly - generate from `lib.rs` via `cargo-readme`

## Commands

```bash
# Development
cargo test --all-features                    # Run all tests
cargo clippy --all-features -- -D warnings   # Full lint check
cargo readme > README.md                     # Regenerate README

# Feature testing
cargo build --no-default-features            # Minimal build
cargo test --features "bogons,rpki"          # Specific features
```

## Notes

- Release process is tag-triggered (`v*`) with automatic crates.io publish
- Examples require specific features declared in Cargo.toml
- Network-dependent tests marked with `#[ignore]` - run with `cargo test -- --ignored`
