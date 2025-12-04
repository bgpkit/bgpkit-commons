//! AS-to-Organization mapping using CAIDA data
//!
//! This module provides access to CAIDA's AS-to-Organization dataset, allowing
//! lookups of AS information including organization details and sibling relationships.
//!
//! # Data source
//! - CAIDA AS Organizations Dataset: <http://www.caida.org/data/as-organizations>

use crate::{BgpkitCommonsError, Result};
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Organization JSON format from CAIDA dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
struct As2orgJsonOrg {
    #[serde(alias = "organizationId")]
    org_id: String,

    changed: Option<String>,

    #[serde(default)]
    name: String,

    country: String,

    /// The RIR or NIR database that contained this entry
    source: String,

    #[serde(alias = "type")]
    data_type: String,
}

/// AS JSON format from CAIDA dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
struct As2orgJsonAs {
    asn: String,

    changed: Option<String>,

    #[serde(default)]
    name: String,

    #[serde(alias = "opaqueId")]
    opaque_id: Option<String>,

    #[serde(alias = "organizationId")]
    org_id: String,

    /// The RIR or NIR database that contained this entry
    source: String,

    #[serde(rename = "type")]
    data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum As2orgJsonEntry {
    Org(As2orgJsonOrg),
    As(As2orgJsonAs),
}

/// Public information for an Autonomous System (AS) enriched with its organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct As2orgAsInfo {
    /// The AS number
    pub asn: u32,
    /// The name provided for the individual AS number
    pub name: String,
    /// The registration country code of the organization
    pub country_code: String,
    /// Organization identifier (as used in the dataset)
    pub org_id: String,
    /// Organization name
    pub org_name: String,
    /// The RIR database that contained this entry
    pub source: String,
}

/// In-memory accessor for CAIDA's AS-to-Organization dataset.
pub struct As2org {
    as_map: HashMap<u32, As2orgJsonAs>,
    org_map: HashMap<String, As2orgJsonOrg>,
    as_to_org: HashMap<u32, String>,
    org_to_as: HashMap<String, Vec<u32>>,
}

const BASE_URL: &str = "https://publicdata.caida.org/datasets/as-organizations";

impl As2org {
    /// Create a new `As2org` accessor.
    ///
    /// - When `data_file_path` is `None`, the constructor fetches the CAIDA
    ///   index page to discover the most recent `*.as-org2info.jsonl.gz` file
    ///   and reads it via HTTP(S).
    /// - When `Some(path_or_url)` is provided, the path can be a local file or
    ///   a remote URL. Gzipped files are supported transparently.
    pub fn new(data_file_path: Option<String>) -> Result<Self> {
        let entries = match data_file_path {
            Some(path) => parse_as2org_file(path.as_str())?,
            None => {
                let url = get_most_recent_data()?;
                parse_as2org_file(url.as_str())?
            }
        };

        let mut as_map: HashMap<u32, As2orgJsonAs> = HashMap::new();
        let mut org_map: HashMap<String, As2orgJsonOrg> = HashMap::new();

        for entry in entries {
            match entry {
                As2orgJsonEntry::As(as_entry) => {
                    if let Ok(asn) = as_entry.asn.parse::<u32>() {
                        as_map.insert(asn, as_entry);
                    }
                }
                As2orgJsonEntry::Org(org_entry) => {
                    org_map.insert(org_entry.org_id.clone(), org_entry);
                }
            }
        }

        let mut as_to_org: HashMap<u32, String> = HashMap::new();
        let mut org_to_as: HashMap<String, Vec<u32>> = HashMap::new();

        for (asn, as_entry) in as_map.iter() {
            as_to_org.insert(*asn, as_entry.org_id.clone());
            let org_asn = org_to_as.entry(as_entry.org_id.clone()).or_default();
            org_asn.push(*asn);
        }

        Ok(Self {
            as_map,
            org_map,
            as_to_org,
            org_to_as,
        })
    }

    /// List all available dataset files published by CAIDA with their dates.
    pub fn get_all_files_with_dates() -> Result<Vec<(String, NaiveDate)>> {
        get_all_files_with_dates()
    }

