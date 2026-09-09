//! `peeringdb` schema: full PeeringDB API mirror (12 tables).
//!
//! One `Peeringdb::new()` load (all 12 endpoints) is dumped into 12 snapshot
//! tables. Per-table shape: native PeeringDB id as PK, typed foreign-key and
//! search columns, the complete API object in a `record` JSONB column, and
//! `data_as_of`/`source_revision` provenance. Per-table independence inside
//! the PeeringDB family is deferred: the whole family refreshes atomically.

use super::refresh::{SOURCE_REVISION, TableSpec, build_csv_line};
use bgpkit_commons::peeringdb::{
    Campus, Carrier, CarrierFacility, Facility, InternetExchange, IxFacility, IxLan, IxPrefix,
    Network, NetworkFacility, NetworkIxLan, Organization, Peeringdb,
};
use serde::Serialize;
use std::collections::HashMap;

/// Load the full PeeringDB dataset and map it to `(TableSpec, lines)` pairs,
/// one per table. Runs on the blocking pool.
pub(crate) fn load_all(data_as_of: &str) -> Result<Vec<(TableSpec, Vec<String>)>, String> {
    let pdb = Peeringdb::new().map_err(|e| format!("failed to load PeeringDB data: {e}"))?;
    let tables = vec![
        (NET_SPEC, dump_networks(&pdb.networks, data_as_of)?),
        (
            IX_SPEC,
            dump_records(&pdb.internet_exchanges, data_as_of, ix_line)?,
        ),
        (
            IXLAN_SPEC,
            dump_records(&pdb.ixp_lans, data_as_of, ixlan_line)?,
        ),
        (
            IXPFX_SPEC,
            dump_vec(&pdb.ixp_prefixes, data_as_of, ixp_ixpfx_line)?,
        ),
        (
            NETIXLAN_SPEC,
            dump_vec(&pdb.network_ixp_membership, data_as_of, netixlan_line)?,
        ),
        (
            FAC_SPEC,
            dump_records(&pdb.facilities, data_as_of, fac_line)?,
        ),
        (
            NETFAC_SPEC,
            dump_vec(&pdb.network_facilities, data_as_of, netfac_line)?,
        ),
        (
            IXFAC_SPEC,
            dump_vec(&pdb.ixp_facilities, data_as_of, ixfac_line)?,
        ),
        (
            ORG_SPEC,
            dump_records(&pdb.organizations, data_as_of, org_line)?,
        ),
        (
            CAMPUS_SPEC,
            dump_records(&pdb.campuses, data_as_of, campus_line)?,
        ),
        (
            CARRIER_SPEC,
            dump_records(&pdb.carriers, data_as_of, carrier_line)?,
        ),
        (
            CARRIERFAC_SPEC,
            dump_vec(&pdb.carrier_facilities, data_as_of, carrierfac_line)?,
        ),
    ];
    Ok(tables)
}

// ---- table specs -----------------------------------------------------------

const NET_SPEC: TableSpec = TableSpec {
    task: "peeringdb.net",
    table: "peeringdb.net",
    staging_ddl: "CREATE TABLE peeringdb.net_staging (
        id integer NOT NULL,
        asn bigint,
        org_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, asn, org_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.net_staging ADD CONSTRAINT net_staging_pkey PRIMARY KEY (id)",
        "CREATE INDEX net_staging_asn_idx ON peeringdb.net_staging (asn)",
    ],
    index_swaps: &[
        ("peeringdb.net_staging_pkey", "net_pkey"),
        ("peeringdb.net_staging_asn_idx", "net_asn_idx"),
    ],
};

const IX_SPEC: TableSpec = TableSpec {
    task: "peeringdb.ix",
    table: "peeringdb.ix",
    staging_ddl: "CREATE TABLE peeringdb.ix_staging (
        id integer NOT NULL,
        org_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, org_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.ix_staging ADD CONSTRAINT ix_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.ix_staging_pkey", "ix_pkey")],
};

const IXLAN_SPEC: TableSpec = TableSpec {
    task: "peeringdb.ixlan",
    table: "peeringdb.ixlan",
    staging_ddl: "CREATE TABLE peeringdb.ixlan_staging (
        id integer NOT NULL,
        ix_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, ix_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.ixlan_staging ADD CONSTRAINT ixlan_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.ixlan_staging_pkey", "ixlan_pkey")],
};

const IXPFX_SPEC: TableSpec = TableSpec {
    task: "peeringdb.ixpfx",
    table: "peeringdb.ixpfx",
    staging_ddl: "CREATE TABLE peeringdb.ixpfx_staging (
        id integer NOT NULL,
        ixlan_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, ixlan_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.ixpfx_staging ADD CONSTRAINT ixpfx_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.ixpfx_staging_pkey", "ixpfx_pkey")],
};

