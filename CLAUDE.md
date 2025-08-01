# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building
- `cargo build` - Build the library with default features
- `cargo build --all-features` - Build with all features enabled

### Testing  
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo test --features asinfo` - Run tests with asinfo feature enabled

### Code Quality
- `cargo fmt --check` - Check code formatting (required for CI)
- `cargo fmt` - Format code
- `cargo clippy --all-features -- -D warnings` - Run clippy lints (CI fails on warnings)

### Documentation
- `cargo readme` - Generate README.md from lib.rs doc comments (requires cargo-readme)
- `cargo doc --open` - Generate and open documentation

## Architecture Overview

This is a Rust library providing BGP-related data and utilities through a single `BgpkitCommons` struct that lazily loads different data modules as needed.

### Core Design Pattern
The library follows a lazy-loading pattern:
1. Create a `BgpkitCommons` instance with `BgpkitCommons::new()`
2. Load only the data modules you need with `load_xxx()` methods
3. Use the data through corresponding `xxx_yyy()` methods

### Module Structure
- **mrt_collectors/**: RouteViews and RIPE RIS collector information
- **asinfo/** (feature-gated): AS information, country lookup, population data, hegemony scores
- **countries/**: Country code to name mappings
- **rpki/**: RPKI validation data (historical from RIPE, real-time from Cloudflare)  
- **bogons/**: Bogon ASN and prefix detection
- **as2rel/**: AS-level relationship data

### Feature Flags

#### Module Features
- `asinfo`: AS information, country lookup, population data, hegemony scores (requires as2org-rs and peeringdb-rs)
- `as2rel`: AS-level relationship data generated by BGPKIT
- `bogons`: Bogon ASN and prefix detection from IANA registries
- `countries`: Country code to name mappings from GeoNames
- `mrt_collectors`: RouteViews and RIPE RIS collector information
- `rpki`: RPKI validation data (historical from RIPE, real-time from Cloudflare)

#### Dependency Management
- All dependencies are optional and only included when their respective features are enabled (except `thiserror` for error handling)
- This allows for minimal builds with only the required functionality

#### Convenience Features
- `all`: Enables all module features (this is the default for backwards compatibility)
- `default = ["all"]`: All modules enabled by default

#### Building with Specific Features
```bash
# Build with only bogons support
cargo build --no-default-features --features bogons

# Build with asinfo and countries only  
cargo build --no-default-features --features "asinfo,countries"

# Build with all features (same as default)
cargo build --features all
```

#### Examples
Each example has required features specified:
- `as2org`: requires `asinfo` and `countries` features
- `collectors`: requires `mrt_collectors` feature  
- `list_aspas`: requires `rpki` feature

Examples will automatically use the correct features when built with default settings, or you can build them with minimal features:
```bash
cargo run --no-default-features --features "rpki" --example list_aspas
```

### Key Implementation Details
- All data loading methods return `Result<()>` and can fail if external APIs are unavailable
- The `reload()` method refreshes all currently loaded data sources
- RPKI data can be loaded for specific dates (historical) or current (Cloudflare API)
- AsInfo supports both fresh loading and cached data loading with `load_asinfo_cached()`

### Testing Structure
- Unit tests are embedded in module files
- Integration tests in `tests/` directory test full workflows
- Examples in `examples/` directory demonstrate usage patterns

## Development Workflow Preferences

### Code Quality
- Always run `cargo fmt` after finishing each round of code editing
- Run clippy checks before committing changes
- **IMPORTANT**: Before committing any changes, run all relevant tests and checks from `.github/workflows/rust.yaml`:
  - `cargo fmt --check` - Check code formatting
  - `cargo build --no-default-features` - Build with no features
  - `cargo build` - Build with default features
  - `cargo test` - Run all tests
  - `cargo clippy --all-features -- -D warnings` - Run clippy on all features
  - `cargo clippy --no-default-features` - Run clippy with no features
  - Fix any issues before committing

### Documentation
- Update CHANGELOG.md when implementing fixes or features
- Add changes to the "Unreleased changes" section with appropriate subsections (Feature flags, Bug fixes, Code improvements, etc.)
- **IMPORTANT**: When changing lib.rs documentation, always run `cargo readme > README.md` and commit the README.md changes with a simple message "docs: update README.md from lib.rs documentation"

### Git Operations
- Do not prompt for git operations unless explicitly requested by the user
- Let the user initiate commits and other git actions when they're ready
- **IMPORTANT**: When pushing commits, always list all commits to be pushed first using `git log --oneline origin/[branch]..HEAD` and ask for user confirmation

### Commit Messages and Changelog Writing Guidelines
- **Keep language factual and professional**: Avoid subjective or exaggerated descriptive words
- **Avoid words like**: "comprehensive", "extensive", "amazing", "powerful", "robust", "excellent", etc.
- **Use objective language**: State what was added, changed, or fixed without editorial commentary
- **Good examples**: "Added RPKI documentation", "Fixed validation logic", "Updated error handling"
- **Poor examples**: "Added comprehensive RPKI documentation", "Significantly improved validation", "Enhanced robust error handling"
- **Exception**: Technical precision words are acceptable when factually accurate (e.g., "efficient lookup", "atomic operation")

### Release Process
When preparing a release, follow these steps in order:
1. **Update CHANGELOG.md**: 
   - Move all "Unreleased changes" to a new version section with the release version number and date
   - Add any missing changes that were implemented but not documented
   - Follow the existing format: `## v[VERSION] - YYYY-MM-DD`
2. **Update Cargo.toml**:
   - Update the `version` field to the new version number
   - Follow semantic versioning (major.minor.patch)
3. **Review changes before committing**:
   - Run `git diff` to show all changes
   - Ask the user to confirm the diff is correct
   - Check for accidental version mismatches or unwanted changelog entries
4. **Commit the release preparation**:
   - After user confirmation, commit with message: `release: prepare v[VERSION]`
5. **Create and push git tag**:
   - Create a new git tag with the version number: `git tag v[VERSION]`
   - Push commits first: `git push origin [branch-name]`
   - Then push the tag: `git push origin v[VERSION]`