//! Load current RPKI information from Cloudflare RPKI portal.

use crate::Result;

use super::RpkiTrie;
use super::rpki_client::RpkiClientData;

const CLOUDFLARE_RPKI_URL: &str = "https://rpki.cloudflare.com/rpki.json";

impl RpkiTrie {
    /// Load current RPKI data from Cloudflare RPKI portal.
    ///
    /// This loads real-time RPKI data from Cloudflare's public RPKI JSON endpoint.
    /// The data includes ROAs, ASPAs, and BGPsec keys.
    pub fn from_cloudflare() -> Result<Self> {
        let data = RpkiClientData::from_url(CLOUDFLARE_RPKI_URL)?;
        Self::from_rpki_client_data(data, None)
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
}
