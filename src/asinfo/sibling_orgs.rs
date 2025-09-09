use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::info;

const BGPKIT_SIBLING_ORGS_URL: &str = "https://data.bgpkit.com/commons/sibling-orgs.txt";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingOrgsUtils {
    sibling_orgs_map: HashMap<String, HashSet<String>>,
}

impl SiblingOrgsUtils {
    pub fn new() -> Result<Self> {
        info!(
            "loading sibling orgs information from {}",
            BGPKIT_SIBLING_ORGS_URL
        );
        let mut sibling_orgs = vec![];
        for line in oneio::read_lines(BGPKIT_SIBLING_ORGS_URL)? {
            let line_str = line?.trim().to_string();
            if line_str.is_empty() || line_str.starts_with('#') {
                // skip empty line or line started with #
                continue;
            }
            let orgs: Vec<String> = line_str.split_whitespace().map(|x| x.to_owned()).collect();
            sibling_orgs.push(orgs);
        }

        let mut res_map = HashMap::new();
        for sibling_lst in sibling_orgs {
            let mut org_set: HashSet<String> = HashSet::new();
            sibling_lst.iter().for_each(|org| {
                org_set.insert(org.to_lowercase());
            });

            sibling_lst.iter().for_each(|org| {
                let org_id = org.to_owned();
                res_map.insert(org_id.to_lowercase(), org_set.clone());
            });
        }

        Ok(SiblingOrgsUtils {
            sibling_orgs_map: res_map,
        })
    }

    pub fn are_sibling_orgs(&self, org_1: &str, org_2: &str) -> bool {
        if let Some(s) = self.sibling_orgs_map.get(org_1.to_lowercase().as_str()) {
            if s.contains(org_2.to_lowercase().as_str()) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sibling_orgs() {
        let utils = SiblingOrgsUtils::new().unwrap();

        // GTT
        assert!(utils.are_sibling_orgs("GC-494-ARIN", "ORG-GCI2-RIPE"));
        // GTT with random cases
        assert!(utils.are_sibling_orgs("Gc-494-ArIn", "OrG-gCi2-RiPe"));
        // GTT and Cogent (not sibling)
        assert!(!utils.are_sibling_orgs("GC-494-ARIN", "COGC-ARIN"));
    }
}
