//! Parquet export of all bgpkit-commons data sources.
//!
//! Each function writes one Parquet file from a loaded data source. The output
//! schema is source-faithful: one file per upstream source, full source fields
//! preserved, no cross-source merging.
//!
//! All writers use Arrow RecordBatch -> Parquet with Zstandard compression.
//! Callers pass a directory path; each writer writes `<dir>/<name>.parquet`.
//!
//! # Feature flag
//!
//! This entire module is behind the `export` feature, which pulls in `arrow`
//! and `parquet` as dependencies.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanArray, Date32Array, ListArray, RecordBatch, StringArray, UInt8Array,
    UInt32Array,
};
use arrow::buffer::{NullBuffer, OffsetBuffer};
use arrow::datatypes::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;

use crate::BgpkitCommons;

type WriteResult = Result<(), ExportError>;

/// Error type for export operations.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    #[error("Module not loaded: {0}")]
    ModuleNotLoaded(String),
}

const ZSTD_LEVEL: i32 = 3;

// ===========================================================================
// Core write helper
// ===========================================================================

fn write_parquet(path: impl AsRef<Path>, batch: RecordBatch) -> Result<(), ExportError> {
    let file = File::create(path.as_ref())?;
    let props = parquet::file::properties::WriterProperties::builder()
        .set_compression(parquet::basic::Compression::ZSTD(
            parquet::basic::ZstdLevel::try_new(ZSTD_LEVEL).unwrap_or_default(),
        ))
        .build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

// ===========================================================================
// Array builders
// ===========================================================================

fn string_array(data: Vec<Option<String>>) -> ArrayRef {
    Arc::new(StringArray::from(data))
}

fn string_array_nn(data: Vec<String>) -> ArrayRef {
    Arc::new(StringArray::from(data))
}

fn u32_array(data: Vec<Option<u32>>) -> ArrayRef {
    Arc::new(UInt32Array::from(data))
}

fn u32_array_nn(data: Vec<u32>) -> ArrayRef {
    Arc::new(UInt32Array::from(data))
}

fn u8_array_nn(data: Vec<u8>) -> ArrayRef {
    Arc::new(UInt8Array::from(data))
}

fn bool_array(data: Vec<Option<bool>>) -> ArrayRef {
    Arc::new(BooleanArray::from(data))
}

fn date32_array(data: Vec<Option<i32>>) -> ArrayRef {
    Arc::new(Date32Array::from(data))
}

fn list_string_field(name: &str, nullable: bool) -> Field {
    Field::new(
        name,
        DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
        nullable,
    )
}

fn build_string_list(data: &[Vec<String>]) -> ArrayRef {
    let mut values: Vec<Option<String>> = Vec::new();
    let mut offsets: Vec<i32> = vec![0];
    for list in data {
        for s in list {
            values.push(Some(s.clone()));
        }
        offsets.push(values.len() as i32);
    }
    let field = Arc::new(Field::new("item", DataType::Utf8, true));
    let offsets = OffsetBuffer::new(offsets.into());
    let values_arr: ArrayRef = Arc::new(StringArray::from(values));
    Arc::new(ListArray::new(field, offsets, values_arr, None))
}

fn build_nullable_string_list(data: &[Option<Vec<String>>]) -> ArrayRef {
    let mut values: Vec<Option<String>> = Vec::new();
    let mut offsets: Vec<i32> = vec![0];
    let mut nulls: Vec<bool> = Vec::new();
    for list in data {
        match list {
            None => {
                nulls.push(false);
                offsets.push(*offsets.last().unwrap());
            }
            Some(list) => {
                nulls.push(true);
                for s in list {
                    values.push(Some(s.clone()));
                }
                offsets.push(values.len() as i32);
            }
        }
    }
    let field = Arc::new(Field::new("item", DataType::Utf8, true));
    let offsets = OffsetBuffer::new(offsets.into());
    let values_arr: ArrayRef = Arc::new(StringArray::from(values));
    let null_buf = NullBuffer::from(nulls);
    Arc::new(ListArray::new(field, offsets, values_arr, Some(null_buf)))
}

fn date_to_date32(date: chrono::NaiveDate) -> Option<i32> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)?;
    Some(date.signed_duration_since(epoch).num_days() as i32)
}

// ===========================================================================
// Per-source writers
// ===========================================================================

