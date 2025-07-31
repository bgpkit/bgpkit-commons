//! Load current RPKI information from Cloudflare RPKI portal.

use crate::Result;
use chrono::DateTime;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Rir, RoaEntry, RpkiTrie};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub expires: u64,
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
            
            // Convert expires timestamp to NaiveDateTime
            let not_after = DateTime::from_timestamp(roa.expires as i64, 0)
                .map(|dt| dt.naive_utc());
            
            let roa_entry = RoaEntry {
                prefix,
                asn: roa.asn,
                max_length,
                rir,
                not_before: None,
                not_after,
            };

            trie.insert_roa(roa_entry);
        }
        Ok(trie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    #[ignore] // This test requires network access and is for manual testing
    // Run with: cargo test --release --features rpki test_cloudflare_rpki_expiry_loading -- --ignored --nocapture
    fn test_cloudflare_rpki_expiry_loading() {
        println!("Loading RPKI data from Cloudflare...");
        
        // Load the RPKI data
        let trie = RpkiTrie::from_cloudflare().expect("Failed to load Cloudflare RPKI data");
        
        // Count total ROAs
        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!("Total ROAs loaded: {}", total_roas);
        
        // Count ROAs with expiry dates
        let mut roas_with_expiry = 0;
        let mut expired_roas = 0;
        let mut future_roas = 0;
        let current_time = Utc::now().naive_utc();
        
        for (prefix, roas) in trie.trie.iter() {
            for roa in roas {
                if roa.not_after.is_some() {
                    roas_with_expiry += 1;
                    
                    if let Some(not_after) = roa.not_after {
                        if not_after < current_time {
                            expired_roas += 1;
                            println!("Expired ROA found: prefix={}, asn={}, expired={}", 
                                     prefix, roa.asn, not_after);
                        }
                    }
                }
                
                if let Some(not_before) = roa.not_before {
                    if not_before > current_time {
                        future_roas += 1;
                        println!("Future ROA found: prefix={}, asn={}, valid_from={}", 
                                 prefix, roa.asn, not_before);
                    }
                }
            }
        }
        
        println!("\nSummary:");
        println!("- ROAs with expiry dates: {}", roas_with_expiry);
        println!("- Expired ROAs: {}", expired_roas);
        println!("- Future ROAs: {}", future_roas);
        
        // Test expiry validation with a sample ROA if any have expiry dates
        if roas_with_expiry > 0 {
            // Find a ROA with expiry date to test
            for (prefix, roas) in trie.trie.iter() {
                for roa in roas {
                    if roa.not_after.is_some() {
                        println!("\nTesting validation with expiry check for prefix={}, asn={}", prefix, roa.asn);
                        
                        // Test with current time
                        let validation = trie.validate_check_expiry(&prefix, roa.asn, None);
                        println!("Validation result (current time): {:?}", validation);
                        
                        // Test with a far future time
                        let future_time = DateTime::from_timestamp(3000000000, 0)
                            .map(|dt| dt.naive_utc())
                            .unwrap();
                        let future_validation = trie.validate_check_expiry(&prefix, roa.asn, Some(future_time));
                        println!("Validation result (future time): {:?}", future_validation);
                        
                        break;
                    }
                }
                if roas_with_expiry > 0 { break; }
            }
        }
        
        // Basic sanity check - we should have loaded some ROAs
        assert!(total_roas > 0, "No ROAs were loaded from Cloudflare");
        println!("\nTest completed successfully!");
    }
}
