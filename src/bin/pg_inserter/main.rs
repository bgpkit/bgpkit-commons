//! `pg_inserter`: bulk-load BGP reference data from bgpkit-commons into
//! PostgreSQL.
//!
//! Snapshot-only, per-source refresh: each task loads one data source and
//! atomically replaces its table via a staging COPY + swap, so readers never
//! see an empty or half-written table. Every run is recorded in
//! `meta.ingest_run` for provenance.

mod asndata;
mod refresh;

use asndata::Task;
use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(
    name = "pg_inserter",
    version,
    about = "Bulk-load BGP reference data into PostgreSQL"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// PostgreSQL connection URL (defaults to the DATABASE_URL environment variable)
    #[arg(long, env = "DATABASE_URL", global = true)]
    database_url: Option<String>,
}

#[derive(Subcommand, Clone, Copy, PartialEq, Eq)]
enum Commands {
    /// asndata.asnames: RIPE NCC ASN names and countries
    Asnames,
    /// asndata.as2org: CAIDA AS-to-organization mapping
    As2org,
    /// asndata.population: APNIC ASN population estimates
    Population,
    /// asndata.hegemony: IIJ IHR hegemony scores
    Hegemony,
    /// asndata.delegated: RIR delegated statistics (full records)
    Delegated,
    /// Run every asndata task sequentially
    Asndata,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_ansi(false).init();
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    let Some(database_url) = cli.database_url else {
        eprintln!(
            "DATABASE_URL is not set; pass --database-url or set the DATABASE_URL environment variable"
        );
        exit(10);
    };

    let tasks: &[Task] = match cli.command {
        Commands::Asnames => &[Task::Asnames],
        Commands::As2org => &[Task::As2org],
        Commands::Population => &[Task::Population],
        Commands::Hegemony => &[Task::Hegemony],
        Commands::Delegated => &[Task::Delegated],
        Commands::Asndata => asndata::ALL_TASKS,
    };

    let mut exit_code = 0;
    for task in tasks {
        let spec = task.spec();
        let load = task.load_fn();
        if let Err(code) = refresh::run_task(spec, load, &database_url).await {
            // one task failing must not stop the remaining tasks
            exit_code = code;
        }
    }
    if exit_code != 0 {
        exit(exit_code);
    }
}