/// Export country data to `<dir>/countries.parquet`.
pub fn countries(dir: impl AsRef<Path>, commons: &BgpkitCommons) -> WriteResult {
    #[cfg(feature = "countries")]
    {
        let countries = commons
            .country_all()
            .map_err(|_| ExportError::ModuleNotLoaded("countries".into()))?;

        let schema = Arc::new(Schema::new(vec![
            Field::new("code", DataType::Utf8, false),
            Field::new("code3", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("capital", DataType::Utf8, false),
            Field::new("continent", DataType::Utf8, false),
            Field::new("ltd", DataType::Utf8, true),
            list_string_field("neighbors", false),
        ]));

        let mut codes = Vec::new();
        let mut code3s = Vec::new();
        let mut names = Vec::new();
        let mut capitals = Vec::new();
        let mut continents = Vec::new();
        let mut tlds = Vec::new();
        let mut neighbors_list = Vec::new();

        for c in countries {
            codes.push(c.code);
            code3s.push(c.code3);
            names.push(c.name);
            capitals.push(c.capital);
            continents.push(c.continent);
            tlds.push(c.ltd);
            neighbors_list.push(c.neighbors);
        }

        let neighbors_arr = build_string_list(&neighbors_list);

        let batch = RecordBatch::try_new(
            schema,
            vec![
                string_array_nn(codes),
                string_array_nn(code3s),
                string_array_nn(names),
                string_array_nn(capitals),
                string_array_nn(continents),
                string_array(tlds),
                neighbors_arr,
            ],
        )?;

        write_parquet(dir.as_ref().join("countries.parquet"), batch)?;
        Ok(())
    }
    #[cfg(not(feature = "countries"))]
    {
        let _ = (dir, commons);
        Err(ExportError::ModuleNotLoaded("countries".into()))
    }
}

/// Export MRT collector metadata to `<dir>/mrt_collectors.parquet`.
pub fn mrt_collectors(dir: impl AsRef<Path>, commons: &BgpkitCommons) -> WriteResult {
    #[cfg(feature = "mrt_collectors")]
    {
        let collectors = commons
            .mrt_collectors_all()
            .map_err(|_| ExportError::ModuleNotLoaded("mrt_collectors".into()))?;

        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("project", DataType::Utf8, false),
            Field::new("data_url", DataType::Utf8, false),
            Field::new("activated_on", DataType::Utf8, false),
            Field::new("deactivated_on", DataType::Utf8, true),
            Field::new("country", DataType::Utf8, false),
        ]));

        let mut names = Vec::new();
        let mut projects = Vec::new();
        let mut urls = Vec::new();
        let mut activated = Vec::new();
        let mut deactivated = Vec::new();
        let mut countries = Vec::new();

        for c in collectors {
            names.push(c.name);
            projects.push(c.project.to_string());
            urls.push(c.data_url);
            activated.push(c.activated_on.to_string());
            deactivated.push(c.deactivated_on.map(|d| d.to_string()));
            countries.push(c.country);
        }

        let batch = RecordBatch::try_new(
            schema,
            vec![
                string_array_nn(names),
                string_array_nn(projects),
                string_array_nn(urls),
                string_array_nn(activated),
                string_array(deactivated),
                string_array_nn(countries),
            ],
        )?;

        write_parquet(dir.as_ref().join("mrt_collectors.parquet"), batch)?;
        Ok(())
    }
    #[cfg(not(feature = "mrt_collectors"))]
    {
        let _ = (dir, commons);
        Err(ExportError::ModuleNotLoaded("mrt_collectors".into()))
    }
}

