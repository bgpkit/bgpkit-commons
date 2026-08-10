//! CLI binary for exporting all bgpkit-commons data sources to Parquet.
//!
//! Usage:
//!   bgpkit-export --output-dir ./commons-export
//!   bgpkit-export --output-dir ./commons-export --with-peeringdb --with-irr --with-rpki

use std::path::PathBuf;

use bgpkit_commons::export;
use bgpkit_commons::{BgpkitCommons, asinfo::AsInfoBuilder};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bgpkit-export",
    version,
    about = "Export all bgpkit-commons data to Parquet"
)]
struct Cli {
    /// Output directory for Parquet files
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Include as2org (CAIDA AS-to-organization) data
    #[arg(long)]
    with_as2org: bool,

    /// Include AS population (APNIC) data
    #[arg(long)]
    with_population: bool,

    /// Include AS hegemony (IIJ IHR) data
    #[arg(long)]
    with_hegemony: bool,

    /// Include PeeringDB tables (requires PEERINGDB_API_KEY)
    #[arg(long)]
    with_peeringdb: bool,

    /// Include IRR records (large, downloads from all IRR registries)
    #[arg(long)]
    with_irr: bool,

    /// Include RPKI ROA + ASPA snapshot (Cloudflare real-time)
    #[arg(long)]
    with_rpki: bool,

    /// Also write asninfo.jsonl (legacy merged AsInfo JSONL)
    #[arg(long)]
    with_asninfo_jsonl: bool,

    /// Subcommand selector (reserved for future use)
    #[command(subcommand)]
    _command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {}

fn main() {
    tracing_subscriber::fmt().with_ansi(false).init();
    let cli = Cli::parse();

    let output_dir = &cli.output_dir;
    std::fs::create_dir_all(output_dir).expect("failed to create output directory");

    let mut commons = BgpkitCommons::new();
    let mut failures: Vec<String> = Vec::new();

    // Load core asinfo (asn.txt always loaded; optional sources gated by flags)
    tracing::info!("loading asinfo data...");
    let mut builder = AsInfoBuilder::new();
    if cli.with_as2org {
        builder = builder.with_as2org();
    }
    if cli.with_population {
        builder = builder.with_population();
    }
    if cli.with_hegemony {
        builder = builder.with_hegemony();
    }
    if cli.with_peeringdb {
        builder = builder.with_peeringdb();
    }
    match commons.load_asinfo_with(builder) {
        Ok(()) => {}
        Err(e) => {
            tracing::error!("failed to load asinfo: {e}");
            failures.push(format!("asinfo: {e}"));
        }
    }

    // Load modules needed for export
    tracing::info!("loading countries...");
    if let Err(e) = commons.load_countries() {
        tracing::warn!("failed to load countries: {e}");
        failures.push(format!("countries: {e}"));
    }

    tracing::info!("loading bogons...");
    if let Err(e) = commons.load_bogons() {
        tracing::warn!("failed to load bogons: {e}");
        failures.push(format!("bogons: {e}"));
    }

    tracing::info!("loading mrt_collectors...");
    if let Err(e) = commons.load_mrt_collectors() {
        tracing::warn!("failed to load mrt_collectors: {e}");
        failures.push(format!("mrt_collectors: {e}"));
    }

    tracing::info!("loading as2rel...");
    if let Err(e) = commons.load_as2rel() {
        tracing::warn!("failed to load as2rel: {e}");
        failures.push(format!("as2rel: {e}"));
    }

    // ---- Export core files ----
    let mut exported: Vec<String> = Vec::new();

    export_source("asn_names", &mut exported, &mut failures, || {
        export::asn_names(output_dir, &commons)
    });
    export_source("countries", &mut exported, &mut failures, || {
        export::countries(output_dir, &commons)
    });
    export_source("iana_bogons", &mut exported, &mut failures, || {
        export::iana_bogons(output_dir, &commons)
    });
    export_source("mrt_collectors", &mut exported, &mut failures, || {
        export::mrt_collectors(output_dir, &commons)
    });
    export_source("as_relationships", &mut exported, &mut failures, || {
        export::as_relationships(output_dir, &commons)
    });
    export_source("rir_delegated", &mut exported, &mut failures, || {
        export::rir_delegated(output_dir)
    });

    // ---- Optional: asninfo.jsonl (legacy) ----
    if cli.with_asninfo_jsonl {
        tracing::info!("writing asninfo.jsonl...");
        match write_asninfo_jsonl(output_dir, &commons) {
            Ok(()) => exported.push("asninfo.jsonl".to_string()),
            Err(e) => {
                tracing::error!("failed to write asninfo.jsonl: {e}");
                failures.push(format!("asninfo.jsonl: {e}"));
            }
        }
    }

    // ---- Optional: PeeringDB ----
    if cli.with_peeringdb {
        tracing::info!("exporting PeeringDB tables...");
        tracing::warn!("PeeringDB export is not yet implemented in this PR");
        failures.push("peeringdb: not yet implemented".to_string());
    }

    // ---- Optional: IRR ----
    if cli.with_irr {
        tracing::info!("exporting IRR records...");
        tracing::warn!("IRR export is not yet implemented in this PR");
        failures.push("irr: not yet implemented".to_string());
    }

    // ---- Optional: RPKI ----
    if cli.with_rpki {
        tracing::info!("exporting RPKI ROAs + ASPAs...");
        tracing::warn!("RPKI export is not yet implemented in this PR");
        failures.push("rpki: not yet implemented".to_string());
    }

    // ---- Write manifest ----
    let manifest = build_manifest(&exported, &failures);
    let manifest_path = output_dir.join("manifest.json");
    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            std::fs::write(&manifest_path, json).expect("failed to write manifest.json");
            tracing::info!("wrote manifest to {}", manifest_path.display());
        }
        Err(e) => {
            tracing::error!("failed to serialize manifest: {e}");
        }
    }

    // ---- Summary ----
    tracing::info!("export complete: {} files written", exported.len());
    if !failures.is_empty() {
        tracing::warn!("{} failures:", failures.len());
        for f in &failures {
            tracing::warn!("  - {f}");
        }
    }
}

