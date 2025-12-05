#![cfg(feature = "asinfo")]

/// Integration test for basic AS information retrieval.
#[test]
fn test_basic_info() {
    // Create a new instance of BgpkitCommons.
    let mut commons = bgpkit_commons::BgpkitCommons::new();

    // Load AS information with default parameters.
    // no as2org, no population data, no as-hegemony
    commons.load_asinfo(false, false, false, false).unwrap();

    // Assert that the AS name for AS number 3333 is correct.
    assert_eq!(
        commons.asinfo_get(3333).unwrap().unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );

    // Assert that the AS name for AS number 400644 is correct.
    assert_eq!(
        commons.asinfo_get(400644).unwrap().unwrap().name,
        "BGPKIT-LLC"
    );

    // Assert that the country for AS number 400644 is correct.
    assert_eq!(commons.asinfo_get(400644).unwrap().unwrap().country, "US");

    // Retrieve all AS information and assert that the AS name for AS number 3333 is correct.
    let all_asinfo = commons.asinfo_all().unwrap();
    assert_eq!(
        all_asinfo.get(&3333).unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );
}

#[test]
fn test_loading_cached() {
    // Create a new instance of BgpkitCommons.
    let mut commons = bgpkit_commons::BgpkitCommons::new();

    // Load AS information previously generated and cached.
    commons.load_asinfo_cached().unwrap();

    // Assert that the AS name for AS number 3333 is correct.
    assert_eq!(
        commons.asinfo_get(3333).unwrap().unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );

    let bgpkit_info = commons.asinfo_get(400644).unwrap().unwrap();

    // Assert that the AS name for AS number 400644 is correct.
    assert_eq!(bgpkit_info.name, "BGPKIT-LLC");

    // Assert that the country for AS number 400644 is correct.
    assert_eq!(bgpkit_info.country, "US");

    // Assert that the additional datatsets are also loaded.
    assert!(bgpkit_info.peeringdb.is_some());
    assert!(bgpkit_info.hegemony.is_some());
    assert!(bgpkit_info.as2org.is_some());

    // make sure the preferred name is retrieved correctly.
    assert_eq!(bgpkit_info.get_preferred_name(), "BGPKIT");

    assert_eq!(
        bgpkit_info.peeringdb.unwrap().irr_as_set.unwrap(),
        "AS400644:AS-BGPKIT"
    );
}