/// Export IANA bogon (special-purpose) ASN + prefix registries to
/// `<dir>/iana_bogons.parquet`.
pub fn iana_bogons(dir: impl AsRef<Path>, commons: &BgpkitCommons) -> WriteResult {
    #[cfg(feature = "bogons")]
    {
        let prefixes = commons
            .get_bogon_prefixes()
            .map_err(|_| ExportError::ModuleNotLoaded("bogons".into()))?;
        let asns = commons
            .get_bogon_asns()
            .map_err(|_| ExportError::ModuleNotLoaded("bogons".into()))?;

        let schema = Arc::new(Schema::new(vec![
            Field::new("record_kind", DataType::Utf8, false),
            Field::new("asn_start", DataType::UInt32, true),
            Field::new("asn_end", DataType::UInt32, true),
            Field::new("prefix", DataType::Utf8, true),
            Field::new("description", DataType::Utf8, false),
            list_string_field("rfc_urls", true),
            Field::new("allocation_date", DataType::Date32, true),
            Field::new("termination_date", DataType::Date32, true),
            Field::new("is_source", DataType::Boolean, true),
            Field::new("is_destination", DataType::Boolean, true),
            Field::new("is_forwardable", DataType::Boolean, true),
            Field::new("is_global", DataType::Boolean, true),
            Field::new("is_reserved", DataType::Boolean, true),
        ]));

        let mut kinds = Vec::new();
        let mut asn_starts = Vec::new();
        let mut asn_ends = Vec::new();
        let mut prefixes_col = Vec::new();
        let mut descriptions = Vec::new();
        let mut rfc_urls_list = Vec::new();
        let mut alloc_dates = Vec::new();
        let mut term_dates = Vec::new();
        let mut is_source = Vec::new();
        let mut is_dest = Vec::new();
        let mut is_forward = Vec::new();
        let mut is_global = Vec::new();
        let mut is_reserved = Vec::new();

        // ASN bogons
        for a in &asns {
            kinds.push("asn".to_string());
            asn_starts.push(Some(a.asn_range.0));
            asn_ends.push(Some(a.asn_range.1));
            prefixes_col.push(None);
            descriptions.push(a.description.clone());
            rfc_urls_list.push(Some(a.rfc_urls.clone()));
            alloc_dates.push(None);
            term_dates.push(None);
            is_source.push(None);
            is_dest.push(None);
            is_forward.push(None);
            is_global.push(None);
            is_reserved.push(None);
        }

        // Prefix bogons
        for p in &prefixes {
            let kind = match p.prefix {
                ipnet::IpNet::V4(_) => "prefix_v4",
                ipnet::IpNet::V6(_) => "prefix_v6",
            };
            kinds.push(kind.to_string());
            asn_starts.push(None);
            asn_ends.push(None);
            prefixes_col.push(Some(p.prefix.to_string()));
            descriptions.push(p.description.clone());
            rfc_urls_list.push(Some(p.rfc_urls.clone()));
            alloc_dates.push(date_to_date32(p.allocation_date));
            term_dates.push(p.termination_date.and_then(date_to_date32));
            is_source.push(Some(p.source));
            is_dest.push(Some(p.destination));
            is_forward.push(Some(p.forwardable));
            is_global.push(Some(p.global));
            is_reserved.push(Some(p.reserved));
        }

        let rfc_arr = build_nullable_string_list(&rfc_urls_list);

        let batch = RecordBatch::try_new(
            schema,
            vec![
                string_array_nn(kinds),
                u32_array(asn_starts),
                u32_array(asn_ends),
                string_array(prefixes_col),
                string_array_nn(descriptions),
                rfc_arr,
                date32_array(alloc_dates),
                date32_array(term_dates),
                bool_array(is_source),
                bool_array(is_dest),
                bool_array(is_forward),
                bool_array(is_global),
                bool_array(is_reserved),
            ],
        )?;

        write_parquet(dir.as_ref().join("iana_bogons.parquet"), batch)?;
        Ok(())
    }
    #[cfg(not(feature = "bogons"))]
    {
        let _ = (dir, commons);
        Err(ExportError::ModuleNotLoaded("bogons".into()))
    }
}

