/// Integration test for basic AS information retrieval.
#[test]
fn test_basic_info() {
    // Create a new instance of BgpkitCommons.
    let mut commons = bgpkit_commons::BgpkitCommons::new();

    // Load AS information with default parameters.
    // no as2org, no population data, no as-hegemony
    commons.load_asinfo(false, false, false).unwrap();

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
