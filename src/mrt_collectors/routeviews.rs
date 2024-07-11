//! RouteViews mrt_collectors information
//!
//! API source: <https://api.routeviews.org/collector/?format=json>

use crate::mrt_collectors::{MrtCollector, MrtCollectorProject, ToMrtCollector};
use anyhow::Result;
use chrono::DateTime;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RVCollector {
    pub url: String,
    pub name: String,
    pub label: String,
    #[serde(rename(deserialize = "type"))]
    pub software: String,
    pub lat: String,
    pub lng: String,
    pub country: String,
    pub rir_region: String,
    pub ipv4: String,
    pub ipv6: String,
    pub bmp: i64,
    pub rpki: i64,
    pub ix_speed: i64,
    pub multihop: i64,
    pub installed: String,
    pub removed: Option<String>,
    pub scamper: i64,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RvData {
    pub count: i64,
    #[serde(rename(deserialize = "results"))]
    pub data: Vec<RVCollector>,
}

impl ToMrtCollector for RVCollector {
    fn to_mrt_collector(&self) -> Option<MrtCollector> {
        if self.name.as_str() == "route-views" {
            return None;
        }

        let activated_on = match DateTime::parse_from_rfc3339(self.installed.as_str()) {
            Ok(t) => t.naive_utc(),
            Err(_) => return None,
        };

        let deactivated_on = match &self.removed {
            None => None,
            Some(ts_str) => {
                let ts = match DateTime::parse_from_rfc3339(ts_str) {
                    Ok(t) => t.naive_utc(),
                    Err(_) => return None,
                };
                Some(ts)
            }
        };

        let data_url = match self.name.as_str() {
            "route-views2" => "http://archive.routeviews.org/bgpdata".to_string(),
            c => format!("http://archive.routeviews.org/{}/bgpdata", c),
        };

        Some(MrtCollector {
            data_url,
            name: self.name.clone(),
            project: MrtCollectorProject::RouteViews,
            activated_on,
            deactivated_on,
            country: self.country.clone(),
        })
    }
}

/// Get RouteViews mrt_collectors meta information
pub fn get_routeviews_collectors() -> Result<Vec<MrtCollector>> {
    let data =
        oneio::read_json_struct::<RvData>("https://api.routeviews.org/collector/?format=json")?;
    Ok(data
        .data
        .into_iter()
        // exclude the Cisco collector that does not provide MRT archive
        .filter(|c| c.name.as_str() != "route-views")
        .map(|c| c.to_mrt_collector().unwrap())
        .collect())
}
