use bgpkit_commons::asnames::{get_asnames, AsName};
use std::collections::HashMap;

fn main() {
    let asnames: HashMap<u32, AsName> = get_asnames().unwrap();
    println!(
        "{}",
        serde_json::to_string_pretty(asnames.get(&400644).unwrap()).unwrap()
    );
    assert_eq!(
        asnames.get(&3333).unwrap().name,
        "RIPE-NCC-AS Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"
    );
    assert_eq!(asnames.get(&400644).unwrap().name, "BGPKIT-LLC");
    assert_eq!(asnames.get(&400644).unwrap().country, "US");
}
