use crate::as2rel::As2relBgpkitData;
use crate::asinfo::AsInfo;
use crate::collectors::MrtCollector;
use crate::countries::Country;
use crate::rpki::{RoaEntry, RpkiValidation};
use crate::BgpkitCommons;
use anyhow::{anyhow, Result};

/// Countries functions
impl BgpkitCommons {
    pub fn country_all(&self) -> Result<Vec<Country>> {
        if self.countries.is_none() {
            return Err(anyhow!("countries is not loaded"));
        }

        Ok(self.countries.as_ref().unwrap().all_countries())
    }

    pub fn country_by_code(&self, code: &str) -> Result<Option<Country>> {
        if self.countries.is_none() {
            return Err(anyhow!("countries is not loaded"));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_code(code))
    }

    pub fn country_by_name(&self, name: &str) -> Result<Vec<Country>> {
        if self.countries.is_none() {
            return Err(anyhow!("countries is not loaded"));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_name(name))
    }

    pub fn country_by_code3(&self, code: &str) -> Result<Option<Country>> {
        if self.countries.is_none() {
            return Err(anyhow!("countries is not loaded"));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_code(code))
    }
}

impl BgpkitCommons {
    pub fn rpki_lookup_by_prefix(&self, prefix: &str) -> Result<Vec<RoaEntry>> {
        if self.rpki_trie.is_none() {
            return Err(anyhow!("rpki is not loaded"));
        }

        let prefix = prefix.parse()?;

        Ok(self.rpki_trie.as_ref().unwrap().lookup_by_prefix(&prefix))
    }

    pub fn rpki_validate(&self, asn: u32, prefix: &str) -> Result<RpkiValidation> {
        if self.rpki_trie.is_none() {
            return Err(anyhow!("rpki is not loaded"));
        }
        let prefix = prefix.parse()?;
        Ok(self.rpki_trie.as_ref().unwrap().validate(&prefix, asn))
    }
}

impl BgpkitCommons {
    pub fn mrt_collectors_all(&self) -> Result<Vec<MrtCollector>> {
        if self.mrt_collectors.is_none() {
            return Err(anyhow!("mrt_collectors is not loaded"));
        }
        Ok(self.mrt_collectors.clone().unwrap())
    }

    pub fn mrt_collectors_by_name(&self, name: &str) -> Result<Option<MrtCollector>> {
        if self.mrt_collectors.is_none() {
            return Err(anyhow!("mrt_collectors is not loaded"));
        }
        Ok(self
            .mrt_collectors
            .as_ref()
            .unwrap()
            .iter()
            .find(|x| x.name == name)
            .cloned())
    }

    pub fn mrt_collectors_by_country(&self, country: &str) -> Option<Vec<MrtCollector>> {
        self.mrt_collectors
            .as_ref()
            .map(|c| c.iter().filter(|x| x.country == country).cloned().collect())
    }
}

impl BgpkitCommons {
    pub fn bogon_matches_str(&self, s: &str) -> Option<bool> {
        self.bogons.as_ref().map(|b| b.matches_str(s))
    }

    pub fn bogon_is_bogon_prefix(&self, prefix: &str) -> Option<bool> {
        let prefix = prefix.parse().ok()?;
        self.bogons.as_ref().map(|b| b.is_bogon_prefix(&prefix))
    }

    pub fn bogon_is_bogon_asn(&self, asn: u32) -> Option<bool> {
        self.bogons.as_ref().map(|b| b.is_bogon_asn(asn))
    }
}

impl BgpkitCommons {
    pub fn asinfo_get(&self, asn: u32) -> Result<Option<AsInfo>> {
        if self.asinfo.is_none() {
            return Err(anyhow!("asinfo is not loaded"));
        }

        Ok(self.asinfo.as_ref().unwrap().get(asn).cloned())
    }

    pub fn asinfo_are_siblings(&self, asn1: u32, asn2: u32) -> Result<bool> {
        if self.asinfo.is_none() {
            return Err(anyhow!("asinfo is not loaded"));
        }
        if self.asinfo.as_ref().unwrap().load_as2org {
            return Err(anyhow!("asinfo is not loaded with as2org data"));
        }

        let info_1_opt = self.asinfo_get(asn1)?;
        let info_2_opt = self.asinfo_get(asn2)?;
        if info_1_opt.is_some() || info_2_opt.is_some() {
            let org_1_opt = info_1_opt.unwrap().as2org;
            let org_2_opt = info_2_opt.unwrap().as2org;
            if org_1_opt.is_some() || org_2_opt.is_some() {
                return Ok(org_1_opt.unwrap().org_id == org_2_opt.unwrap().org_id);
            }
        }
        Ok(false)
    }
}

impl BgpkitCommons {
    pub fn as2rel_lookup_pair(
        &self,
        asn1: u32,
        asn2: u32,
    ) -> Result<(Vec<As2relBgpkitData>, Vec<As2relBgpkitData>)> {
        if self.as2rel.is_none() {
            return Err(anyhow!("as2rel is not loaded"));
        }

        Ok(self.as2rel.as_ref().unwrap().lookup_pair(asn1, asn2))
    }
}
