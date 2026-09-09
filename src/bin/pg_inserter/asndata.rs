//! `asndata` schema: per-source AS reference-data tasks.
//!
//! Each task loads one source independently and produces CSV lines for the
//! generic snapshot-refresh runner. A failing source only fails its own task.

use super::refresh::{TableSpec, build_csv_line, push_csv_field};
use bgpkit_commons::asinfo::{AsInfo, AsInfoBuilder};
use bgpkit_commons::delegated::{self, DelegatedRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Task {
    Asnames,
    As2org,
    Population,
    Hegemony,
    Delegated,
}

pub(crate) const ALL_TASKS: &[Task] = &[
    Task::Asnames,
    Task::As2org,
    Task::Population,
    Task::Hegemony,
    Task::Delegated,
];

impl Task {
    pub(crate) const fn spec(self) -> TableSpec {
        match self {
            Task::Asnames => TableSpec {
                task: "asndata.asnames",
                table: "asndata.asnames",
                staging_ddl: "CREATE TABLE asndata.asnames_staging (
                    asn bigint NOT NULL,
                    name text NOT NULL,
                    country text NOT NULL,
                    data_as_of timestamptz NOT NULL,
                    source_revision text NOT NULL
                )",
                copy_columns: "asn, name, country, data_as_of, source_revision",
                index_ddl: &[
                    "ALTER TABLE asndata.asnames_staging ADD CONSTRAINT asnames_staging_pkey PRIMARY KEY (asn)",
                ],
                index_swaps: &[("asndata.asnames_staging_pkey", "asnames_pkey")],
            },
            Task::As2org => TableSpec {
                task: "asndata.as2org",
                table: "asndata.as2org",
                staging_ddl: "CREATE TABLE asndata.as2org_staging (
                    asn bigint NOT NULL,
                    name text NOT NULL,
                    country text NOT NULL,
                    org_id text NOT NULL,
                    org_name text NOT NULL,
                    data_as_of timestamptz NOT NULL,
                    source_revision text NOT NULL
                )",
                copy_columns: "asn, name, country, org_id, org_name, data_as_of, source_revision",
                index_ddl: &[
                    "ALTER TABLE asndata.as2org_staging ADD CONSTRAINT as2org_staging_pkey PRIMARY KEY (asn)",
                    "CREATE INDEX as2org_staging_org_name_idx ON asndata.as2org_staging (org_name)",
                ],
                index_swaps: &[
                    ("asndata.as2org_staging_pkey", "as2org_pkey"),
                    ("asndata.as2org_staging_org_name_idx", "as2org_org_name_idx"),
                ],
            },
            Task::Population => TableSpec {
                task: "asndata.population",
                table: "asndata.population",
                staging_ddl: "CREATE TABLE asndata.population_staging (
                    asn bigint NOT NULL,
                    user_count bigint NOT NULL,
                    percent_country double precision NOT NULL,
                    percent_global double precision NOT NULL,
                    sample_count bigint NOT NULL,
                    data_as_of timestamptz NOT NULL,
                    source_revision text NOT NULL
                )",
                copy_columns: "asn, user_count, percent_country, percent_global, sample_count, data_as_of, source_revision",
                index_ddl: &[
                    "ALTER TABLE asndata.population_staging ADD CONSTRAINT population_staging_pkey PRIMARY KEY (asn)",
                ],
                index_swaps: &[("asndata.population_staging_pkey", "population_pkey")],
            },
            Task::Hegemony => TableSpec {
                task: "asndata.hegemony",
                table: "asndata.hegemony",
                staging_ddl: "CREATE TABLE asndata.hegemony_staging (
                    asn bigint NOT NULL,
                    ipv4 double precision NOT NULL,
                    ipv6 double precision NOT NULL,
                    data_as_of timestamptz NOT NULL,
                    source_revision text NOT NULL
                )",
                copy_columns: "asn, ipv4, ipv6, data_as_of, source_revision",
                index_ddl: &[
                    "ALTER TABLE asndata.hegemony_staging ADD CONSTRAINT hegemony_staging_pkey PRIMARY KEY (asn)",
                ],
                index_swaps: &[("asndata.hegemony_staging_pkey", "hegemony_pkey")],
            },
            Task::Delegated => TableSpec {
                task: "asndata.delegated",
                table: "asndata.delegated",
                staging_ddl: "CREATE TABLE asndata.delegated_staging (
                    registry text NOT NULL,
                    country text NOT NULL,
                    record_type text NOT NULL,
                    start text NOT NULL,
                    value text NOT NULL,
                    date text NOT NULL,
                    status text NOT NULL,
                    extensions jsonb,
                    data_as_of timestamptz NOT NULL,
                    source_revision text NOT NULL
                )",
                copy_columns: "registry, country, record_type, start, value, date, status, extensions, data_as_of, source_revision",
                index_ddl: &[
                    "CREATE INDEX delegated_staging_type_registry_idx ON asndata.delegated_staging (record_type, registry)",
                ],
                index_swaps: &[(
                    "asndata.delegated_staging_type_registry_idx",
                    "delegated_type_registry_idx",
                )],
            },
        }
    }

    pub(crate) const fn load_fn(self) -> super::refresh::LoadFn {
        match self {
            Task::Asnames => load_asnames,
            Task::As2org => load_as2org,
            Task::Population => load_population,
            Task::Hegemony => load_hegemony,
            Task::Delegated => load_delegated,
        }
    }
}

