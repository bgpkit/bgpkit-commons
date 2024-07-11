use crate::mrt_collectors::{MrtCollector, MrtCollectorProject, ToMrtCollector};
use anyhow::Result;
use chrono::DateTime;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RisCollector {
    pub id: i64,
    pub name: String,
    pub geographical_location: String,
    pub topological_location: String,
    pub activated_on: String,
    pub deactivated_on: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct Data {
    pub rrcs: Vec<RisCollector>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RisData {
    pub data: Data,
    pub status: String,
    pub status_code: i64,
}

impl ToMrtCollector for RisCollector {
    fn to_mrt_collector(&self) -> Option<MrtCollector> {
        let activated_on = match DateTime::parse_from_rfc3339(
            format!("{}-01T00:00:00Z", self.activated_on.as_str()).as_str(),
        ) {
            Ok(t) => t.naive_utc(),
            Err(_) => return None,
        };

        let deactivated_on = match self.deactivated_on.is_empty() {
            true => None,
            false => match DateTime::parse_from_rfc3339(
                format!("{}-01T00:00:00Z", self.deactivated_on.as_str()).as_str(),
            ) {
                Ok(t) => Some(t.naive_utc()),
                Err(_) => return None,
            },
        };

        let name = self.name.to_lowercase();

        let data_url = format!("https://data.ris.ripe.net/{}", name.as_str());

        let country = self
            .geographical_location
            .split(',')
            .map(|c| c.trim())
            .collect::<Vec<&str>>()[1]
            .to_string();

        Some(MrtCollector {
            data_url,
            name,
            project: MrtCollectorProject::RipeRis,
            activated_on,
            deactivated_on,
            country,
        })
    }
}

/// Get RIPE RIS mrt_collectors meta information
pub fn get_riperis_collectors() -> Result<Vec<MrtCollector>> {
    let data = oneio::read_json_struct::<RisData>("https://stat.ripe.net/data/rrc-info/data.json")?;
    Ok(data
        .data
        .rrcs
        .into_iter()
        .map(|c| c.to_mrt_collector().unwrap())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_riperis_collectors() {
        dbg!(get_riperis_collectors().unwrap());
    }
}
