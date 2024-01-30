# Changelog

All notable changes to this project will be documented in this file.

## v0.5.0 - 2024-01-30

### Breaking changes

- switch to `rustls` as the default TLS backend
  - users can still opt-in to use `native-tls` by specifying `default-features = false` and use `native-tls` feature flag