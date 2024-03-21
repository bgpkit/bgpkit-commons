use bgpkit_commons::asnames::{get_asnames, AsName};
use bgpkit_commons::collectors::get_all_collectors;
use bgpkit_commons::countries::Countries;
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[clap(author, version)]
#[clap(propagate_version = true)]
#[command(arg_required_else_help(true))]
/// oneio reads files from local or remote locations with any compression.
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Export data to local files
    Export {
        /// output directory
        #[clap(short, long, default_value = ".")]
        output_dir: String,
    },
}

fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Export { output_dir } => {
            // strip output dir's trailing slash
            let output_dir = output_dir.trim_end_matches('/');
            std::fs::create_dir_all(output_dir).unwrap();

            info!("exporting MRT collectors data...");
            let collectors_vec = get_all_collectors().unwrap();
            let file_path = format!("{}/collectors.json", output_dir);
            let collectors_json = serde_json::to_string_pretty(&collectors_vec).unwrap();
            oneio::get_writer(&file_path)
                .unwrap()
                .write_all(collectors_json.as_bytes())
                .unwrap();

            info!("exporting countries data...");
            let countries_vec = Countries::new().unwrap().all_countries();
            let countries_file = format!("{}/countries.json", output_dir);
            let countries_json = serde_json::to_string_pretty(&countries_vec).unwrap();
            oneio::get_writer(&countries_file)
                .unwrap()
                .write_all(countries_json.as_bytes())
                .unwrap();

            info!("exporting ASNs data...");
            let asns_vec = get_asnames()
                .unwrap()
                .values()
                .cloned()
                .collect::<Vec<AsName>>();
            let asns_json = serde_json::to_string_pretty(&asns_vec).unwrap();
            let asns_file = format!("{}/asns.json", output_dir);
            oneio::get_writer(&asns_file)
                .unwrap()
                .write_all(asns_json.as_bytes())
                .unwrap();
        }
    }
}