const NETIXLAN_SPEC: TableSpec = TableSpec {
    task: "peeringdb.netixlan",
    table: "peeringdb.netixlan",
    staging_ddl: "CREATE TABLE peeringdb.netixlan_staging (
        id integer NOT NULL,
        asn bigint,
        net_id integer,
        ix_id integer,
        ixlan_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, asn, net_id, ix_id, ixlan_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.netixlan_staging ADD CONSTRAINT netixlan_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.netixlan_staging_pkey", "netixlan_pkey")],
};

const FAC_SPEC: TableSpec = TableSpec {
    task: "peeringdb.fac",
    table: "peeringdb.fac",
    staging_ddl: "CREATE TABLE peeringdb.fac_staging (
        id integer NOT NULL,
        org_id integer,
        campus_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, org_id, campus_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.fac_staging ADD CONSTRAINT fac_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.fac_staging_pkey", "fac_pkey")],
};

const NETFAC_SPEC: TableSpec = TableSpec {
    task: "peeringdb.netfac",
    table: "peeringdb.netfac",
    staging_ddl: "CREATE TABLE peeringdb.netfac_staging (
        id integer NOT NULL,
        net_id integer,
        fac_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, net_id, fac_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.netfac_staging ADD CONSTRAINT netfac_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.netfac_staging_pkey", "netfac_pkey")],
};

const IXFAC_SPEC: TableSpec = TableSpec {
    task: "peeringdb.ixfac",
    table: "peeringdb.ixfac",
    staging_ddl: "CREATE TABLE peeringdb.ixfac_staging (
        id integer NOT NULL,
        ix_id integer,
        fac_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, ix_id, fac_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.ixfac_staging ADD CONSTRAINT ixfac_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.ixfac_staging_pkey", "ixfac_pkey")],
};

const ORG_SPEC: TableSpec = TableSpec {
    task: "peeringdb.org",
    table: "peeringdb.org",
    staging_ddl: "CREATE TABLE peeringdb.org_staging (
        id integer NOT NULL,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.org_staging ADD CONSTRAINT org_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.org_staging_pkey", "org_pkey")],
};

const CAMPUS_SPEC: TableSpec = TableSpec {
    task: "peeringdb.campus",
    table: "peeringdb.campus",
    staging_ddl: "CREATE TABLE peeringdb.campus_staging (
        id integer NOT NULL,
        org_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, org_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.campus_staging ADD CONSTRAINT campus_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.campus_staging_pkey", "campus_pkey")],
};

const CARRIER_SPEC: TableSpec = TableSpec {
    task: "peeringdb.carrier",
    table: "peeringdb.carrier",
    staging_ddl: "CREATE TABLE peeringdb.carrier_staging (
        id integer NOT NULL,
        org_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, org_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.carrier_staging ADD CONSTRAINT carrier_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.carrier_staging_pkey", "carrier_pkey")],
};

const CARRIERFAC_SPEC: TableSpec = TableSpec {
    task: "peeringdb.carrierfac",
    table: "peeringdb.carrierfac",
    staging_ddl: "CREATE TABLE peeringdb.carrierfac_staging (
        id integer NOT NULL,
        carrier_id integer,
        fac_id integer,
        record jsonb NOT NULL,
        data_as_of timestamptz NOT NULL,
        source_revision text NOT NULL
    )",
    copy_columns: "id, carrier_id, fac_id, record, data_as_of, source_revision",
    index_ddl: &[
        "ALTER TABLE peeringdb.carrierfac_staging ADD CONSTRAINT carrierfac_staging_pkey PRIMARY KEY (id)",
    ],
    index_swaps: &[("peeringdb.carrierfac_staging_pkey", "carrierfac_pkey")],
};

// ---- row mapping -----------------------------------------------------------

/// Serialize one record to a double-quoted JSON field. Serialization cannot
/// fail for these plain data structs; a failure maps to a hard error rather
/// than a silently NULL record.
fn record_field<T: Serialize>(record: &T) -> Result<String, String> {
    serde_json::to_string(record).map_err(|e| format!("failed to serialize record: {e}"))
}

fn finish_line(id: u32, typed: &[Option<String>], record: String, data_as_of: &str) -> String {
    let mut fields = Vec::with_capacity(typed.len() + 3);
    fields.extend_from_slice(typed);
    fields.push(Some(record));
    fields.push(Some(data_as_of.to_string()));
    fields.push(Some(SOURCE_REVISION.to_string()));
    format!("{id},{}", build_csv_line(&fields))
}

fn opt_u32(v: Option<u32>) -> Option<String> {
    v.map(|x| x.to_string())
}

