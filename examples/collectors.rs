use bgpkit_commons::collectors::{
    get_all_collectors, get_riperis_collectors, get_routeviews_collectors,
};

fn main() {
    println!("get route views collectors");
    let mut routeviews_collectors = get_routeviews_collectors().unwrap();
    routeviews_collectors.sort();
    let earliest = routeviews_collectors.first().unwrap();
    let latest = routeviews_collectors.last().unwrap();
    println!("\t total of {} collectors", routeviews_collectors.len());
    println!(
        "\t earliest collector: {} (activated on {})",
        earliest.name, earliest.activated_on
    );
    println!(
        "\t latest collector: {} (activated on {})",
        latest.name, latest.activated_on
    );

    println!();

    println!("get ripe ris collectors");
    let mut riperis_collectors = get_riperis_collectors().unwrap();
    riperis_collectors.sort();
    let earliest = riperis_collectors.first().unwrap();
    let latest = riperis_collectors.last().unwrap();
    println!("\t total of {} collectors", riperis_collectors.len());
    println!(
        "\t earliest collector: {} (activated on {})",
        earliest.name, earliest.activated_on
    );
    println!(
        "\t latest collector: {} (activated on {})",
        latest.name, latest.activated_on
    );

    println!();
    println!("get all collectors");

    let mut all_collectors = get_all_collectors().unwrap();
    all_collectors.sort();
    let earliest = all_collectors.first().unwrap();
    let latest = all_collectors.last().unwrap();
    println!("\t total of {} collectors", all_collectors.len());
    println!(
        "\t earliest collector: {} (activated on {})",
        earliest.name, earliest.activated_on
    );
    println!(
        "\t latest collector: {} (activated on {})",
        latest.name, latest.activated_on
    );
}
