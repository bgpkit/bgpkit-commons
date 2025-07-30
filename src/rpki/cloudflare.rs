//! Load current RPKI information from Cloudflare RPKI portal.

use crate::Result;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Rir, RoaEntry, RpkiTrie};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfData {
    pub metadata: CfMetaData,
    pub roas: Vec<CfRoaEntry>,
    pub aspas: Vec<CfAspaEntry>,
    pub bgpsec_keys: Vec<CfBgpsecKeysEntry>,
}

impl CfData {
    pub fn new() -> Result<Self> {
        let data: CfData =
            oneio::read_json_struct::<CfData>("https://rpki.cloudflare.com/rpki.json")?;
        Ok(data)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfMetaData {
    pub buildmachine: Option<String>,
    pub buildtime: Option<String>,
    pub elapsedtime: Option<u32>,
    pub usertime: Option<u32>,
    pub systemtime: Option<u32>,
    pub roas: Option<u32>,
    pub failedroas: Option<u32>,
    pub invalidroas: Option<u32>,
    pub spls: Option<u32>,
    pub failedspls: Option<u32>,
    pub invalidspls: Option<u32>,
    pub aspas: Option<u32>,
    pub failedaspas: Option<u32>,
    pub invalidaspas: Option<u32>,
    pub bgpsec_pubkeys: Option<u32>,
    pub certificates: Option<u32>,
    pub invalidcertificates: Option<u32>,
    pub taks: Option<u32>,
    pub tals: Option<u32>,
    pub invalidtals: Option<u32>,
    pub talfiles: Option<Vec<String>>,
    pub manifests: Option<u32>,
    pub failedmanifests: Option<u32>,
    pub crls: Option<u32>,
    pub gbrs: Option<u32>,
    pub repositories: Option<u32>,
    pub vrps: Option<u32>,
    pub uniquevrps: Option<u32>,
    pub vsps: Option<u32>,
    pub uniquevsps: Option<u32>,
    pub vaps: Option<u32>,
    pub uniquevaps: Option<u32>,
    pub cachedir_new_files: Option<u32>,
    pub cachedir_del_files: Option<u32>,
    pub cachedir_del_dirs: Option<u32>,
    pub cachedir_superfluous_files: Option<u32>,
    pub cachedir_del_superfluous_files: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfAspaEntry {
    pub customer_asid: u32,
    pub expires: i64,
    pub providers: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfBgpsecKeysEntry {
    pub asn: u32,
    pub ski: String,
    pub pubkey: String,
    pub ta: String,
    pub expires: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfRoaEntry {
    pub prefix: String,
    #[serde(rename = "maxLength")]
    pub max_length: u8,
    pub asn: u32,
    pub ta: String,
}

impl RpkiTrie {
    pub fn from_cloudflare() -> Result<Self> {
        let data: CfData =
            oneio::read_json_struct::<CfData>("https://rpki.cloudflare.com/rpki.json")?;

        let mut trie = RpkiTrie {
            aspas: data.aspas,
            ..Default::default()
        };

        for roa in data.roas {
            let prefix = roa.prefix.parse::<IpNet>()?;
            let max_length = roa.max_length;
            let rir = Rir::from_str(roa.ta.as_str()).ok();
            let roa_entry = RoaEntry {
                prefix,
                asn: roa.asn,
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
