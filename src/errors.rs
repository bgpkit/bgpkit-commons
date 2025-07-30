//! Error types for bgpkit-commons
//!
//! This module defines structured error types using `thiserror` for better error handling
//! and debugging. Each error type provides specific context about what failed and why.

use thiserror::Error;

/// Main error type for bgpkit-commons operations
#[derive(Error, Debug)]
pub enum BgpkitCommonsError {
    /// Error when trying to access data from a module that hasn't been loaded yet
    #[error("Module '{module}' data not loaded. Call {load_method}() first")]
    ModuleNotLoaded {
        module: &'static str,
        load_method: &'static str,
    },

    /// Error when trying to access specific functionality that requires additional configuration
    #[error(
        "Module '{module}' not loaded with required configuration: {requirement}. Call {load_method}"
    )]
    ModuleNotConfigured {
        module: &'static str,
        requirement: &'static str,
        load_method: &'static str,
    },

    /// Error when external data sources are unavailable or return invalid data
    #[error("Failed to load data from {data_source}: {details}")]
    DataSourceError {
        data_source: String,
        details: String,
    },

    /// Error when input data format is invalid
    #[error("Invalid {data_type} format '{input}': {reason}")]
    InvalidFormat {
        data_type: &'static str,
        input: String,
        reason: String,
    },

    /// Error when required features are not enabled
    #[error(
        "Feature '{feature}' is required but not enabled. Add '{feature}' to your Cargo.toml features"
    )]
    FeatureNotEnabled { feature: &'static str },

    /// Network or I/O related errors
    #[error("Network/IO error: {0}")]
    NetworkError(#[from] std::io::Error),

    /// JSON parsing errors
    #[cfg(feature = "serde_json")]
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Date/time parsing errors
    #[cfg(feature = "chrono")]
    #[error("Date/time parsing error: {0}")]
    ChronoError(#[from] chrono::ParseError),

    /// IP network parsing errors
    #[cfg(feature = "ipnet")]
    #[error("IP network parsing error: {0}")]
    IpNetError(#[from] ipnet::AddrParseError),

    /// OneIO errors (file/network operations)
    #[cfg(feature = "oneio")]
    #[error("OneIO error: {0}")]
    OneIoError(#[from] oneio::OneIoError),

    /// Parsing errors (int, float, etc.)
    #[error("Parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// Parsing errors (int, float, etc.)
    #[error("Parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    /// Generic error for cases not covered by specific error types
    #[error("Internal error: {0}")]
    Internal(String),
}

impl BgpkitCommonsError {
    /// Create a module not loaded error
    pub fn module_not_loaded(module: &'static str, load_method: &'static str) -> Self {
        Self::ModuleNotLoaded {
            module,
            load_method,
        }
    }

    /// Create a module not configured error
    pub fn module_not_configured(
        module: &'static str,
        requirement: &'static str,
        load_method: &'static str,
    ) -> Self {
        Self::ModuleNotConfigured {
            module,
            requirement,
            load_method,
        }
    }

    /// Create a data source error
    pub fn data_source_error(source: impl Into<String>, details: impl Into<String>) -> Self {
        Self::DataSourceError {
            data_source: source.into(),
            details: details.into(),
        }
    }

    /// Create an invalid format error
    pub fn invalid_format(
        data_type: &'static str,
        input: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidFormat {
            data_type,
            input: input.into(),
            reason: reason.into(),
        }
    }

    /// Create a feature not enabled error
    pub fn feature_not_enabled(feature: &'static str) -> Self {
        Self::FeatureNotEnabled { feature }
    }
}

/// Result type alias for bgpkit-commons operations
pub type Result<T> = std::result::Result<T, BgpkitCommonsError>;

/// Module-specific error constants for consistent error messages
pub mod modules {
    pub const ASINFO: &str = "asinfo";
    pub const AS2REL: &str = "as2rel";
    pub const BOGONS: &str = "bogons";
    pub const COUNTRIES: &str = "countries";
    pub const MRT_COLLECTORS: &str = "mrt_collectors";
    pub const MRT_COLLECTOR_PEERS: &str = "mrt_collector_peers";
    pub const RPKI: &str = "rpki";
}

/// Load method constants for consistent error messages
pub mod load_methods {
    pub const LOAD_ASINFO: &str = "load_asinfo";
    pub const LOAD_ASINFO_CACHED: &str = "load_asinfo_cached";
    pub const LOAD_AS2REL: &str = "load_as2rel";
    pub const LOAD_BOGONS: &str = "load_bogons";
    pub const LOAD_COUNTRIES: &str = "load_countries";
    pub const LOAD_MRT_COLLECTORS: &str = "load_mrt_collectors";
    pub const LOAD_MRT_COLLECTOR_PEERS: &str = "load_mrt_collector_peers";
    pub const LOAD_RPKI: &str = "load_rpki";
}

/// Data source constants for consistent error messages
pub mod data_sources {
    pub const RIPE_NCC: &str = "RIPE NCC";
    pub const CLOUDFLARE: &str = "Cloudflare";
    pub const CAIDA: &str = "CAIDA";
    pub const BGPKIT: &str = "BGPKIT";
    pub const GEONAMES: &str = "GeoNames";
    pub const IANA: &str = "IANA";
    pub const ROUTEVIEWS: &str = "RouteViews";
    pub const PEERINGDB: &str = "PeeringDB";
    pub const APNIC: &str = "APNIC";
    pub const IIJ_IHR: &str = "IIJ IHR";
}