    /// Returns the URL for the latest AS-to-Organization dataset file.
    pub fn get_latest_file_url() -> String {
        format!("{BASE_URL}/latest.as-org2info.jsonl.gz")
    }

    /// Get enriched information for a specific ASN, if present.
    pub fn get_as_info(&self, asn: u32) -> Option<As2orgAsInfo> {
        let as_entry = self.as_map.get(&asn)?;
        let org_id = as_entry.org_id.as_str();
        let org_entry = self.org_map.get(org_id)?;
        Some(As2orgAsInfo {
            asn,
            name: as_entry.name.clone(),
            country_code: org_entry.country.clone(),
            org_id: org_id.to_string(),
            org_name: org_entry.name.clone(),
            source: org_entry.source.clone(),
        })
    }

    /// Return all ASNs that belong to the same organization as the given ASN.
    pub fn get_siblings(&self, asn: u32) -> Option<Vec<As2orgAsInfo>> {
        let org_id = self.as_to_org.get(&asn)?;
        let org_asns = self.org_to_as.get(org_id)?.to_vec();
        Some(
            org_asns
                .iter()
                .filter_map(|asn| self.get_as_info(*asn))
                .collect(),
        )
    }

    /// Return `true` if both ASNs belong to the same organization.
    pub fn are_siblings(&self, asn1: u32, asn2: u32) -> bool {
        let org1 = match self.as_to_org.get(&asn1) {
            None => return false,
            Some(o) => o,
        };
        let org2 = match self.as_to_org.get(&asn2) {
            None => return false,
            Some(o) => o,
        };
        org1 == org2
    }
}

