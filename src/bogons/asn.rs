use crate::bogons::utils::{find_rfc_links, remove_footnotes, replace_commas_in_quotes};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

const IANA_ASN_SPECIAL_REGISTRY: &str = "https://www.iana.org/assignments/iana-as-numbers-special-registry/special-purpose-as-numbers.csv";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BogonAsn {
    pub asn_range: (u32, u32),
    pub description: String,
    pub rfc_urls: Vec<String>,
}

impl BogonAsn {
    pub fn matches(&self, asn: u32) -> bool {
        asn >= self.asn_range.0 && asn <= self.asn_range.1
    }
}

fn convert_to_range(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    let start = parts[0].parse::<u32>()?;
    let end = if parts.len() > 1 {
        parts[1].parse::<u32>()?
    } else {
        start
    };
    Ok((start, end))
}

pub fn load_bogon_asns() -> Result<Vec<BogonAsn>> {
    let mut bogons = Vec::new();
    let mut prev_line: Option<String> = None;
    for line in oneio::read_lines(IANA_ASN_SPECIAL_REGISTRY)? {
        let mut text = line.ok().ok_or(anyhow!("error reading line"))?;
        if text.trim() == "" || text.starts_with("AS") {
            continue;
        }
        // remove triple quotes
        text = text.replace("\"\"\"", "\"");
        // remove footnote
        text = remove_footnotes(text);
        // replace commas in quotes
        text = replace_commas_in_quotes(text.as_str());

        let splits: Vec<&str> = text.split(',').map(|x| x.trim()).collect();
        if splits.len() != 3 {
            if prev_line.is_some() {
                return Err(anyhow!("row missing fields: {}", text.as_str()));
            }
            prev_line = Some(text);
            continue;
        }
        prev_line = None;

        let asn_range = convert_to_range(splits[0].replace('\"', "").trim())?;
        let description = splits[1].to_string();
        let rfc_urls = find_rfc_links(splits[2]);

        bogons.push(BogonAsn {
            asn_range,
            description,
            rfc_urls,
        });
    }

    Ok(bogons)
}
