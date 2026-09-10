//! `pg_inserter`: bulk-load BGP reference data from bgpkit-commons into
//! PostgreSQL.
//!
//! Snapshot-only, per-source refresh: each task loads one data source and
//! atomically replaces its table via a staging COPY + swap, so readers never
//! see an empty or half-written table. Every run is recorded in
//! `meta.ingest_run` for provenance.
//!
//! Privileges: the connecting role needs `USAGE` and `CREATE` on each target
//! schema (or database-level `CREATE` so the loader can create them on the
//! first run); the loader owns the tables it creates. The swap replaces each
//! live table with a new OID, so per-table `GRANT`s do not carry over —
//! consumers must be granted read access via default privileges, which the
//! renamed staging tables inherit at creation time:
//!
//! ```sql
//! ALTER DEFAULT PRIVILEGES FOR ROLE <loader> IN SCHEMA <schema>
//!     GRANT SELECT ON TABLES TO <consumer>;
//! ```
//!
//! Views or foreign keys depending on a live table block its swap; the
//! loader fails loudly rather than silently dropping them.

mod asndata;
mod irr;
mod peeringdb;
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
    /// peeringdb schema: full PeeringDB mirror (12 tables)
    Peeringdb,
    /// irr schema: full IRR RPSL object store (snapshot-only)
    Irr,
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

    let mut exit_code = 0;
    if cli.command == Commands::Peeringdb {
        let data_as_of = chrono::Utc::now();
        let tables = match tokio::task::spawn_blocking({
            let data_as_of = data_as_of.to_rfc3339();
            move || peeringdb::load_all(&data_as_of)
        })
        .await
        {
            Ok(Ok(tables)) => tables,
            Ok(Err(e)) => {
                eprintln!("peeringdb: data load failed: {e}");
                // Best-effort family-level provenance row for a load that
                // failed before any table write.
                refresh::record_family_error_run(
                    "peeringdb",
                    "peeringdb.net",
                    &database_url,
                    &format!("data load failed: {e}"),
                )
                .await;
                exit(11);
            }
            Err(e) => {
                eprintln!("peeringdb: data loader task failed: {e}");
                refresh::record_family_error_run(
                    "peeringdb",
                    "peeringdb.net",
                    &database_url,
                    &format!("data loader task failed: {e}"),
                )
                .await;
                exit(11);
            }
        };
        if let Err(code) = refresh::run_tables(tables, data_as_of, &database_url).await {
            exit_code = code;
        }
    } else if cli.command == Commands::Irr {
        if let Err(code) = irr::run(&database_url).await {
            exit_code = code;
        }
    } else {
        let tasks: &[Task] = match cli.command {
            Commands::Asnames => &[Task::Asnames],
            Commands::As2org => &[Task::As2org],
            Commands::Population => &[Task::Population],
            Commands::Hegemony => &[Task::Hegemony],
            Commands::Delegated => &[Task::Delegated],
            Commands::Asndata => asndata::ALL_TASKS,
            Commands::Peeringdb | Commands::Irr => unreachable!(),
        };
        for task in tasks {
            let spec = task.spec();
            let load = task.load_fn();
            if let Err(code) = refresh::run_task(spec, load, &database_url).await {
                // one task failing must not stop the remaining tasks
                exit_code = code;
            }
        }
    }
    if exit_code != 0 {
        exit(exit_code);
    }
}
