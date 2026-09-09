//! Generic snapshot-refresh runner shared by all pg_inserter tasks: advisory
//! lock, staging COPY, empty-dataset guard, atomic swap, and
//! `meta.ingest_run` provenance.

use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_util::SinkExt;
use std::pin::Pin;
use tokio_postgres::{Client, CopyInSink, NoTls};
use tracing::{error, info, warn};

pub(crate) const SOURCE_REVISION: &str = env!("CARGO_PKG_VERSION");

/// COPY payload chunk size (bytes) flushed to PostgreSQL at a time.
const COPY_CHUNK_BYTES: usize = 512 * 1024;

const INGEST_RUN_DDL: &str = "CREATE TABLE IF NOT EXISTS meta.ingest_run (
    run_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    task text NOT NULL,
    status text NOT NULL,
    row_count bigint,
    data_as_of timestamptz,
    source_revision text NOT NULL,
    started_at timestamptz NOT NULL,
    finished_at timestamptz NOT NULL,
    duration_secs double precision NOT NULL,
    error text
)";

const INGEST_INSERT_SQL: &str = "INSERT INTO meta.ingest_run (task, status, row_count, data_as_of, source_revision, started_at, finished_at, duration_secs, error) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";

/// Static description of one refreshable table.
#[derive(Debug, Clone)]
pub(crate) struct TableSpec {
    /// Provenance task name, e.g. `asndata.asnames`.
    pub(crate) task: &'static str,
    /// Fully qualified live table, e.g. `asndata.asnames`.
    pub(crate) table: &'static str,
    /// DDL creating the staging table (same columns as the live table).
    pub(crate) staging_ddl: &'static str,
    /// Comma-separated COPY column list.
    pub(crate) copy_columns: &'static str,
    /// Statements building integrity indexes on the staging table (hard errors).
    pub(crate) index_ddl: &'static [&'static str],
    /// `(staging_name, live_name)` index renames applied after the table swap.
    pub(crate) index_swaps: &'static [(&'static str, &'static str)],
}

/// Produce the CSV lines for one snapshot. Runs on the blocking pool.
pub(crate) type LoadFn = fn(&str) -> Result<Vec<String>, String>;

/// Run one snapshot-refresh task to completion, recording provenance.
pub(crate) async fn run_task(spec: TableSpec, load: LoadFn, database_url: &str) -> Result<(), i32> {
    let started_at = Utc::now();
    let data_as_of = started_at;
    let data_as_of_str = data_as_of.to_rfc3339();
    // The commons data load is synchronous and internally creates/drops
    // blocking HTTP clients that own their own tokio runtimes; dropping such
    // a runtime inside an async context panics (tokio >= 1.48), so it must
    // run on the blocking pool instead.
    let lines = match tokio::task::spawn_blocking(move || load(&data_as_of_str)).await {
        Ok(Ok(lines)) => lines,
        Ok(Err(e)) => {
            let message = format!("data load failed: {e}");
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            return Err(11);
        }
        Err(e) => {
            let message = format!("data loader task failed: {e}");
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            return Err(11);
        }
    };

    let mut client = match connect(database_url).await {
        Ok(client) => client,
        Err((code, message)) => {
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            return Err(code);
        }
    };
    match write_table(&mut client, &spec, &lines, started_at, data_as_of).await {
        Ok(row_count) => {
            info!("{}: complete ({row_count} rows)", spec.task);
            Ok(())
        }
        Err((code, message)) => {
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            Err(code)
        }
    }
}

/// Run several already-loaded table snapshots through one connection. A
/// failing table records its own error provenance row and does not stop the
/// remaining tables (per-source independence).
pub(crate) async fn run_tables(
    tables: Vec<(TableSpec, Vec<String>)>,
    database_url: &str,
) -> Result<(), i32> {
    let started_at = Utc::now();
    let data_as_of = started_at;
    let mut client = match connect(database_url).await {
        Ok(client) => client,
        Err((code, message)) => {
            error!("failed to connect: {message}");
            return Err(code);
        }
    };
    let mut exit_code = 0;
    for (spec, lines) in &tables {
        match write_table(&mut client, spec, lines, started_at, data_as_of).await {
            Ok(row_count) => {
                info!("{}: complete ({row_count} rows)", spec.task);
            }
            Err((code, message)) => {
                error!("{}: {message}", spec.task);
                record_error_run(spec, database_url, started_at, &message).await;
                exit_code = code;
            }
        }
    }
    if exit_code != 0 {
        Err(exit_code)
    } else {
        Ok(())
    }
}

