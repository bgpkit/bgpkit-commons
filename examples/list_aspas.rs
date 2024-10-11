use serde_json::json;

fn main() {
    let cf_data = bgpkit_commons::rpki::CfData::new().unwrap();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!(cf_data.aspas)).unwrap()
    );
}
