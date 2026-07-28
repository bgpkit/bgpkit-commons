//! PeeringDB HTTP client with authentication

use crate::errors::data_sources;
use crate::{BgpkitCommonsError, Result};
use std::io::Read;
use tracing::warn;

pub(crate) const NET_API_URL: &str = "https://www.peeringdb.com/api/net";
pub(crate) const IX_API_URL: &str = "https://www.peeringdb.com/api/ix";
pub(crate) const IXLAN_API_URL: &str = "https://www.peeringdb.com/api/ixlan";
pub(crate) const IXPFX_API_URL: &str = "https://www.peeringdb.com/api/ixpfx";
pub(crate) const NETIXLAN_API_URL: &str = "https://www.peeringdb.com/api/netixlan";
pub(crate) const FAC_API_URL: &str = "https://www.peeringdb.com/api/fac";
pub(crate) const NETFAC_API_URL: &str = "https://www.peeringdb.com/api/netfac";
pub(crate) const IXFAC_API_URL: &str = "https://www.peeringdb.com/api/ixfac";
pub(crate) const ORG_API_URL: &str = "https://www.peeringdb.com/api/org";
pub(crate) const CAMPUS_API_URL: &str = "https://www.peeringdb.com/api/campus";
pub(crate) const CARRIER_API_URL: &str = "https://www.peeringdb.com/api/carrier";
pub(crate) const CARRIERFAC_API_URL: &str = "https://www.peeringdb.com/api/carrierfac";

/// Get a reader for a PeeringDB API endpoint with proper authentication headers.
///
/// Uses the `PEERINGDB_API_KEY` environment variable for authentication.
/// When the key is absent, the `Authorization` header is omitted entirely
/// (PeeringDB returns 400 for an empty `Api-Key` header when gzip is advertised).
pub fn get_peeringdb_reader(url: &str) -> Result<Box<dyn Read + Send>> {
    let api_key = std::env::var("PEERINGDB_API_KEY").unwrap_or_else(|_| {
        warn!("missing PEERINGDB_API_KEY env var, call may fail due to rate limiting");
        String::new()
    });

    let authorization = format!("Api-Key {api_key}");
    let user_agent = format!("bgpkit-commons/{}", env!("CARGO_PKG_VERSION"));
    let mut client_builder = oneio::OneIo::builder().header_str("User-Agent", &user_agent);
    if !api_key.is_empty() {
        client_builder = client_builder.header_str("Authorization", &authorization);
    }
    let client = client_builder.build()?;

    let res = client.get_http_reader_raw(url).map_err(|e| {
        BgpkitCommonsError::data_source_error(
            data_sources::PEERINGDB,
            format!("request failed: {e}"),
        )
    })?;

    Ok(Box::new(res))
}
