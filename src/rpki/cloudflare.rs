//! Load current RPKI information from Cloudflare RPKI portal.

use anyhow::Result;
use ipnet::IpNet;
use serde::Deserialize;
use std::str::FromStr;

use super::{Rir, RoaEntry, RpkiTrie};

#[derive(Clone, Debug, Deserialize)]
struct CfData {
    roas: Vec<CfRoaEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct CfRoaEntry {
    prefix: String,
    #[serde(rename = "maxLength")]
    max_length: u8,
    asn: String,
    ta: String,
}

impl RpkiTrie {
    pub fn from_cloudflare() -> Result<Self> {
        let data: CfData =
            oneio::read_json_struct::<CfData>("https://rpki.cloudflare.com/rpki.json")?;

        let mut trie = RpkiTrie::default();

        for roa in data.roas {
            let prefix = roa.prefix.parse::<IpNet>()?;
            let asn = roa
                .asn
                .to_lowercase()
                .strip_prefix("as")
                .unwrap()
                .parse::<u32>()?;
            let max_length = roa.max_length;
            let rir = match Rir::from_str(roa.ta.as_str()) {
                Ok(rir) => Some(rir),
                Err(_) => None,
            };
            let roa_entry = RoaEntry {
                prefix,
                asn,
                max_length,
                rir,
                not_before: None,
                not_after: None,
            };

            trie.insert_roa(roa_entry);
        }
        Ok(trie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpki::RpkiValidation;
    use std::str::FromStr;

    #[test]
    fn test_cloudflare_rpki() {
        let trie = RpkiTrie::from_cloudflare().unwrap();
        let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
        assert_eq!(trie.validate(&prefix, 13335), RpkiValidation::Valid);
    }
}
