use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

const COLLECTOR_PEERS_URL: &str = "https://api.bgpkit.com/v3/peers/list";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrtCollectorPeersData {
    pub count: u32,
    pub data: Vec<MrtCollectorPeer>,
}

/// MRT collector meta information
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MrtCollectorPeer {
    /// latest available dated
    pub date: NaiveDate,
    /// collector peer IP
    pub ip: IpAddr,
    /// collector peer ASN
    pub asn: u32,
    /// collector name
    pub collector: String,
    /// number of IPv4 prefixes
    pub num_v4_pfxs: u32,
    /// number of IPv6 prefixes
    pub num_v6_pfxs: u32,
    /// number of connected ASNs
    pub num_connected_asns: u32,
}

impl MrtCollectorPeer {
    pub fn is_full_feed_v4(&self) -> bool {
        self.num_v4_pfxs >= 700_000
    }

    pub fn is_full_feed_v6(&self) -> bool {
        self.num_v6_pfxs >= 100_000
    }

    pub fn is_full_feed(&self) -> bool {
        self.is_full_feed_v4() || self.is_full_feed_v6()
    }
}

pub fn get_mrt_collector_peers() -> Result<Vec<MrtCollectorPeer>> {
    let peers: MrtCollectorPeersData = oneio::read_json_struct(COLLECTOR_PEERS_URL)?;

    Ok(peers.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_peers() {
        let mut peers = get_mrt_collector_peers().unwrap();
        assert!(!peers.is_empty());
        // sort peers by the number of connected ASNs
        peers.sort_by(|a, b| b.num_connected_asns.cmp(&a.num_connected_asns));
        // print top 10 peers
        for peer in peers.iter().take(10) {
            println!("{:?}", peer);
        }
    }
}