fn dump_networks(
    networks: &HashMap<u32, Network>,
    data_as_of: &str,
) -> Result<Vec<String>, String> {
    let mut asns: Vec<u32> = networks.keys().copied().collect();
    asns.sort_unstable();
    asns.iter()
        .map(|asn| {
            let n = networks
                .get(asn)
                .ok_or_else(|| format!("missing network {asn}"))?;
            let record = record_field(n)?;
            Ok(finish_line(
                n.id,
                &[opt_u32(n.asn), opt_u32(n.org_id)],
                record,
                data_as_of,
            ))
        })
        .collect()
}

fn dump_records<T: Serialize>(
    map: &HashMap<u32, T>,
    data_as_of: &str,
    line_fn: fn(&T, &str) -> Result<String, String>,
) -> Result<Vec<String>, String> {
    let mut ids: Vec<u32> = map.keys().copied().collect();
    ids.sort_unstable();
    ids.iter()
        .map(|id| {
            let r = map.get(id).ok_or_else(|| format!("missing record {id}"))?;
            line_fn(r, data_as_of)
        })
        .collect()
}

fn dump_vec<T>(
    records: &[T],
    data_as_of: &str,
    line_fn: fn(&T, &str) -> Result<String, String>,
) -> Result<Vec<String>, String> {
    records.iter().map(|r| line_fn(r, data_as_of)).collect()
}

fn ix_line(ix: &InternetExchange, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        ix.id,
        &[opt_u32(ix.org_id)],
        record_field(ix)?,
        data_as_of,
    ))
}

fn ixlan_line(lan: &IxLan, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        lan.id,
        &[opt_u32(Some(lan.ix_id))],
        record_field(lan)?,
        data_as_of,
    ))
}

fn ixp_ixpfx_line(pfx: &IxPrefix, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        pfx.id,
        &[opt_u32(Some(pfx.ixlan_id))],
        record_field(pfx)?,
        data_as_of,
    ))
}

fn netixlan_line(m: &NetworkIxLan, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        m.id,
        &[
            opt_u32(Some(m.asn)),
            opt_u32(Some(m.net_id)),
            opt_u32(Some(m.ix_id)),
            opt_u32(Some(m.ixlan_id)),
        ],
        record_field(m)?,
        data_as_of,
    ))
}

fn fac_line(fac: &Facility, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        fac.id,
        &[opt_u32(fac.org_id), opt_u32(fac.campus_id)],
        record_field(fac)?,
        data_as_of,
    ))
}

fn netfac_line(nf: &NetworkFacility, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        nf.id,
        &[opt_u32(Some(nf.net_id)), opt_u32(Some(nf.fac_id))],
        record_field(nf)?,
        data_as_of,
    ))
}

fn ixfac_line(xf: &IxFacility, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        xf.id,
        &[opt_u32(Some(xf.ix_id)), opt_u32(Some(xf.fac_id))],
        record_field(xf)?,
        data_as_of,
    ))
}

fn org_line(org: &Organization, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(org.id, &[], record_field(org)?, data_as_of))
}

fn campus_line(campus: &Campus, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        campus.id,
        &[opt_u32(campus.org_id)],
        record_field(campus)?,
        data_as_of,
    ))
}

fn carrier_line(carrier: &Carrier, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        carrier.id,
        &[opt_u32(carrier.org_id)],
        record_field(carrier)?,
        data_as_of,
    ))
}

fn carrierfac_line(cf: &CarrierFacility, data_as_of: &str) -> Result<String, String> {
    Ok(finish_line(
        cf.id,
        &[opt_u32(Some(cf.carrier_id)), opt_u32(Some(cf.fac_id))],
        record_field(cf)?,
        data_as_of,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct FakeRecord {
        id: u32,
        name: String,
    }

    #[test]
    fn finish_line_shape() {
        let record = record_field(&FakeRecord {
            id: 42,
            name: "test".to_string(),
        })
        .unwrap();
        let line = finish_line(
            42,
            &[Some("13335".to_string()), Some("7".to_string())],
            record,
            "2026-09-09T00:00:00+00:00",
        );
        assert!(line.starts_with("42,\"13335\",\"7\",\"{"));
        // inner JSON quotes are doubled by CSV escaping
        assert!(line.contains("\"\"id\"\":42"));
        assert!(line.ends_with(&format!("\"{SOURCE_REVISION}\"")));
    }

    #[test]
    fn null_fk_fields_are_sql_null() {
        let record = record_field(&FakeRecord {
            id: 1,
            name: "test".to_string(),
        })
        .unwrap();
        let line = finish_line(
            1,
            &[None, Some("3".to_string())],
            record,
            "2026-09-09T00:00:00+00:00",
        );
        assert!(line.starts_with("1,,\"3\",\"{"));
    }
}
