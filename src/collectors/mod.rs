/*!
Module for getting meta information for the public MRT collectors.

Currently supported MRT collector projects:
- RIPE RIS
- RouteViews

*/

use anyhow::Result;
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

mod riperis;
mod routeviews;

pub use riperis::get_riperis_collectors;
pub use routeviews::get_routeviews_collectors;

/// MRT collector project enum
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MrtCollectorProject {
    RouteViews,
    RipeRis,
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
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MrtCollector {
    /// name of the collector
    pub name: String,
    /// collector project
    pub project: MrtCollectorProject,
    /// MRT data files root URL
    pub data_url: String,
    /// collector activation timestamp
    pub activated_on: NaiveDateTime,
    /// collector deactivation timestamp (None for active collectors)
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

/// Get all MRT collectors from all data sources
pub fn get_all_collectors() -> Result<Vec<MrtCollector>> {
    let mut collectors = vec![];
    collectors.extend(get_routeviews_collectors()?);
    collectors.extend(get_riperis_collectors()?);
    Ok(collectors)
}
