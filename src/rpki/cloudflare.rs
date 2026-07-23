//! Load current RPKI information from Cloudflare RPKI portal.

use crate::errors::data_sources;
use crate::{BgpkitCommonsError, Result};

use super::RpkiTrie;
use super::rpki_client::RpkiClientData;

const CLOUDFLARE_RPKI_URL: &str = "https://rpki.cloudflare.com/rpki.json";

/// Result of loading RPKI data from the Cloudflare RPKI portal, including the
/// HTTP validators needed for subsequent conditional requests.
///
/// See [`RpkiTrie::from_cloudflare_conditional`] for the poll-and-swap workflow
/// this type is designed for.
#[derive(Clone)]
pub struct RpkiLoad {
    /// The freshly built RPKI trie
    pub trie: RpkiTrie,
    /// Value of the source's `ETag` header, if present.
    ///
    /// Pass this back to [`RpkiTrie::from_cloudflare_conditional`] to receive
    /// `Ok(None)` (a cheap ~0-byte `304 Not Modified` response) until the
    /// upstream data actually changes.
    pub etag: Option<String>,
    /// Value of the source's `Last-Modified` header, if present.
    ///
    /// Usable as a fallback validator when the source does not provide an ETag.
    pub last_modified: Option<String>,
}

impl RpkiTrie {
    /// Load current RPKI data from Cloudflare RPKI portal.
    ///
    /// This loads real-time RPKI data from Cloudflare's public RPKI JSON endpoint.
    /// The data includes ROAs, ASPAs, and BGPsec keys.
    ///
    /// The request advertises `Accept-Encoding: gzip`, reducing the transfer
    /// size from ~97 MB to ~4.6 MB.
    ///
    /// For long-running services that poll frequently, prefer
    /// [`RpkiTrie::from_cloudflare_conditional`], which skips the download and
    /// parse entirely when the upstream data has not changed.
    pub fn from_cloudflare() -> Result<Self> {
        match Self::from_cloudflare_conditional(None, None)? {
            Some(load) => Ok(load.trie),
            None => Err(BgpkitCommonsError::data_source_error(
                data_sources::CLOUDFLARE,
                "server returned 304 Not Modified for an unconditional request",
            )),
        }
    }

    /// Conditionally load current RPKI data from Cloudflare RPKI portal.
    ///
    /// Sends the given `etag` as `If-None-Match` and `last_modified` as
    /// `If-Modified-Since`. When the upstream data has not changed, the server
    /// responds with `304 Not Modified` and this returns `Ok(None)` without
    /// downloading or re-parsing the ~97 MB payload.
    ///
    /// On change, returns `Ok(Some(`[`RpkiLoad`]))` containing a freshly built
    /// trie and the new validator values to use for the next poll.
    ///
    /// # Example: poll-and-swap loop
    ///
    /// ```rust,no_run
    /// use bgpkit_commons::rpki::RpkiTrie;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut etag: Option<String> = None;
    /// loop {
    ///     match RpkiTrie::from_cloudflare_conditional(etag.as_deref(), None)? {
    ///         Some(load) => {
    ///             etag = load.etag.clone();
    ///             // Atomically swap `load.trie` into shared state (e.g. with
    ///             // `arc-swap`) so readers never block during the rebuild.
    ///         }
    ///         None => { /* unchanged: keep serving the existing trie */ }
    ///     }
    ///     std::thread::sleep(std::time::Duration::from_secs(300));
    /// }
    /// # }
    /// ```
    pub fn from_cloudflare_conditional(
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<Option<RpkiLoad>> {
        let Some(fetch) =
            RpkiClientData::from_url_conditional(CLOUDFLARE_RPKI_URL, etag, last_modified)?
        else {
            return Ok(None);
        };
        let trie = Self::from_rpki_client_data(fetch.data, None)?;
        Ok(Some(RpkiLoad {
            trie,
            etag: fetch.etag,
            last_modified: fetch.last_modified,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires network access
    fn test_from_cloudflare() {
        let trie = RpkiTrie::from_cloudflare().expect("Failed to load Cloudflare RPKI data");

        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!("Loaded {} ROAs from Cloudflare", total_roas);
        println!("Loaded {} ASPAs", trie.aspas.len());

        assert!(total_roas > 0, "Should have loaded some ROAs");
    }

    #[test]
    #[ignore] // Requires network access
    fn test_from_cloudflare_conditional() {
        // Initial unconditional load: must return data and an ETag validator.
        let load = RpkiTrie::from_cloudflare_conditional(None, None)
            .expect("Failed to load Cloudflare RPKI data")
            .expect("Unconditional load should return data");
        assert!(load.etag.is_some(), "Cloudflare should return an ETag");
        assert!(load.last_modified.is_some());

        // Polling with the same validators must short-circuit with Ok(None).
        let unchanged = RpkiTrie::from_cloudflare_conditional(
            load.etag.as_deref(),
            load.last_modified.as_deref(),
        )
        .expect("Conditional request failed");
        assert!(
            unchanged.is_none(),
            "Same validators should yield 304 Not Modified"
        );

        // A stale/unknown validator must trigger a full re-download.
        let changed = RpkiTrie::from_cloudflare_conditional(Some("\"stale-etag\""), None)
            .expect("Conditional request failed");
        assert!(changed.is_some(), "Stale ETag should yield fresh data");
    }
}