fn export_source(
    name: &str,
    exported: &mut Vec<String>,
    failures: &mut Vec<String>,
    f: impl FnOnce() -> Result<(), export::ExportError>,
) {
    tracing::info!("exporting {name}...");
    match f() {
        Ok(()) => {
            let filename = match name {
                "asn_names" => "asn_names.parquet",
                "countries" => "countries.parquet",
                "iana_bogons" => "iana_bogons.parquet",
                "mrt_collectors" => "mrt_collectors.parquet",
                "as_relationships" => "as_relationships.parquet",
                "rir_delegated" => "rir_delegated.parquet",
                _ => name,
            };
            exported.push(filename.to_string());
            tracing::info!("  wrote {filename}");
        }
        Err(e) => {
            tracing::error!("failed to export {name}: {e}");
            failures.push(format!("{name}: {e}"));
        }
    }
}

fn write_asninfo_jsonl(
    dir: &std::path::Path,
    commons: &BgpkitCommons,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("asninfo.jsonl");
    let all = commons.asinfo_all()?;
    let mut file = std::fs::File::create(&path)?;
    let sorted: Vec<_> = all
        .keys()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for asn in sorted {
        if let Some(info) = all.get(asn) {
            let json = serde_json::to_string(info)?;
            use std::io::Write;
            writeln!(file, "{json}")?;
        }
    }
    Ok(())
}

fn build_manifest(exported: &[String], failures: &[String]) -> serde_json::Value {
    use serde_json::json;
    let now = chrono::Utc::now();
    json!({
        "format_version": 1,
        "generated_at": now.to_rfc3339(),
        "tables": exported.iter().map(|name| {
            json!({
                "name": name,
            })
        }).collect::<Vec<_>>(),
        "partial": !failures.is_empty(),
        "failures": failures,
    })
}