async fn connect(database_url: &str) -> Result<Client, (i32, String)> {
    info!("connecting to PostgreSQL ...");
    let (client, connection) = match tokio_postgres::connect(database_url, NoTls).await {
        Ok(pair) => pair,
        Err(e) => return Err((14, format!("failed to connect to PostgreSQL: {e}"))),
    };
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("postgres connection error: {e}");
        }
    });
    Ok(client)
}

/// Write one table snapshot: advisory lock, staging COPY, empty-dataset
/// guard, atomic swap, and the ingest_run provenance row.
async fn write_table(
    client: &mut Client,
    spec: &TableSpec,
    lines: &[String],
    started_at: DateTime<Utc>,
    data_as_of: DateTime<Utc>,
) -> Result<u64, (i32, String)> {
    let mut sink = prepare_copy(client, spec).await?;
    let mut buf: Vec<u8> = Vec::with_capacity(COPY_CHUNK_BYTES);
    for line in lines {
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        if buf.len() >= COPY_CHUNK_BYTES {
            sink.as_mut()
                .feed(Bytes::from(std::mem::take(&mut buf)))
                .await
                .map_err(|e| (15, format!("failed to stream COPY data: {e}")))?;
        }
    }
    if !buf.is_empty() {
        sink.as_mut()
            .feed(Bytes::from(buf))
            .await
            .map_err(|e| (15, format!("failed to stream COPY data: {e}")))?;
    }
    finish_table(client, spec, sink, started_at, data_as_of).await
}

/// Streaming variant of `run_task` for datasets too large to materialize in
/// memory: the producer runs on the blocking pool and pushes CSV lines
/// through a bounded channel; the COPY sink consumes them incrementally.
pub(crate) async fn run_streaming_task(
    spec: TableSpec,
    produce: impl FnOnce(tokio::sync::mpsc::Sender<Result<String, String>>) -> Result<(), String>
    + Send
    + 'static,
    database_url: &str,
) -> Result<(), i32> {
    let started_at = Utc::now();
    let data_as_of = started_at;
    let mut client = match connect(database_url).await {
        Ok(client) => client,
        Err((code, message)) => {
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            return Err(code);
        }
    };
    match write_table_streaming(&mut client, &spec, produce, started_at, data_as_of).await {
        Ok(row_count) => {
            info!("{}: complete ({row_count} rows)", spec.task);
            Ok(())
        }
        Err((code, message)) => {
            error!("{}: {message}", spec.task);
            record_error_run(&spec, database_url, started_at, &message).await;
            Err(code)
        }
    }
}

async fn write_table_streaming(
    client: &mut Client,
    spec: &TableSpec,
    produce: impl FnOnce(tokio::sync::mpsc::Sender<Result<String, String>>) -> Result<(), String>
    + Send
    + 'static,
    started_at: DateTime<Utc>,
    data_as_of: DateTime<Utc>,
) -> Result<u64, (i32, String)> {
    let mut sink = prepare_copy(client, spec).await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<String, String>>(64);
    let producer = tokio::task::spawn_blocking(move || produce(tx));

    let mut buf: Vec<u8> = Vec::with_capacity(COPY_CHUNK_BYTES);
    while let Some(item) = rx.recv().await {
        let line = item.map_err(|e| (16, format!("row producer failed: {e}")))?;
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        if buf.len() >= COPY_CHUNK_BYTES {
            sink.as_mut()
                .feed(Bytes::from(std::mem::take(&mut buf)))
                .await
                .map_err(|e| (15, format!("failed to stream COPY data: {e}")))?;
        }
    }
    // The channel closed: the producer finished. Surface its result before
    // committing anything.
    match producer.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err((11, format!("row producer failed: {e}"))),
        Err(e) => return Err((11, format!("row producer task failed: {e}"))),
    }
    if !buf.is_empty() {
        sink.as_mut()
            .feed(Bytes::from(buf))
            .await
            .map_err(|e| (15, format!("failed to stream COPY data: {e}")))?;
    }
    finish_table(client, spec, sink, started_at, data_as_of).await
}

/// Acquire the per-table advisory lock, ensure schemas, and start the staging
/// COPY. Returns the open sink for the caller to feed.
async fn prepare_copy(
    client: &mut Client,
    spec: &TableSpec,
) -> Result<Pin<Box<CopyInSink<Bytes>>>, (i32, String)> {
    // Serialize concurrent runs of the same task before any DDL/COPY work.
    client
        .execute("SELECT pg_advisory_lock($1)", &[&lock_key(spec.table)])
        .await
        .map_err(|e| (15, format!("failed to acquire advisory lock: {e}")))?;

    ensure_schema(client, "meta").await?;
    let schema = spec.table.split('.').next().unwrap_or_default();
    ensure_schema(client, schema).await?;
    client
        .batch_execute(INGEST_RUN_DDL)
        .await
        .map_err(|e| (15, format!("failed to create meta.ingest_run: {e}")))?;
    client
        .batch_execute(&format!("DROP TABLE IF EXISTS {}_staging", spec.table))
        .await
        .map_err(|e| (15, format!("failed to drop staging table: {e}")))?;
    client
        .batch_execute(spec.staging_ddl)
        .await
        .map_err(|e| (15, format!("failed to create staging table: {e}")))?;

    let copy_sql = format!(
        "COPY {}_staging ({}) FROM STDIN WITH (FORMAT csv)",
        spec.table, spec.copy_columns
    );
    let sink = client
        .copy_in(&copy_sql)
        .await
        .map_err(|e| (15, format!("failed to start COPY: {e}")))?;
    Ok(Box::pin(sink))
}

