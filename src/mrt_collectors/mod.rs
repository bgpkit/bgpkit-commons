/*!
Module for getting meta information for the public MRT mrt_collectors.

Currently supported MRT collector projects:
- RIPE RIS
- RouteViews

*/

use anyhow::{Result, anyhow};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

mod peers;
mod riperis;
mod routeviews;

use crate::BgpkitCommons;
pub use peers::{MrtCollectorPeer, get_mrt_collector_peers};
pub use riperis::get_riperis_collectors;
pub use routeviews::get_routeviews_collectors;

/// MRT collector project enum
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MrtCollectorProject {
    RouteViews,
    RipeRis,
}

// Custom serialization function for the `age` field
fn serialize_project<S>(project: &MrtCollectorProject, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(match project {
        MrtCollectorProject::RouteViews => "routeview",
        MrtCollectorProject::RipeRis => "riperis",
    })
}

impl Display for MrtCollectorProject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MrtCollectorProject::RouteViews => {
                write!(f, "routeviews")
            }
            MrtCollectorProject::RipeRis => {
                write!(f, "riperis")
            }
        }
    }
}

/// MRT collector meta information
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MrtCollector {
    /// name of the collector
    pub name: String,
    /// collector project
    #[serde(serialize_with = "serialize_project")]
    pub project: MrtCollectorProject,
    /// MRT data files root URL
    pub data_url: String,
    /// collector activation timestamp
    pub activated_on: NaiveDateTime,
    /// collector deactivation timestamp (None for active mrt_collectors)
    pub deactivated_on: Option<NaiveDateTime>,
    /// country where the collect runs in
    pub country: String,
}

pub trait ToMrtCollector {
    fn to_mrt_collector(&self) -> Option<MrtCollector>;
}

impl PartialOrd<Self> for MrtCollector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MrtCollector {
    fn cmp(&self, other: &Self) -> Ordering {
        self.activated_on.cmp(&other.activated_on)
    }
}

/// Get all MRT mrt_collectors from all data sources
pub fn get_all_collectors() -> Result<Vec<MrtCollector>> {
    let mut collectors = vec![];
    collectors.extend(get_routeviews_collectors()?);
    collectors.extend(get_riperis_collectors()?);
    Ok(collectors)
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

    pub fn mrt_collector_peers_all(&self) -> Result<Vec<MrtCollectorPeer>> {
        if self.mrt_collector_peers.is_none() {
            return Err(anyhow!(
                "mrt_collector_peers is not loaded, call commons.load_mrt_collector_peers() first"
            ));
        }
        Ok(self.mrt_collector_peers.clone().unwrap())
    }

    pub fn mrt_collector_peers_full_feed(&self) -> Result<Vec<MrtCollectorPeer>> {
        if self.mrt_collector_peers.is_none() {
            return Err(anyhow!("mrt_collector_peers is not loaded"));
        }
        // Filter out mrt_collectors that have full feed
        Ok(self
            .mrt_collector_peers
            .as_ref()
            .unwrap()
            .iter()
            .filter(|x| x.is_full_feed())
            .cloned()
            .collect())
    }
}
