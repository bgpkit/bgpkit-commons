//! Extract typed [`IrrObject`]s from parsed RPSL objects.
//!
//! Each function reads the relevant attributes from a `rpsl::Object<Raw>` and
//! produces an owned typed struct. Unknown or non-target object types are
//! skipped by the caller (see [`crate::irr::stream`]).

use std::collections::BTreeMap;

use ipnet::IpNet;
use rpsl::{Object, spec::Raw};

use crate::BgpkitCommonsError;
use crate::irr::types::{
    AsSet, AutNum, IrrObject, IrrObjectType, Mntner, Organisation, Route, RouteSet,
};

type RpslObj<'a> = Object<'a, Raw>;

/// Collect all values for an attribute name from an RPSL object.
fn collect_values(obj: &RpslObj<'_>, name: &str) -> Vec<String> {
    obj.get(name).into_iter().map(String::from).collect()
}

/// Collect all values and flatten comma-separated entries.
/// Used for `members:` attributes which may contain `AS1, AS2, AS3`.
fn collect_values_flat(obj: &RpslObj<'_>, name: &str) -> Vec<String> {
    let mut result = Vec::new();
    for value in obj.get(name) {
        for part in value.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
        }
    }
    result
}

/// Collect all remaining attributes into a BTreeMap (excluding the ones
/// we already extracted into named fields).
fn collect_extra(obj: &RpslObj<'_>, skip: &[&str]) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    for attr in obj.iter() {
        let name = attr.name.as_ref();
        if skip.contains(&name) {
            continue;
        }
        let entry: &mut Vec<String> = map.entry(name.to_string()).or_default();
        for v in attr.value.with_content() {
            entry.push(v.to_string());
        }
    }
    map
}

/// Parse an ASN from an RPSL value string like `"AS13335"`, `"as13335"`, or `"13335"`.
/// Handles case-insensitive `AS` prefix and strips inline comments (e.g. `AS24665 # SUTC-AS`).
fn parse_asn(s: &str) -> Result<u32, BgpkitCommonsError> {
    let s = s.trim();
    // Strip inline comments (common in RADB route objects)
    let s = s.split('#').next().unwrap_or(s).trim();
    let digits = if s.len() >= 2 && s[..2].eq_ignore_ascii_case("AS") {
        &s[2..]
    } else {
        s
    };
    digits
        .parse::<u32>()
        .map_err(|_| BgpkitCommonsError::invalid_format("ASN", s, "not a valid AS number"))
}

