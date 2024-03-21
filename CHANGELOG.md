# Changelog

All notable changes to this project will be documented in this file.

## v0.5.2 - 2024-03-20

### Highlights

* update `oneio` to `0.16.5` to fix route-views collector API issue

## v0.5.1 - 2024-03-20

### Highlights

* add new `bgpkit-commons` binary with `export` subcommand to export all data to JSON files
* replace `reqwest` with `oneio` as the default HTTP client

## v0.5.0 - 2024-01-30

### Breaking changes

- switch to `rustls` as the default TLS backend
    - users can still opt-in to use `native-tls` by specifying `default-features = false` and use `native-tls` feature
      flag