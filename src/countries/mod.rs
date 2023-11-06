use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Country data structure
///
/// Information coming from <https://download.geonames.org/export/dump/countryInfo.txt>
#[derive(Debug, Clone)]
pub struct Country {
    /// 2-letter country code
    pub code: String,
    /// 3-letter country code
    pub code3: String,
    /// Country name
    pub name: String,
    /// Capital city
    pub capital: String,
    /// Continent
    pub continent: String,
    /// Country's top-level domain
    pub ltd: Option<String>,
    /// Neighboring countries in 2-letter country code
    pub neighbors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Countries {
    countries: HashMap<String, Country>,
}

const DATA_URL: &str = "https://download.geonames.org/export/dump/countryInfo.txt";

impl Countries {
    pub fn new() -> Result<Self> {
        let mut countries: Vec<Country> = vec![];
        for line in oneio::read_lines(DATA_URL)? {
            let text = line.ok().ok_or(anyhow!("error reading line"))?;
            if text.trim() == "" || text.starts_with('#') {
                continue;
            }
            let splits: Vec<&str> = text.split("\t").collect();
            if splits.len() != 19 {
                return Err(anyhow!("row missing fields: {}", text.as_str()));
            }
            let code = splits[0].to_string();
            let code3 = splits[1].to_string();
            let name = splits[4].to_string();
            let capital = splits[5].to_string();
            let continent = splits[8].to_string();
            let ltd = match splits[9] {
                "" => None,
                d => Some(d.to_string()),
            };
            let neighbors = splits[17]
                .split(",")
                .map(|x| x.to_string())
                .collect::<Vec<String>>();
            countries.push(Country {
                code,
                code3,
                name,
                capital,
                continent,
                ltd,
                neighbors,
            })
        }

        let mut countries_map: HashMap<String, Country> = HashMap::new();
        for country in countries {
            countries_map.insert(country.code.clone(), country);
        }
        Ok(Countries {
            countries: countries_map,
        })
    }

    /// Lookup country by 2-letter country code
    pub fn lookup_by_code(&self, code: &str) -> Option<Country> {
        self.countries.get(code).cloned()
    }

    /// Lookup country by country name
    ///
    /// This function is case-insensitive and search for countries with name that contains the given name string
    pub fn lookup_by_name(&self, name: &str) -> Vec<Country> {
        let lower_name = name.to_lowercase();
        let mut countries: Vec<Country> = vec![];
        for country in self.countries.values() {
            if country.name.to_lowercase().contains(&lower_name) {
                countries.push(country.clone());
            }
        }
        countries
    }

    /// Get all countries
    pub fn all_countries(&self) -> Vec<Country> {
        self.countries.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_countries() {
        let countries = Countries::new().unwrap();

        assert_eq!(
            countries.lookup_by_code("US").unwrap().name,
            "United States"
        );
        assert_eq!(countries.lookup_by_name("united states").len(), 2);
        assert_eq!(countries.lookup_by_name("united kingdom").len(), 1);
    }
}
