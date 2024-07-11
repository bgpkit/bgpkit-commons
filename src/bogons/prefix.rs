use crate::bogons::utils::{find_rfc_links, remove_footnotes, replace_commas_in_quotes};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

const IANA_PREFIX_SPECIAL_REGISTRY: [&str; 2] = [
    "https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry-1.csv",
    "https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry-1.csv",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BogonPrefix {
    pub prefix: IpNet,
    pub description: String,
    pub rfc_urls: Vec<String>,
    pub allocation_date: NaiveDate,
    pub termination_date: Option<NaiveDate>,
    pub source: bool,
    pub destination: bool,
    pub forwardable: bool,
    pub global: bool,
    pub reserved: bool,
}

impl BogonPrefix {
    pub fn matches(&self, prefix: &IpNet) -> bool {
        // if address family different, it is not a match
        match (self.prefix, prefix) {
            (IpNet::V4(_), IpNet::V6(_)) | (IpNet::V6(_), IpNet::V4(_)) => return false,
            _ => {}
        }
        self.prefix.contains(prefix)
    }
}

pub fn load_bogon_prefixes() -> Result<Vec<BogonPrefix>> {
    let mut bogons = Vec::new();
    for iana_link in IANA_PREFIX_SPECIAL_REGISTRY {
        let mut prev_line: Option<String> = None;
        for line in oneio::read_lines(iana_link)? {
            let mut text = line.ok().ok_or(anyhow!("error reading line"))?;
            if let Some(t) = &prev_line {
                text = format!("{},{}", t, text);
            }
            if text.trim() == "" || text.starts_with("Address") {
                continue;
            }
            // remove triple quotes
            text = text.replace("\"\"\"", "\"");
            // remove footnote
            text = remove_footnotes(text);
            // replace commas in quotes
            text = replace_commas_in_quotes(text.as_str());

            let splits: Vec<&str> = text.split(',').map(|x| x.trim()).collect();
            if splits.len() != 10 {
                if prev_line.is_some() {
                    return Err(anyhow!("row missing fields: {}", text.as_str()));
                }
                prev_line = Some(text);
                continue;
            }
            prev_line = None;

            let prefixes = splits[0]
                .replace('\"', "")
                .split(' ')
                .map(|x| x.trim().parse::<IpNet>().unwrap())
                .collect::<Vec<IpNet>>();
            let description = splits[1].to_string();
            let rfc_urls = find_rfc_links(splits[2]);
            let allocation_date =
                NaiveDate::parse_from_str(format!("{}-01", splits[3]).as_str(), "%Y-%m-%d")?;
            let termination_date = match format!("{}-01", splits[3]).as_str() {
                "" | "N/A" => None,
                d => Some(NaiveDate::parse_from_str(d, "%Y-%m-%d")?),
            };
            let source = matches!(splits[5].to_lowercase().as_str(), "true");
            let destination = matches!(splits[6].to_lowercase().as_str(), "true");
            let forwardable = matches!(splits[7].to_lowercase().as_str(), "true");
            let global = matches!(splits[8].to_lowercase().as_str(), "true");
            let reserved = matches!(splits[9].to_lowercase().as_str(), "true");
            bogons.extend(prefixes.into_iter().map(|prefix| BogonPrefix {
                prefix,
                description: description.clone(),
                rfc_urls: rfc_urls.clone(),
                allocation_date,
                termination_date,
                source,
                destination,
                forwardable,
                global,
                reserved,
            }));
        }
    }
    Ok(bogons)
}
