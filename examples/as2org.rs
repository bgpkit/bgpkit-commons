use serde_json::{Value, json};
use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();
    info!("loading asn info data ...");
    let mut commons = bgpkit_commons::BgpkitCommons::new();
    commons.load_asinfo(true, false, false).unwrap();
    commons.load_countries().unwrap();
    let as_info_map = commons.asinfo_all().unwrap();

    let path = "as2org.jsonl";

    info!("writing asn info data to '{}' ...", path);
    let mut writer = oneio::get_writer(path).unwrap();
    let mut info_vec = as_info_map.values().collect::<Vec<_>>();
    info_vec.sort_by(|a, b| a.asn.cmp(&b.asn));
    let values_vec: Vec<Value> = info_vec.into_iter().map(|v| json!(v)).collect();
    for as_info in values_vec {
        writeln!(writer, "{}", serde_json::to_string(&as_info).unwrap()).unwrap();
    }
    drop(writer);
}