/// Attempt to extract a typed IRR object from a parsed RPSL object.
///
/// Returns `Ok(Some(...))` for supported object types with parseable data,
/// `Ok(None)` for unsupported or empty object types, and `Err` if a supported
/// object has malformed critical fields.
pub fn extract(obj: &RpslObj<'_>) -> Result<Option<IrrObject>, BgpkitCommonsError> {
    let first = match obj.iter().next() {
        Some(attr) => attr.name.as_ref(),
        None => return Ok(None),
    };

    let obj_type = match IrrObjectType::from_first_attr(first) {
        Some(t) => t,
        None => return Ok(None),
    };

    let result = match obj_type {
        IrrObjectType::AutNum => {
            let asn_str = obj.get("aut-num").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("aut-num", "(empty)", "missing aut-num value")
            })?;
            IrrObject::AutNum(AutNum {
                asn: parse_asn(asn_str)?,
                as_name: collect_values(obj, "as-name")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                descr: collect_values(obj, "descr"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["aut-num", "as-name", "descr", "source"]),
            })
        }
        IrrObjectType::Route => {
            let prefix_str = obj.get("route").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("route", "(empty)", "missing route value")
            })?;
            let prefix: IpNet = prefix_str.trim().parse().map_err(|_| {
                BgpkitCommonsError::invalid_format("route", prefix_str, "not a valid IP prefix")
            })?;
            let origin_str = obj.get("origin").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("route", "(empty)", "missing origin value")
            })?;
            IrrObject::Route(Route {
                prefix,
                origin: parse_asn(origin_str)?,
                descr: collect_values(obj, "descr"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["route", "origin", "descr", "source"]),
            })
        }
        IrrObjectType::Route6 => {
            let prefix_str = obj.get("route6").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("route6", "(empty)", "missing route6 value")
            })?;
            let prefix: IpNet = prefix_str.trim().parse().map_err(|_| {
                BgpkitCommonsError::invalid_format("route6", prefix_str, "not a valid IPv6 prefix")
            })?;
            let origin_str = obj.get("origin").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("route6", "(empty)", "missing origin value")
            })?;
            IrrObject::Route6(Route {
                prefix,
                origin: parse_asn(origin_str)?,
                descr: collect_values(obj, "descr"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["route6", "origin", "descr", "source"]),
            })
        }
        IrrObjectType::AsSet => {
            let name = obj.get("as-set").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("as-set", "(empty)", "missing as-set value")
            })?;
            let mut members = Vec::new();
            let mut set_members = Vec::new();
            for m in collect_values_flat(obj, "members") {
                if m.starts_with("AS-") || m.starts_with("as-") || m.contains('-') {
                    set_members.push(m);
                } else {
                    match parse_asn(&m) {
                        Ok(asn) => members.push(asn),
                        Err(_) => set_members.push(m),
                    }
                }
            }
            IrrObject::AsSet(AsSet {
                name: name.to_string(),
                members,
                set_members,
                descr: collect_values(obj, "descr"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["as-set", "members", "descr", "source"]),
            })
        }
        IrrObjectType::RouteSet => {
            let name = obj.get("route-set").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format(
                    "route-set",
                    "(empty)",
                    "missing route-set value",
                )
            })?;
            let mut members = Vec::new();
            let mut set_members = Vec::new();
            for m in collect_values_flat(obj, "members") {
                if let Ok(prefix) = m.parse::<IpNet>() {
                    members.push(prefix);
                } else {
                    set_members.push(m);
                }
            }
            IrrObject::RouteSet(RouteSet {
                name: name.to_string(),
                members,
                set_members,
                descr: collect_values(obj, "descr"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["route-set", "members", "descr", "source"]),
            })
        }
        IrrObjectType::Mntner => {
            let name = obj.get("mntner").into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format("mntner", "(empty)", "missing mntner value")
            })?;
            IrrObject::Mntner(Mntner {
                name: name.to_string(),
                auth: collect_values(obj, "auth"),
                upd_to: collect_values(obj, "upd-to"),
                mnt_nfy: collect_values(obj, "mnt-nfy"),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(obj, &["mntner", "auth", "upd-to", "mnt-nfy", "source"]),
            })
        }
        IrrObjectType::Organisation => {
            let id_key = if first == "org" {
                "org"
            } else {
                "organisation"
            };
            let id = obj.get(id_key).into_iter().next().ok_or_else(|| {
                BgpkitCommonsError::invalid_format(
                    "organisation",
                    "(empty)",
                    "missing organisation value",
                )
            })?;
            let name = {
                let v = collect_values(obj, "org-name");
                if v.is_empty() {
                    collect_values(obj, "name")
                } else {
                    v
                }
            }
            .into_iter()
            .next()
            .unwrap_or_default();
            IrrObject::Organisation(Organisation {
                id: id.to_string(),
                name,
                org_type: collect_values(obj, "org-type").into_iter().next(),
                address: collect_values(obj, "address"),
                country: collect_values(obj, "country").into_iter().next(),
                abuse_c: collect_values(obj, "abuse-c").into_iter().next(),
                source: collect_values(obj, "source")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                extra: collect_extra(
                    obj,
                    &[
                        id_key, "org-name", "org-type", "address", "country", "abuse-c", "source",
                    ],
                ),
            })
        }
    };

    Ok(Some(result))
}
