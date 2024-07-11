fn main() {
    tracing_subscriber::fmt().init();

    let mut commons = bgpkit_commons::BgpkitCommons::new();
    commons.load_asinfo(false, false, false).unwrap();

    println!(
        "{}",
        serde_json::to_string_pretty(&commons.asinfo_get(400644).unwrap()).unwrap()
    );
    assert_eq!(
        commons.asinfo_get(3333).unwrap().unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );
    assert_eq!(
        commons.asinfo_get(400644).unwrap().unwrap().name,
        "BGPKIT-LLC"
    );
    assert_eq!(commons.asinfo_get(400644).unwrap().unwrap().country, "US");
}