/// Fixes misinterpretation of strings encoded in Latin-1 that were mistakenly decoded as UTF-8.
fn fix_latin1_misinterpretation(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        // Check for the pattern of misinterpreted Latin-1 chars
        if c == 'Ã' && chars.peek().is_some() {
            let next_char = chars.next().unwrap();

            // Calculate the original Latin-1 character
            let byte_value = match next_char {
                '\u{0080}'..='\u{00BF}' => 0xC0 + (next_char as u32 - 0x0080),
                // Handle other ranges as needed
                _ => {
                    // If it doesn't match the pattern, treat as normal chars
                    result.push(c);
                    result.push(next_char);
                    continue;
                }
            };

            // Convert to the correct character
            if let Some(correct_char) = char::from_u32(byte_value) {
                result.push(correct_char);
            } else {
                // Fallback for invalid characters
                result.push(c);
                result.push(next_char);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse remote AS2Org file into Vec of DataEntry
fn parse_as2org_file(path: &str) -> Result<Vec<As2orgJsonEntry>> {
    let mut res: Vec<As2orgJsonEntry> = vec![];

    for line in oneio::read_lines(path)? {
        let line = fix_latin1_misinterpretation(&line?);
        if line.contains(r#""type":"ASN""#) {
            let data = serde_json::from_str::<As2orgJsonAs>(line.as_str());
            match data {
                Ok(data) => {
                    res.push(As2orgJsonEntry::As(data));
                }
                Err(e) => {
                    return Err(BgpkitCommonsError::data_source_error(
                        crate::errors::data_sources::CAIDA,
                        format!("error parsing AS line: {}", e),
                    ));
                }
            }
        } else {
            let data = serde_json::from_str::<As2orgJsonOrg>(line.as_str());
            match data {
                Ok(data) => {
                    res.push(As2orgJsonEntry::Org(data));
                }
                Err(e) => {
                    return Err(BgpkitCommonsError::data_source_error(
                        crate::errors::data_sources::CAIDA,
                        format!("error parsing Org line: {}", e),
                    ));
                }
            }
        }
    }
    Ok(res)
}

/// Returns a vector of tuples containing the full URLs of AS2Org data files and their corresponding dates.
fn get_all_files_with_dates() -> Result<Vec<(String, NaiveDate)>> {
    let data_link: Regex = Regex::new(r".*(\d{8}\.as-org2info\.jsonl\.gz).*").map_err(|e| {
        BgpkitCommonsError::Internal(format!("failed to compile regex: {}", e))
    })?;
    let content = oneio::read_to_string(BASE_URL)?;
    let mut res: Vec<(String, NaiveDate)> = data_link
        .captures_iter(content.as_str())
        .filter_map(|cap| {
            let file = cap[1].to_owned();
            let date = NaiveDate::parse_from_str(&file[..8], "%Y%m%d").ok()?;
            Some((format!("{BASE_URL}/{file}"), date))
        })
        .collect();
    res.sort_by_key(|(_, date)| *date);
    Ok(res)
}

fn get_most_recent_data() -> Result<String> {
    let files = get_all_files_with_dates()?;
    let last_file = files
        .last()
        .ok_or_else(|| BgpkitCommonsError::data_source_error(
            crate::errors::data_sources::CAIDA,
            "No dataset files found",
        ))?;
    Ok(last_file.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_latin1_misinterpretation() {
        // Test that the function handles normal strings correctly
        let normal = "Hello World";
        assert_eq!(fix_latin1_misinterpretation(normal), normal);

        // Test empty string
        assert_eq!(fix_latin1_misinterpretation(""), "");

        // Test string without misinterpreted characters
        let ascii_only = "ACME Corporation Inc.";
        assert_eq!(fix_latin1_misinterpretation(ascii_only), ascii_only);

        // Test string with special characters that shouldn't be modified
        let special = "Test @#$%^&*() 123";
        assert_eq!(fix_latin1_misinterpretation(special), special);
    }

    #[test]
    fn test_as2org_json_org_deserialization() {
        let json = r#"{"organizationId":"ORG-TEST","changed":"20240101","name":"Test Org","country":"US","source":"ARIN","type":"Organization"}"#;
        let org: As2orgJsonOrg = serde_json::from_str(json).unwrap();
        assert_eq!(org.org_id, "ORG-TEST");
        assert_eq!(org.name, "Test Org");
        assert_eq!(org.country, "US");
        assert_eq!(org.source, "ARIN");
        assert_eq!(org.data_type, "Organization");
    }

    #[test]
    fn test_as2org_json_org_with_missing_optional_fields() {
        let json = r#"{"organizationId":"ORG-TEST2","name":"Another Org","country":"DE","source":"RIPE","type":"Organization"}"#;
        let org: As2orgJsonOrg = serde_json::from_str(json).unwrap();
        assert_eq!(org.org_id, "ORG-TEST2");
        assert!(org.changed.is_none());
    }

    #[test]
    fn test_as2org_json_as_deserialization() {
        let json = r#"{"asn":"12345","changed":"20240101","name":"Test AS","opaqueId":"opaque123","organizationId":"ORG-TEST","source":"ARIN","type":"ASN"}"#;
        let as_entry: As2orgJsonAs = serde_json::from_str(json).unwrap();
        assert_eq!(as_entry.asn, "12345");
        assert_eq!(as_entry.name, "Test AS");
        assert_eq!(as_entry.org_id, "ORG-TEST");
        assert_eq!(as_entry.opaque_id, Some("opaque123".to_string()));
        assert_eq!(as_entry.source, "ARIN");
        assert_eq!(as_entry.data_type, "ASN");
    }

    #[test]
    fn test_as2org_json_as_with_missing_optional_fields() {
        let json = r#"{"asn":"67890","name":"Minimal AS","organizationId":"ORG-MIN","source":"APNIC","type":"ASN"}"#;
        let as_entry: As2orgJsonAs = serde_json::from_str(json).unwrap();
        assert_eq!(as_entry.asn, "67890");
        assert!(as_entry.changed.is_none());
        assert!(as_entry.opaque_id.is_none());
    }

    #[test]
    fn test_as2org_json_as_with_empty_name() {
        // Test the #[serde(default)] attribute for name field
        let json = r#"{"asn":"11111","organizationId":"ORG-EMPTY","source":"RIPE","type":"ASN"}"#;
        let as_entry: As2orgJsonAs = serde_json::from_str(json).unwrap();
        assert_eq!(as_entry.name, ""); // default empty string
    }

    #[test]
    fn test_as2org_as_info_struct() {
        let info = As2orgAsInfo {
            asn: 12345,
            name: "Test AS".to_string(),
            country_code: "US".to_string(),
            org_id: "ORG-TEST".to_string(),
            org_name: "Test Organization".to_string(),
            source: "ARIN".to_string(),
        };
        assert_eq!(info.asn, 12345);
        assert_eq!(info.name, "Test AS");
        assert_eq!(info.country_code, "US");
        assert_eq!(info.org_id, "ORG-TEST");
        assert_eq!(info.org_name, "Test Organization");
        assert_eq!(info.source, "ARIN");
    }

    #[test]
    fn test_as2org_as_info_serialization() {
        let info = As2orgAsInfo {
            asn: 12345,
            name: "Test AS".to_string(),
            country_code: "US".to_string(),
            org_id: "ORG-TEST".to_string(),
            org_name: "Test Organization".to_string(),
            source: "ARIN".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"asn\":12345"));
        assert!(json.contains("\"name\":\"Test AS\""));

        // Test round-trip
        let deserialized: As2orgAsInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.asn, info.asn);
        assert_eq!(deserialized.name, info.name);
    }

    #[test]
    fn test_get_latest_file_url() {
        let url = As2org::get_latest_file_url();
        assert!(url.starts_with("https://publicdata.caida.org/datasets/as-organizations/"));
        assert!(url.ends_with(".as-org2info.jsonl.gz"));
    }

    // Integration tests that require network access - marked as ignored by default
    #[test]
    #[ignore]
    fn test_as2org_new_from_latest() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Verify the database was loaded by checking if we have some data
        assert!(!as2org.as_map.is_empty());
        assert!(!as2org.org_map.is_empty());
    }

    #[test]
    #[ignore]
    fn test_as2org_get_as_info_existing() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Test with a well-known ASN (Google)
        let info = as2org.get_as_info(15169);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.asn, 15169);
        assert!(!info.org_id.is_empty());
        assert!(!info.org_name.is_empty());
        assert!(!info.country_code.is_empty());
        assert!(!info.source.is_empty());
    }

    #[test]
    #[ignore]
    fn test_as2org_get_as_info_nonexistent() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Test with a likely non-existent ASN
        let info = as2org.get_as_info(999999999);
        assert!(info.is_none());
    }

    #[test]
    #[ignore]
    fn test_as2org_get_siblings() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Test with Google's AS15169
        let siblings = as2org.get_siblings(15169);
        assert!(siblings.is_some());
        let siblings = siblings.unwrap();
        // Google should have at least a few sibling ASNs
        assert!(!siblings.is_empty());
        // The queried ASN should be included in siblings
        assert!(siblings.iter().any(|s| s.asn == 15169));
    }

    #[test]
    #[ignore]
    fn test_as2org_get_siblings_nonexistent() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        let siblings = as2org.get_siblings(999999999);
        assert!(siblings.is_none());
    }

    #[test]
    #[ignore]
    fn test_as2org_are_siblings_true() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Google ASNs 15169 and 36040 are known siblings
        assert!(as2org.are_siblings(15169, 36040));
    }

    #[test]
    #[ignore]
    fn test_as2org_are_siblings_false() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Google (15169) and Cloudflare (13335) are not siblings
        assert!(!as2org.are_siblings(15169, 13335));
    }

    #[test]
    #[ignore]
    fn test_as2org_are_siblings_nonexistent() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Non-existent ASN should return false
        assert!(!as2org.are_siblings(15169, 999999999));
        assert!(!as2org.are_siblings(999999999, 15169));
        assert!(!as2org.are_siblings(999999999, 999999998));
    }

    #[test]
    #[ignore]
    fn test_as2org_are_siblings_same_asn() {
        let as2org = As2org::new(None).expect("Failed to load AS2org database");
        // Same ASN should be its own sibling
        assert!(as2org.are_siblings(15169, 15169));
    }

    #[test]
    #[ignore]
    fn test_as2org_get_all_files_with_dates() {
        let files = As2org::get_all_files_with_dates().expect("Failed to get file list");
        assert!(!files.is_empty());
        // Files should be sorted by date (ascending)
        for i in 1..files.len() {
            assert!(files[i].1 >= files[i - 1].1);
        }
        // Each URL should point to CAIDA
        for (url, _) in &files {
            assert!(url.contains("publicdata.caida.org"));
            assert!(url.ends_with(".as-org2info.jsonl.gz"));
        }
    }
}
