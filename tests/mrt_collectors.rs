#![cfg(feature = "mrt_collectors")]

#[test]
fn test_get_collectors() {
    let mut commons = bgpkit_commons::BgpkitCommons::new();
    commons.load_mrt_collectors().unwrap();

    let collectors = commons.mrt_collectors_all().unwrap();
    assert!(!collectors.is_empty());
}

#[test]
fn test_get_collector_peers() {
    let mut commons = bgpkit_commons::BgpkitCommons::new();
    commons.load_mrt_collector_peers().unwrap();
    let all_collector_peers = commons.mrt_collector_peers_all().unwrap();
    assert!(!all_collector_peers.is_empty());
    let full_feed_collector_peers = commons.mrt_collector_peers_full_feed().unwrap();
    assert!(!full_feed_collector_peers.is_empty());
    assert!(full_feed_collector_peers.len() < all_collector_peers.len());
}