/// Finish a staging COPY and publish the table: empty-dataset guard, staging
/// indexes, single-transaction atomic swap, and the ingest_run provenance row.
async fn finish_table(
    client: &mut Client,
    spec: &TableSpec,
    mut sink: Pin<Box<CopyInSink<Bytes>>>,
    started_at: DateTime<Utc>,
    data_as_of: DateTime<Utc>,
) -> Result<u64, (i32, String)> {
    let copied_rows = sink
        .as_mut()
        .finish()
        .await
        .map_err(|e| (15, format!("failed to finish COPY: {e}")))?;
    info!(
        "{}: COPY complete ({copied_rows} rows in staging)",
        spec.task
    );

    // Guard against replacing a healthy table with a broken snapshot.
    let existing_rows = if table_exists(client, spec.table).await? {
        let n: i64 = client
            .query_one(&format!("SELECT count(*) FROM {}", spec.table), &[])
            .await
            .map_err(|e| (15, format!("failed to count {} rows: {e}", spec.table)))?
            .get(0);
        Some(n)
    } else {
        None
    };
    check_swap_safety(existing_rows, copied_rows)
        .map_err(|message| (15, format!("{message} (staging table left for inspection)")))?;

    for stmt in spec.index_ddl {
        client
            .batch_execute(stmt)
            .await
            .map_err(|e| (15, format!("failed to build staging index: {e}")))?;
    }

    // Atomic swap in a single transaction: readers always see either the old
    // or the new table, never an empty one. The ingest_run row commits with
    // the swap.
    let tx = client
        .transaction()
        .await
        .map_err(|e| (15, format!("failed to begin swap transaction: {e}")))?;
    tx.batch_execute(&format!("DROP TABLE IF EXISTS {}", spec.table))
        .await
        .map_err(|e| (15, format!("failed to drop {}: {e}", spec.table)))?;
    let live_name = spec.table.rsplit('.').next().unwrap_or_default();
    tx.batch_execute(&format!(
        "ALTER TABLE {}_staging RENAME TO {}",
        spec.table, live_name
    ))
    .await
    .map_err(|e| {
        (
            15,
            format!("failed to rename staging to {}: {e}", spec.table),
        )
    })?;
    for (old_name, new_name) in spec.index_swaps {
        tx.batch_execute(&format!("ALTER INDEX {} RENAME TO {}", old_name, new_name))
            .await
            .map_err(|e| (15, format!("failed to rename index {old_name}: {e}")))?;
    }
    let finished_at = Utc::now();
    let duration_secs = (finished_at - started_at).num_milliseconds() as f64 / 1000.0;
    let row_count = copied_rows.min(i64::MAX as u64) as i64;
    tx.execute(
        INGEST_INSERT_SQL,
        &[
            &spec.task,
            &"ok",
            &row_count,
            &data_as_of,
            &SOURCE_REVISION,
            &started_at,
            &finished_at,
            &duration_secs,
            &None::<&str>,
        ],
    )
    .await
    .map_err(|e| (15, format!("failed to record ingest_run: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| (15, format!("failed to commit swap: {e}")))?;
    Ok(copied_rows)
}

/// Best-effort provenance record for a failed run. Logs warnings only; it
/// never changes the caller's exit code. Uses a fresh connection so the
/// failure of the original client does not matter.
async fn record_error_run(
    spec: &TableSpec,
    database_url: &str,
    started_at: DateTime<Utc>,
    message: &str,
) {
    let (client, connection) = match tokio_postgres::connect(database_url, NoTls).await {
        Ok(pair) => pair,
        Err(e) => {
            warn!(
                "{}: could not connect to record ingest_run error: {e}",
                spec.task
            );
            return;
        }
    };
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("postgres connection error: {e}");
        }
    });
    // Best-effort DDL: when the failure happened before the first run's DDL
    // (e.g. the data load failed), the provenance table may not exist yet.
    let _ = client
        .batch_execute("CREATE SCHEMA IF NOT EXISTS meta")
        .await;
    let _ = client.batch_execute(INGEST_RUN_DDL).await;
    let finished_at = Utc::now();
    let duration_secs = (finished_at - started_at).num_milliseconds() as f64 / 1000.0;
    let record = client
        .execute(
            INGEST_INSERT_SQL,
            &[
                &spec.task,
                &"error",
                &None::<i64>,           // row count unknown for a failed run
                &None::<DateTime<Utc>>, // no data snapshot completed
                &SOURCE_REVISION,
                &started_at,
                &finished_at,
                &duration_secs,
                &Some(message),
            ],
        )
        .await;
    if let Err(e) = record {
        warn!("{}: failed to record ingest_run error: {e}", spec.task);
    }
}