/// Build an `AsInfo` map with the given source flags and return it
/// ASN-sorted. `asnames` is always loaded (it is the base dataset).
fn load_infos(builder: AsInfoBuilder) -> Result<Vec<AsInfo>, String> {
    let utils = builder
        .build()
        .map_err(|e| format!("failed to load asinfo data: {e}"))?;
    let mut infos: Vec<AsInfo> = utils.asinfo_map.into_values().collect();
    infos.sort_by_key(|i| i.asn);
    Ok(infos)
}

fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    push_csv_field(&mut out, value);
    out
}

fn load_asnames(data_as_of: &str) -> Result<Vec<String>, String> {
    let infos = load_infos(AsInfoBuilder::new())?;
    Ok(infos
        .iter()
        .map(|i| {
            let fields = vec![
                Some(i.name.clone()),
                Some(i.country.clone()),
                Some(data_as_of.to_string()),
                Some(super::refresh::SOURCE_REVISION.to_string()),
            ];
            format!("{},{}", i.asn, build_csv_line(&fields))
        })
        .collect())
}

fn load_as2org(data_as_of: &str) -> Result<Vec<String>, String> {
    let infos = load_infos(AsInfoBuilder::new().with_as2org())?;
    Ok(infos
        .iter()
        .filter_map(|i| {
            let a2o = i.as2org.as_ref()?;
            let fields = vec![
                Some(a2o.name.clone()),
                Some(a2o.country.clone()),
                Some(a2o.org_id.clone()),
                Some(a2o.org_name.clone()),
                Some(data_as_of.to_string()),
                Some(super::refresh::SOURCE_REVISION.to_string()),
            ];
            Some(format!("{},{}", i.asn, build_csv_line(&fields)))
        })
        .collect())
}

fn load_population(data_as_of: &str) -> Result<Vec<String>, String> {
    let infos = load_infos(AsInfoBuilder::new().with_population())?;
    Ok(infos
        .iter()
        .filter_map(|i| {
            let p = i.population.as_ref()?;
            Some(format!(
                "{},{},{},{},{},{},{}",
                i.asn,
                p.user_count,
                p.percent_country,
                p.percent_global,
                p.sample_count,
                quoted(data_as_of),
                quoted(super::refresh::SOURCE_REVISION)
            ))
        })
        .collect())
}

fn load_hegemony(data_as_of: &str) -> Result<Vec<String>, String> {
    let infos = load_infos(AsInfoBuilder::new().with_hegemony())?;
    Ok(infos
        .iter()
        .filter_map(|i| {
            let h = i.hegemony.as_ref()?;
            Some(format!(
                "{},{},{},{},{}",
                i.asn,
                h.ipv4,
                h.ipv6,
                quoted(data_as_of),
                quoted(super::refresh::SOURCE_REVISION)
            ))
        })
        .collect())
}

fn load_delegated(data_as_of: &str) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for url in delegated::RIR_DELEGATED_STATS_URLS {
        let reader = delegated::fetch(url).map_err(|e| format!("failed to fetch {url}: {e}"))?;
        for record in delegated::parse_reader(reader) {
            let record = record.map_err(|e| format!("failed to parse {url}: {e}"))?;
            lines.push(delegated_line(&record, data_as_of));
        }
    }
    // Deterministic row order via the natural column order of the line.
    lines.sort();
    Ok(lines)
}

fn delegated_line(record: &DelegatedRecord, data_as_of: &str) -> String {
    let fields = vec![
        Some(record.registry.clone()),
        Some(record.country.clone()),
        Some(record.record_type.clone()),
        Some(record.start.clone()),
        Some(record.value.clone()),
        Some(record.date.clone()),
        Some(record.status.clone()),
        Some(serde_json::to_string(&record.extensions).unwrap_or_default()),
        Some(data_as_of.to_string()),
        Some(super::refresh::SOURCE_REVISION.to_string()),
    ];
    build_csv_line(&fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegated_line_shape() {
        let record = DelegatedRecord {
            registry: "ripencc".to_string(),
            country: "NL".to_string(),
            record_type: "asn".to_string(),
            start: "1".to_string(),
            value: "1".to_string(),
            date: "20240101".to_string(),
            status: "allocated".to_string(),
            extensions: vec!["ext1".to_string()],
        };
        let expected = format!(
            "\"ripencc\",\"NL\",\"asn\",\"1\",\"1\",\"20240101\",\"allocated\",\"[\"\"ext1\"\"]\",\"2026-09-08T00:00:00+00:00\",\"{}\"",
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            delegated_line(&record, "2026-09-08T00:00:00+00:00"),
            expected
        );
    }
}
