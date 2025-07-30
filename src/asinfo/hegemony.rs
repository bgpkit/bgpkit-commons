use crate::errors::data_sources;
use crate::{BgpkitCommonsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

const IIJ_IHR_HEGEMONY_IPV4_GLOBAL: &str =
    "https://data.bgpkit.com/ihr/hegemony/ipv4/global/latest-simplified.csv.gz";
const IIJ_IHR_HEGEMONY_IPV6_GLOBAL: &str =
    "https://data.bgpkit.com/ihr/hegemony/ipv6/global/latest-simplified.csv.gz";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hegemony {
    hegemony_map: HashMap<u32, HegemonyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HegemonyData {
    pub asn: u32,
    pub ipv4: f64,
    pub ipv6: f64,
}

fn load_hegemony(path: &str) -> Result<Vec<(u32, f64)>> {
    info!("loading hegemony scores from {}", path);
    // load global hegemony scores data CSV file with header where there are two columns: ASN, score
    let mut hegemony = Vec::new();
    for line in oneio::read_lines(path)? {
        let text = line.ok().ok_or_else(|| {
            BgpkitCommonsError::data_source_error(data_sources::IIJ_IHR, "error reading line")
        })?;
        if text.trim() == "" || text.starts_with("asn") {
            continue;
        }
        let splits: Vec<&str> = text.split(',').map(|x| x.trim()).collect();
        if splits.len() != 2 {
            return Err(BgpkitCommonsError::invalid_format(
                "hegemony data",
                text.as_str(),
                "row missing fields",
            ));
        }
        let asn = match splits[0].parse::<u32>() {
            Ok(asn) => asn,
            Err(_) => {
                debug!("invalid ASN: {}", text);
                continue;
            }
        };
        let score = splits[1].parse::<f64>()?;
        hegemony.push((asn, score));
    }
    Ok(hegemony)
}

impl Hegemony {
    pub fn new() -> Result<Self> {
        let ipv4 = load_hegemony(IIJ_IHR_HEGEMONY_IPV4_GLOBAL)?;
        let ipv6 = load_hegemony(IIJ_IHR_HEGEMONY_IPV6_GLOBAL)?;
        let mut hegemony_map = HashMap::new();
        for (asn, score) in ipv4 {
            hegemony_map.insert(
                asn,
                HegemonyData {
                    asn,
                    ipv4: score,
                    ipv6: 0.0,
                },
            );
        }
        for (asn, score) in ipv6 {
            hegemony_map
                .entry(asn)
                .or_insert_with(|| HegemonyData {
                    asn,
                    ipv4: 0.0,
                    ipv6: 0.0,
                })
                .ipv6 = score;
        }
        Ok(Self { hegemony_map })
    }

    pub fn get_score(&self, asn: u32) -> Option<&HegemonyData> {
        self.hegemony_map.get(&asn)
    }
}
