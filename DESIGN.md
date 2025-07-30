# BGPKIT-Commons Design Documentation

## Architecture Overview

BGPKIT-Commons uses a **lazy-loading architecture** where data modules are loaded on-demand and cached in memory for subsequent access. This design allows users to:

- Load only the data they need (minimal memory usage)
- Pay loading costs only when necessary (performance optimization)
- Enable/disable modules via feature flags (minimal binary size)

## Core Design Principles

### 1. Lazy Loading Pattern
```rust
let mut commons = BgpkitCommons::new();  // No data loaded yet
commons.load_bogons().unwrap();          // Load on demand
let result = commons.bogons_match(".."); // Fast cached access
```

### 2. Feature-Gated Modules
Each module is behind a feature flag, allowing users to compile only what they need:
```toml
bgpkit-commons = { version = "0.8", features = ["bogons", "countries"] }
```

### 3. Consistent API Surface
All modules follow the same pattern:
- `load_xxx()` - Load data into memory
- `xxx_yyy()` - Access loaded data
- `reload()` - Refresh all loaded data

## Current Module Status

### ✅ Implemented Modules

| Module | Load Method | Access Pattern | Error Handling | Status |
|--------|-------------|----------------|----------------|---------|
| `asinfo` | `load_asinfo()` | `asinfo_get()`, `asinfo_all()` | `Result<T>` | ✅ Complete |
| `countries` | `load_countries()` | `country_by_code()` | `Result<T>` | ✅ Complete |
| `mrt_collectors` | `load_mrt_collectors()` | `mrt_collectors_all()` | `Result<T>` | ✅ Complete |
| `rpki` | `load_rpki()` | `rpki_validate()` | `Result<T>` | ✅ Complete |
| `as2rel` | `load_as2rel()` | `as2rel_lookup()` | `Result<T>` | ✅ Complete |
| `bogons` | `load_bogons()` | `bogons_match()` | `Result<T>` | ✅ Complete |

## Design Inconsistencies & Improvement Plan

### 🔧 Current Issues

#### 1. **Inconsistent Error Handling** ✅ COMPLETED
- **Problem**: Most modules returned `Result<T>` but `bogons` returned `Option<T>`
- **Impact**: Confusing API, silent failures in bogons module
- **Solution**: ✅ Standardized all modules to return `Result<T>` with clear error messages

```rust
// Before (inconsistent)
pub fn bogons_match(&self, s: &str) -> Option<bool>        // Silent failure
pub fn asinfo_get(&self, asn: u32) -> Result<Option<T>>    // Clear error

// After (consistent) ✅
pub fn bogons_match(&self, s: &str) -> Result<bool>        // Clear error for all
```

#### 2. **Mixed Return Patterns** (Priority: Medium)
- **Problem**: Some methods return owned data, others return references
- **Impact**: Inconsistent memory usage and API expectations
- **Solution**: Establish clear guidelines based on data size and usage patterns

#### 3. **Varied Reload Behavior** (Priority: Medium)
- **Problem**: Some modules have internal reload logic, others recreate objects
- **Impact**: Inconsistent performance characteristics
- **Solution**: Implement trait-based reloading interface

### 🎯 Planned Improvements

#### Phase 1: Error Handling Standardization ✅ COMPLETED
- [x] Update `bogons` module to return `Result<T>` instead of `Option<T>`
- [x] Ensure all error messages include guidance on which `load_xxx()` method to call
- [x] Add consistent error types across all modules

#### Phase 2: API Consistency
- [ ] Standardize return types (owned vs referenced data)
- [ ] Implement consistent naming patterns for access methods
- [ ] Add comprehensive documentation for all public methods

#### Phase 3: Advanced Features ✅ PARTIALLY COMPLETED
- [x] Create `LazyLoadable` trait for consistent reload behavior
- [ ] Implement builder pattern for complex modules (like `asinfo`)
- [ ] Add async loading support for better performance

## Module-Specific Details

### AsInfo Module
- **Complexity**: High (multiple data sources, configuration options)
- **Load Pattern**: `load_asinfo(bool, bool, bool, bool)` with feature flags
- **Caching**: Internal `AsInfoUtils` with HashMap storage
- **Reload**: Custom reload logic that preserves configuration

### Bogons Module  
- **Complexity**: Low (static IANA data)
- **Load Pattern**: Simple `load_bogons()` 
- **Caching**: Direct `Bogons` struct storage
- **Status**: Consistent `Result<T>` error handling ✅

### RPKI Module
- **Complexity**: Medium (trie-based storage, multiple ROAs per prefix)
- **Load Pattern**: `load_rpki(Option<Date>)` for historical vs current data
- **Caching**: Custom `RpkiTrie` with `Vec<RoaEntry>` per prefix
- **Recent Fix**: Now properly handles multiple ROAs per prefix ✅

### Countries Module
- **Complexity**: Low (static GeoNames data)
- **Load Pattern**: Simple `load_countries()`
- **Caching**: HashMap-based lookup by country code
- **Access**: Multiple lookup methods (by code, by name, etc.)

### MRT Collectors Module  
- **Complexity**: Medium (two data sources: collectors + peers)
- **Load Pattern**: Separate `load_mrt_collectors()` and `load_mrt_collector_peers()`
- **Caching**: Vec storage for both collector types
- **Access**: Filtered access methods (all, by name, full-feed only)

### AS2Rel Module
- **Complexity**: Medium (relationship data with IPv4/IPv6 separation)
- **Load Pattern**: Simple `load_as2rel()`
- **Caching**: HashMap with complex key structure
- **Access**: Lookup by ASN pair with relationship data

## Implementation Tracking

### ✅ Completed
- Feature-gated module system
- Basic lazy loading for all modules
- RPKI multiple ROAs support
- Comprehensive documentation
- Error handling standardization across all modules
- Enhanced lib.rs functionality documentation
- API consistency improvements (return type patterns)
- Trait-based lazy loading interface (`LazyLoadable` trait)

### 🚧 In Progress  
- None currently

### 📋 Planned
- Builder pattern for complex modules (e.g., `asinfo` with feature flags)
- Async loading support for better performance
- Performance optimizations (caching, streaming)
- Method chaining API

## Usage Patterns

### Basic Single Module
```rust
let mut commons = BgpkitCommons::new();
commons.load_bogons()?;
let is_bogon = commons.bogons_match("23456")?;
```

### Multiple Module Workflow
```rust
let mut commons = BgpkitCommons::new();
commons.load_asinfo(true, false, false, false)?;
commons.load_countries()?;

let asinfo = commons.asinfo_get(13335)?.unwrap();
let country = commons.country_by_code(&asinfo.country)?.unwrap();
```

### Reload Pattern
```rust
// Load initial data
commons.load_bogons()?;
commons.load_countries()?;

// Later refresh all loaded data
commons.reload()?;  // Refreshes both bogons and countries
```

---

*This document is maintained as part of the development process. Update it when making design changes.*