/// Export AS relationship data to `<dir>/as_relationships.parquet`.
pub fn as_relationships(dir: impl AsRef<Path>, commons: &BgpkitCommons) -> WriteResult {
    let schema = Arc::new(Schema::new(vec![
        Field::new("asn1", DataType::UInt32, false),
        Field::new("asn2", DataType::UInt32, false),
        Field::new("rel", DataType::Utf8, false),
        Field::new("paths_count", DataType::UInt32, false),
        Field::new("peers_count", DataType::UInt32, false),
        Field::new("address_family", DataType::UInt8, false),
    ]));

    let mut asn1s = Vec::new();
    let mut asn2s = Vec::new();
    let mut rels = Vec::new();
    let mut paths = Vec::new();
    let mut peers = Vec::new();
    let mut afs = Vec::new();

    for entry in commons
        .as2rel_all_entries()
        .map_err(|e| ExportError::ModuleNotLoaded(e.to_string()))?
    {
        asn1s.push(entry.asn1);
        asn2s.push(entry.asn2);
        rels.push(
            match entry.rel {
                crate::as2rel::AsRelationship::ProviderCustomer => "pc",
                crate::as2rel::AsRelationship::PeerPeer => "pp",
                crate::as2rel::AsRelationship::CustomerProvider => "cp",
            }
            .to_string(),
        );
        paths.push(entry.paths_count);
        peers.push(entry.peers_count);
        afs.push(entry.address_family);
    }

    let batch = RecordBatch::try_new(
        schema,
        vec![
            u32_array_nn(asn1s),
            u32_array_nn(asn2s),
            string_array_nn(rels),
            u32_array_nn(paths),
            u32_array_nn(peers),
            u8_array_nn(afs),
        ],
    )?;

    write_parquet(dir.as_ref().join("as_relationships.parquet"), batch)?;
    Ok(())
}

/// Export AS names (RIPE asn.txt core) to `<dir>/asn_names.parquet`.
pub fn asn_names(dir: impl AsRef<Path>, commons: &BgpkitCommons) -> WriteResult {
    let all = commons
        .asinfo_all()
        .map_err(|e| ExportError::ModuleNotLoaded(e.to_string()))?;

    let schema = Arc::new(Schema::new(vec![
        Field::new("asn", DataType::UInt32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("country", DataType::Utf8, false),
    ]));

    let mut asns = Vec::new();
    let mut names = Vec::new();
    let mut countries = Vec::new();

    let mut sorted: Vec<_> = all.values().collect();
    sorted.sort_by_key(|a| a.asn);

    for info in sorted {
        asns.push(info.asn);
        names.push(info.name.clone());
        countries.push(info.country.clone());
    }

    let batch = RecordBatch::try_new(
        schema,
        vec![
            u32_array_nn(asns),
            string_array_nn(names),
            string_array_nn(countries),
        ],
    )?;

    write_parquet(dir.as_ref().join("asn_names.parquet"), batch)?;
    Ok(())
}

/// Export RIR delegated statistics to `<dir>/rir_delegated.parquet`.
pub fn rir_delegated(dir: impl AsRef<Path>) -> WriteResult {
    use crate::delegated;

    let schema = Arc::new(Schema::new(vec![
        Field::new("registry", DataType::Utf8, false),
        Field::new("country", DataType::Utf8, false),
        Field::new("record_type", DataType::Utf8, false),
        Field::new("start", DataType::Utf8, false),
        Field::new("value", DataType::Utf8, false),
        Field::new("date", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        list_string_field("extensions", true),
    ]));

    let mut registries = Vec::new();
    let mut countries = Vec::new();
    let mut types = Vec::new();
    let mut starts = Vec::new();
    let mut values = Vec::new();
    let mut dates = Vec::new();
    let mut statuses = Vec::new();
    let mut exts_list = Vec::new();

    for url in delegated::RIR_DELEGATED_STATS_URLS {
        match delegated::fetch(url) {
            Ok(reader) => {
                for record in delegated::parse_reader(reader) {
                    match record {
                        Ok(r) => {
                            registries.push(r.registry);
                            countries.push(r.country);
                            types.push(r.record_type);
                            starts.push(r.start);
                            values.push(r.value);
                            dates.push(r.date);
                            statuses.push(r.status);
                            exts_list.push(if r.extensions.is_empty() {
                                None
                            } else {
                                Some(r.extensions)
                            });
                        }
                        Err(e) => {
                            tracing::warn!("failed to parse delegated stats record: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("failed to fetch delegated stats from {url}: {e}");
            }
        }
    }

    let exts_arr = build_nullable_string_list(&exts_list);

    if registries.is_empty() {
        return Err(ExportError::ModuleNotLoaded(
            "rir_delegated: no records fetched (all upstream sources failed)".into(),
        ));
    }

    let batch = RecordBatch::try_new(
        schema,
        vec![
            string_array_nn(registries),
            string_array_nn(countries),
            string_array_nn(types),
            string_array_nn(starts),
            string_array_nn(values),
            string_array_nn(dates),
            string_array_nn(statuses),
            exts_arr,
        ],
    )?;

    write_parquet(dir.as_ref().join("rir_delegated.parquet"), batch)?;
    Ok(())
}
