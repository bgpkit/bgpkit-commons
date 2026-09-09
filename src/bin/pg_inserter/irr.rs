//! `irr` schema: full IRR RPSL object store (snapshot-only).
//!
//! One table `irr.object` holds every RPSL object from the default source set
//! (RIRs + RADB), streamed: split-file registries (RIPE, APNIC) are fetched
//! per object type, whole-database registries (ARIN, LACNIC, AFRINIC, RADB)
//! are fetched once and parsed in a single pass. Rows carry the source, the
//! object type, the primary-key attribute value, the complete ordered
//! attribute list as `record` JSONB, and provenance columns. The volume
//! (tens of millions of route objects) requires the streaming COPY path.

use super::refresh::{SOURCE_REVISION, TableSpec, build_csv_line};
use bgpkit_commons::irr::{self, DumpFormat, IrrObjectType, IrrRecord};

const OBJECT_SPEC: TableSpec = TableSpec {
    task: "irr.object",
    table: "irr.object",
    staging_ddl: "CREATE TABLE irr.object_staging (
        source text NOT NULL,
        object_type text NOT NULL,
        primary_key text NOT NULL,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "source, object_type, primary_key, record, data_as_of, source_revision",
    index_ddl: &[
        "CREATE INDEX object_staging_type_source_idx ON irr.object_staging (object_type, source)",
    ],
    index_swaps: &[(
        "irr.object_staging_type_source_idx",
        "object_type_source_idx",
    )],
};

const ALL_TYPES: &[IrrObjectType] = &[
    IrrObjectType::AutNum,
    IrrObjectType::Route,
    IrrObjectType::Route6,
    IrrObjectType::AsSet,
    IrrObjectType::RouteSet,
    IrrObjectType::Mntner,
];

/// Run the IRR snapshot task. The producer streams parsed objects from every
/// default source into the staging table.
pub(crate) async fn run(database_url: &str) -> Result<(), i32> {
    let data_as_of = chrono::Utc::now().to_rfc3339();
    refresh_streaming(OBJECT_SPEC, data_as_of, database_url).await
}

async fn refresh_streaming(
    spec: TableSpec,
    data_as_of: String,
    database_url: &str,
) -> Result<(), i32> {
    let produce = move |tx: tokio::sync::mpsc::Sender<Result<String, String>>| {
        produce_objects(tx, &data_as_of)
    };
    super::refresh::run_streaming_task(spec, produce, database_url).await
}

/// Stream every object from every default IRR source. Runs on the blocking
/// pool: the RPSL dumps are fetched and parsed synchronously.
fn produce_objects(
    tx: tokio::sync::mpsc::Sender<Result<String, String>>,
    data_as_of: &str,
) -> Result<(), String> {
    for source in irr::default_sources() {
        match source.format {
            DumpFormat::SplitFiles => {
                for object_type in ALL_TYPES {
                    let reader = irr::fetch(&source, *object_type)
                        .map_err(|e| format!("failed to fetch {}: {e}", source.name))?;
                    let format = reader.dump_url.format;
                    for record in irr::parse_reader(reader, format) {
                        let record =
                            record.map_err(|e| format!("failed to parse {}: {e}", source.name))?;
                        tx.blocking_send(Ok(object_line(source.name, &record, data_as_of)))
                            .map_err(|e| format!("row channel closed: {e}"))?;
                    }
                }
            }
            DumpFormat::WholeDb => {
                // One whole-DB file contains every object type; fetch once.
                let reader = irr::fetch(&source, IrrObjectType::AutNum)
                    .map_err(|e| format!("failed to fetch {}: {e}", source.name))?;
                let format = reader.dump_url.format;
                for record in irr::parse_reader(reader, format) {
                    let record =
                        record.map_err(|e| format!("failed to parse {}: {e}", source.name))?;
                    tx.blocking_send(Ok(object_line(source.name, &record, data_as_of)))
                        .map_err(|e| format!("row channel closed: {e}"))?;
                }
            }
        }
    }
    Ok(())
}

fn object_line(source: &str, record: &IrrRecord, data_as_of: &str) -> String {
    // The first attribute is the object's primary key in RPSL.
    let primary_key = record
        .attributes
        .first()
        .map(|attr| attr.value.clone())
        .unwrap_or_default();
    let record_json = serde_json::to_string(record).unwrap_or_default();
    let fields = vec![
        Some(source.to_string()),
        Some(record.object_type.clone()),
        Some(primary_key),
        Some(record_json),
        Some(data_as_of.to_string()),
        Some(SOURCE_REVISION.to_string()),
    ];
    build_csv_line(&fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bgpkit_commons::irr::IrrAttribute;

    #[test]
    fn object_line_shape() {
        let record = IrrRecord {
            object_type: "route".to_string(),
            attributes: vec![
                IrrAttribute {
                    name: "route".to_string(),
                    value: "1.0.0.0/24".to_string(),
                },
                IrrAttribute {
                    name: "origin".to_string(),
                    value: "AS13335".to_string(),
                },
            ],
        };
        let line = object_line("RADB", &record, "2026-09-09T00:00:00+00:00");
        assert!(line.starts_with("\"RADB\",\"route\",\"1.0.0.0/24\",\"{"));
        // inner JSON quotes doubled by CSV escaping; attributes serialize as
        // {name, value} pairs
        assert!(line.contains("\"\"name\"\":\"\"route\"\""));
        assert!(line.contains("\"\"value\"\":\"\"AS13335\"\""));
        assert!(line.ends_with(&format!(
            "\"2026-09-09T00:00:00+00:00\",\"{SOURCE_REVISION}\""
        )));
    }

    #[test]
    fn object_line_missing_key_is_empty() {
        let record = IrrRecord {
            object_type: "x".to_string(),
            attributes: vec![],
        };
        let line = object_line("RIPE", &record, "2026-09-09T00:00:00+00:00");
        assert!(line.starts_with("\"RIPE\",\"x\",\"\",\"{"));
    }
}
