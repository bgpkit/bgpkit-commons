//! # Module: countries
//!
//! This module provides functionalities related to countries. It fetches country data from the GeoNames database and provides various lookup methods to retrieve country information.
//!
//! ## Structures
//!
//! ### Country
//!
//! This structure represents a country with the following fields:
//!
//! - `code`: A 2-letter country code.
//! - `code3`: A 3-letter country code.
//! - `name`: The name of the country.
//! - `capital`: The capital city of the country.
//! - `continent`: The continent where the country is located.
//! - `ltd`: The country's top-level domain. This field is optional.
//! - `neighbors`: A list of neighboring countries represented by their 2-letter country codes.
//!
//! ### Countries
//!
//! This structure represents a collection of countries. It provides various methods to lookup and retrieve country information.
use crate::errors::{data_sources, load_methods, modules};
use crate::{BgpkitCommons, BgpkitCommonsError, LazyLoadable, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Country data structure
///
/// Information coming from <https://download.geonames.org/export/dump/countryInfo.txt>
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Countries data structure that contains a collection of countries
#[derive(Debug, Clone)]
pub struct Countries {
    countries: HashMap<String, Country>,
}

const DATA_URL: &str = "https://download.geonames.org/export/dump/countryInfo.txt";

impl Countries {
    pub fn new() -> Result<Self> {
        let mut countries: Vec<Country> = vec![];
        for line in oneio::read_lines(DATA_URL)? {
            let text = line.ok().ok_or_else(|| {
                BgpkitCommonsError::data_source_error(data_sources::GEONAMES, "error reading line")
            })?;
            if text.trim() == "" || text.starts_with('#') {
                continue;
            }
            let splits: Vec<&str> = text.split('\t').collect();
            if splits.len() != 19 {
                return Err(BgpkitCommonsError::invalid_format(
                    "countries data",
                    text.as_str(),
                    "row missing fields",
                ));
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
                .split(',')
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

impl LazyLoadable for Countries {
    fn reload(&mut self) -> Result<()> {
        *self = Countries::new().map_err(|e| {
            BgpkitCommonsError::data_source_error(data_sources::GEONAMES, e.to_string())
        })?;
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        !self.countries.is_empty()
    }

    fn loading_status(&self) -> &'static str {
        if self.is_loaded() {
            "Countries data loaded"
        } else {
            "Countries data not loaded"
        }
    }
}

impl BgpkitCommons {
    pub fn country_all(&self) -> Result<Vec<Country>> {
        if self.countries.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::COUNTRIES,
                load_methods::LOAD_COUNTRIES,
            ));
        }

        Ok(self.countries.as_ref().unwrap().all_countries())
    }

    pub fn country_by_code(&self, code: &str) -> Result<Option<Country>> {
        if self.countries.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::COUNTRIES,
                load_methods::LOAD_COUNTRIES,
            ));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_code(code))
    }

    pub fn country_by_name(&self, name: &str) -> Result<Vec<Country>> {
        if self.countries.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::COUNTRIES,
                load_methods::LOAD_COUNTRIES,
            ));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_name(name))
    }

    pub fn country_by_code3(&self, code: &str) -> Result<Option<Country>> {
        if self.countries.is_none() {
            return Err(BgpkitCommonsError::module_not_loaded(
                modules::COUNTRIES,
                load_methods::LOAD_COUNTRIES,
            ));
        }
        Ok(self.countries.as_ref().unwrap().lookup_by_code(code))
    }
}
