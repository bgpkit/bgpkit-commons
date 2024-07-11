use anyhow::Result;
use chrono::NaiveDate;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnicAsnPopulationEntry {
    pub rank: u32,
    #[serde(rename = "AS")]
    pub asn: u32,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "CC")]
    pub country_code: String,
    #[serde(rename = "Users")]
    pub user_count: i64,
    #[serde(rename = "Percent of CC Pop")]
    pub percent_country: f64,
    #[serde(rename = "Percent of Internet")]
    pub percent_global: f64,
    #[serde(rename = "Samples")]
    pub sample_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApnicAsnPopulation {
    pub copyright: String,
    pub description: String,
    #[serde(rename = "Date", deserialize_with = "deserialize_date")]
    pub date: NaiveDate,
    #[serde(rename = "Window")]
    pub window: String,
    #[serde(rename = "Data")]
    pub data: Vec<ApnicAsnPopulationEntry>,
}

fn deserialize_date<'de, D>(d: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(d)?;
    NaiveDate::parse_from_str(string.as_str(), "%d/%m/%Y").map_err(de::Error::custom)
}

pub struct AsnPopulation {
    population_map: HashMap<u32, ApnicAsnPopulationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsnPopulationData {
    pub user_count: i64,
    pub percent_country: f64,
    pub percent_global: f64,
    pub sample_count: i64,
}

impl AsnPopulation {
    pub fn new() -> Result<Self> {
        let population: ApnicAsnPopulation =
            oneio::read_json_struct("https://stats.labs.apnic.net/cgi-bin/aspop?f=j")?;
        let mut population_map = HashMap::new();
        for entry in population.data {
            population_map.insert(entry.asn, entry);
        }
        Ok(AsnPopulation { population_map })
    }

    pub fn get(&self, asn: u32) -> Option<AsnPopulationData> {
        self.population_map
            .get(&asn)
            .map(|entry| AsnPopulationData {
                user_count: entry.user_count,
                percent_country: entry.percent_country,
                percent_global: entry.percent_global,
                sample_count: entry.sample_count,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_asn_population() {
        let population = AsnPopulation::new().unwrap();
        dbg!(population.get(15169).unwrap());
    }
}
