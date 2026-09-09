//! RIR delegated-statistics source records.
//!
//! This module fetches and parses delegated-statistics artifacts without
//! applying ASInfo enrichment policy.

use std::io::{BufRead, BufReader, Read};

use crate::{BgpkitCommonsError, Result};

/// URLs for the five RIR delegated-statistics artifacts.
pub const RIR_DELEGATED_STATS_URLS: &[&str] = &[
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-latest",
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-latest",
];

/// One source record from an RIR delegated-statistics artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatedRecord {
    pub registry: String,
    pub country: String,
    pub record_type: String,
    pub start: String,
    pub value: String,
    pub date: String,
    pub status: String,
    pub extensions: Vec<String>,
}

/// Parse delegated-statistics records from a caller-provided reader.
///
/// Empty lines, comments, and RIR summary lines (`registry|*|type|*|value|summary`,
/// which carry aggregate counts rather than records) are ignored. All
/// well-formed source records are returned without filtering by record type,
/// status, country, or ASN range.
pub fn parse_reader<R: Read>(reader: R) -> impl Iterator<Item = Result<DelegatedRecord>> {
    BufReader::new(reader)
        .lines()
        .filter_map(|line| match line {
            Err(error) => Some(Err(error.into())),
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }
                parse_line(line).transpose()
            }
        })
}

fn parse_line(line: &str) -> Result<Option<DelegatedRecord>> {
    let fields = line.split('|').collect::<Vec<_>>();
    // RIR summary lines (`registry|*|type|*|value|summary`) carry aggregate
    // counts for a type, not records. Both wildcard positions are checked so
    // a malformed six-field record still errors.
    if fields.len() == 6 && fields[1] == "*" && fields[3] == "*" && fields[5] == "summary" {
        return Ok(None);
    }
    if fields.len() < 7 {
        return Err(BgpkitCommonsError::invalid_format(
            "delegated statistics record",
            line,
            "expected at least 7 pipe-delimited fields",
        ));
    }

    Ok(Some(DelegatedRecord {
        registry: fields[0].to_string(),
        country: fields[1].to_string(),
        record_type: fields[2].to_string(),
        start: fields[3].to_string(),
        value: fields[4].to_string(),
        date: fields[5].to_string(),
        status: fields[6].to_string(),
        extensions: fields[7..]
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }))
}

/// Open a delegated-statistics artifact for streaming parsing.
pub fn fetch(url: &str) -> Result<Box<dyn Read>> {
    Ok(oneio::get_reader(url)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(input: &str) -> Vec<DelegatedRecord> {
        parse_reader(input.as_bytes())
            .collect::<Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn skips_comments_blanks_and_summary_lines() {
        let input =
            "# header\n\narin|*|asn|*|32961|summary\nripencc|NL|asn|1|1|20240101|allocated\n";
        let records = parse_all(input);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].registry, "ripencc");
    }

    #[test]
    fn malformed_short_line_is_an_error() {
        let input = "a|b|c|d|e\n";
        let result: Vec<_> = parse_reader(input.as_bytes()).collect();
        assert!(result.iter().any(|r| r.is_err()));
    }

    #[test]
    fn summary_like_short_line_without_wildcards_is_an_error() {
        // Six fields ending in "summary" but missing the `*` wildcard
        // positions is malformed data, not a summary line.
        let input = "arin|US|asn|1|5|summary\n";
        let result: Vec<_> = parse_reader(input.as_bytes()).collect();
        assert!(result.iter().any(|r| r.is_err()));
    }
}