/// Ensure a schema exists. A separate probe avoids relying on the privilege
/// semantics of `CREATE SCHEMA IF NOT EXISTS` for a pre-created schema.
async fn ensure_schema(client: &Client, schema: &str) -> Result<(), (i32, String)> {
    let exists: bool = client
        .query_one("SELECT to_regnamespace($1) IS NOT NULL", &[&schema])
        .await
        .map_err(|e| (15, format!("failed to check for schema {schema}: {e}")))?
        .get(0);
    if !exists {
        client
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .await
            .map_err(|e| (15, format!("failed to create schema {schema}: {e}")))?;
    }
    Ok(())
}

async fn table_exists(client: &Client, name: &str) -> Result<bool, (i32, String)> {
    let exists: bool = client
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&name])
        .await
        .map_err(|e| (15, format!("failed to check for table {name}: {e}")))?
        .get(0);
    Ok(exists)
}

/// Refuse to replace a healthy table with a broken snapshot: a new dataset
/// with zero rows, or with fewer than half the currently loaded rows, is
/// treated as a source failure rather than a legitimate refresh.
fn check_swap_safety(existing_rows: Option<i64>, new_rows: u64) -> Result<(), String> {
    match existing_rows {
        Some(existing) if existing > 0 && new_rows < existing as u64 / 2 => Err(format!(
            "refusing to swap: new dataset has {new_rows} rows vs {existing} currently loaded (less than half)"
        )),
        None if new_rows == 0 => Err("refusing to swap: loaded dataset is empty".to_string()),
        _ => Ok(()),
    }
}

/// FNV-1a 64-bit hash of the table name, used as the per-table session-level
/// advisory lock key.
fn lock_key(name: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash as i64
}

/// CSV-encode the optional field list: every `Some` value is double-quoted
/// (embedded quotes doubled); every `None` value becomes an empty unquoted
/// field, i.e. SQL NULL for PostgreSQL COPY.
pub(crate) fn build_csv_line(fields: &[Option<String>]) -> String {
    let mut line = String::new();
    for (idx, field) in fields.iter().enumerate() {
        if idx > 0 {
            line.push(',');
        }
        if let Some(value) = field {
            push_csv_field(&mut line, value);
        }
    }
    line
}

/// Append `value` to `out` as a double-quoted CSV field, doubling embedded
/// double quotes so commas and newlines inside the value survive. NUL bytes
/// are dropped: they are not representable in PostgreSQL's text format.
pub(crate) fn push_csv_field(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\"\""),
            '\0' => {}
            _ => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_quotes_doubled() {
        let mut out = String::new();
        push_csv_field(&mut out, "a\"b");
        assert_eq!(out, "\"a\"\"b\"");
    }

    #[test]
    fn csv_field_commas_newlines_and_nul() {
        let mut out = String::new();
        push_csv_field(&mut out, "a,b\nc\0");
        assert_eq!(out, "\"a,b\nc\"");
    }

    #[test]
    fn csv_line_null_as_empty_field() {
        let line = build_csv_line(&[
            Some("a,b".to_string()),
            Some(String::new()),
            None,
            Some("x\"y".to_string()),
        ]);
        assert_eq!(line, "\"a,b\",\"\",,\"x\"\"y\"");
    }

    #[test]
    fn swap_safety_rejects_empty_first_load() {
        assert!(check_swap_safety(None, 0).is_err());
        assert!(check_swap_safety(None, 1).is_ok());
    }

    #[test]
    fn swap_safety_rejects_less_than_half_of_loaded_rows() {
        assert!(check_swap_safety(Some(100), 49).is_err());
        assert!(check_swap_safety(Some(100), 50).is_ok());
        assert!(check_swap_safety(Some(100), 101).is_ok());
    }

    #[test]
    fn lock_key_is_deterministic_and_distinct() {
        assert_eq!(lock_key("asndata.asnames"), lock_key("asndata.asnames"));
        assert_ne!(lock_key("asndata.asnames"), lock_key("asndata.as2org"));
    }
}
