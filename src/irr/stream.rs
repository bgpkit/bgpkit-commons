//! Streaming parser for IRR bulk dump files.
//!
//! Reads gzipped RPSL text from a URL (via oneio), splits it into
//! per-object chunks at blank lines, parses each with `rpsl::parse_object`,
//! and extracts typed [`IrrObject`]s via [`crate::irr::extract`].
//!
//! For `WholeDb` dumps, objects whose first attribute doesn't match any
//! supported type are skipped cheaply (a string comparison on the first
//! token of each object).

use std::io::{BufRead, BufReader, Read};

use tracing::{info, warn};

use crate::irr::sources::{DumpFormat, IrrDumpUrl, IrrSource, source_by_name};
use crate::irr::types::IrrObjectType;
use crate::irr::types::{IrrAttribute, IrrObject, IrrRecord};
use crate::{BgpkitCommonsError, Result};

/// Statistics from a single dump-file parse pass.
#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    /// Total RPSL objects encountered (including skipped ones).
    pub total_objects: usize,
    /// Objects successfully extracted as typed IRR objects.
    pub extracted: usize,
    /// Objects skipped (unsupported type or parse error).
    pub skipped: usize,
    /// Objects that failed extraction with an error.
    pub errors: usize,
}

/// Streaming iterator over source-faithful RPSL records.
pub struct IrrRecordIter<R: Read> {
    reader: BufReader<R>,
    format: DumpFormat,
    line: Vec<u8>,
    object_lines: Vec<String>,
    finished: bool,
}

/// Parse source-faithful RPSL records from a caller-provided reader.
pub fn parse_reader<R: Read>(reader: R, format: DumpFormat) -> IrrRecordIter<R> {
    IrrRecordIter {
        reader: BufReader::new(reader),
        format,
        line: Vec::new(),
        object_lines: Vec::new(),
        finished: false,
    }
}

impl<R: Read> IrrRecordIter<R> {
    /// Return the publication format associated with this record stream.
    pub fn format(&self) -> DumpFormat {
        self.format
    }
}

impl<R: Read> Iterator for IrrRecordIter<R> {
    type Item = Result<IrrRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            self.line.clear();
            match self.reader.read_until(b'\n', &mut self.line) {
                Err(error) => {
                    self.finished = true;
                    return Some(Err(error.into()));
                }
                Ok(0) => {
                    self.finished = true;
                    if self.object_lines.is_empty() {
                        return None;
                    }
                    return Some(parse_record_lines(std::mem::take(&mut self.object_lines)));
                }
                Ok(_) => {}
            }

            let line = String::from_utf8_lossy(&self.line);
            let line = line.trim_end_matches(['\n', '\r']);
            if line.trim().is_empty() || line.starts_with('#') || line.starts_with('%') {
                if self.object_lines.is_empty() {
                    continue;
                }
                return Some(parse_record_lines(std::mem::take(&mut self.object_lines)));
            }
            self.object_lines.push(line.to_string());
        }
    }
}

fn parse_record_lines(lines: Vec<String>) -> Result<IrrRecord> {
    let mut attributes: Vec<IrrAttribute> = Vec::new();
    for line in lines {
        if line.starts_with(char::is_whitespace) {
            let Some(attribute) = attributes.last_mut() else {
                return Err(BgpkitCommonsError::invalid_format(
                    "RPSL object",
                    line,
                    "continuation line without an attribute",
                ));
            };
            attribute.value.push('\n');
            attribute.value.push_str(line.trim());
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(BgpkitCommonsError::invalid_format(
                "RPSL object",
                line,
                "attribute line is missing ':'",
            ));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(BgpkitCommonsError::invalid_format(
                "RPSL object",
                line,
                "attribute name is empty",
            ));
        }
        attributes.push(IrrAttribute {
            name: name.to_string(),
            value: value.trim().to_string(),
        });
    }

    let object_type = attributes
        .first()
        .map(|attribute| attribute.name.clone())
        .ok_or_else(|| {
            BgpkitCommonsError::invalid_format("RPSL object", "", "object has no attributes")
        })?;
    Ok(IrrRecord {
        object_type,
        attributes,
    })
}

impl std::fmt::Display for ParseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} extracted, {} skipped, {} errors (of {} total)",
            self.extracted, self.skipped, self.errors, self.total_objects
        )
    }
}

/// Parse a single dump file and call `handler` for each extracted object.
///
/// The handler receives each [`IrrObject`] and can collect, filter, or
/// process them as needed. This is the core building block for all
/// IRR data loading.
///
/// # Arguments
///
/// * `dump_url` — The dump file descriptor (URL, transport, format).
/// * `handler` — A closure called for each successfully extracted object.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn parse_dump<F>(dump_url: &IrrDumpUrl, mut handler: F) -> Result<ParseStats>
where
    F: FnMut(IrrObject),
{
    info!("parsing IRR dump: {}", dump_url.url);
    let reader = fetch_dump_url(dump_url)?;
    parse_dump_from_reader(reader, dump_url.format, &mut handler)
}

/// Reader returned by [`fetch`], including the selected dump metadata.
pub struct IrrReader {
    /// Canonical catalog URL and publication format selected for this reader.
    pub dump_url: IrrDumpUrl,
    reader: Box<dyn Read>,
}

impl Read for IrrReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buffer)
    }
}

/// Validate a catalog source and open its dump for an object type.
pub fn fetch(source: &IrrSource, object_type: IrrObjectType) -> Result<IrrReader> {
    let source = source_by_name(source.name).ok_or_else(|| {
        BgpkitCommonsError::invalid_format("IRR source", source.name, "unknown registry name")
    })?;
    let dump_url = source
        .dump_urls(object_type)
        .into_iter()
        .next()
        .ok_or_else(|| {
            BgpkitCommonsError::invalid_format(
                "IRR dump",
                source.name,
                format!("no dump for {}", object_type.key_attr()),
            )
        })?;
    let reader = fetch_dump_url(&dump_url)?;
    Ok(IrrReader { dump_url, reader })
}

fn fetch_dump_url(dump_url: &IrrDumpUrl) -> Result<Box<dyn Read>> {
    Ok(oneio::get_reader(&dump_url.url)?)
}

/// Parse a dump file from any reader (for testing or custom I/O).
pub fn parse_dump_from_reader<R: std::io::Read, F>(
    reader: R,
    format: DumpFormat,
    handler: &mut F,
) -> Result<ParseStats>
where
    F: FnMut(IrrObject),
{
    let mut stats = ParseStats::default();
    for record in parse_reader(reader, format) {
        stats.total_objects += 1;
        match record.and_then(|record| record.to_typed()) {
            Ok(Some(typed)) => {
                stats.extracted += 1;
                handler(typed);
            }
            Ok(None) => stats.skipped += 1,
            Err(error) => {
                stats.errors += 1;
                warn!("IRR parse error: {error}");
            }
        }
    }

    info!("IRR dump parsed: {}", stats);
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irr::types::IrrObject;
    use std::io::Cursor;

    const SAMPLE_RIPE_AUTNUM: &str = "\
aut-num:        AS13335
as-name:        CLOUDFLARENET
descr:          Cloudflare, Inc.
org:            ORG-CF165-RIPE
mnt-by:         CLOUDFLARE-MNT
source:         RIPE

aut-num:        AS15169
as-name:        GOOGLE
descr:          Google LLC
source:         RIPE

route:          1.1.1.0/24
origin:         AS13335
descr:          Cloudflare
source:         RIPE
";

    #[test]
    fn test_parse_aut_num_and_route() {
        let mut objects = Vec::new();
        let stats = parse_dump_from_reader(
            Cursor::new(SAMPLE_RIPE_AUTNUM),
            DumpFormat::SplitFiles,
            &mut |obj| objects.push(obj),
        )
        .unwrap();

        assert_eq!(stats.total_objects, 3);
        assert_eq!(stats.extracted, 3);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);

        // First: aut-num AS13335
        match &objects[0] {
            IrrObject::AutNum(a) => {
                assert_eq!(a.asn, 13335);
                assert_eq!(a.as_name, "CLOUDFLARENET");
                assert_eq!(a.source, "RIPE");
                assert!(a.descr.iter().any(|d| d.contains("Cloudflare")));
            }
            other => panic!("expected AutNum, got {other:?}"),
        }

        // Second: aut-num AS15169
        match &objects[1] {
            IrrObject::AutNum(a) => {
                assert_eq!(a.asn, 15169);
                assert_eq!(a.as_name, "GOOGLE");
            }
            other => panic!("expected AutNum, got {other:?}"),
        }

        // Third: route 1.1.1.0/24
        match &objects[2] {
            IrrObject::Route(r) => {
                assert_eq!(r.prefix.to_string(), "1.1.1.0/24");
                assert_eq!(r.origin, 13335);
            }
            other => panic!("expected Route, got {other:?}"),
        }
    }

    const SAMPLE_ASSET: &str = "\
as-set:         AS-EXAMPLE
descr:          Example AS set
members:        AS64496, AS64497
members:        AS-EXAMPLE-CLIENTS
members:        AS64500
source:         RIPE
";

    #[test]
    fn test_parse_as_set() {
        let mut objects = Vec::new();
        parse_dump_from_reader(
            Cursor::new(SAMPLE_ASSET),
            DumpFormat::SplitFiles,
            &mut |obj| objects.push(obj),
        )
        .unwrap();

        assert_eq!(objects.len(), 1);
        match &objects[0] {
            IrrObject::AsSet(s) => {
                assert_eq!(s.name, "AS-EXAMPLE");
                assert_eq!(s.members, vec![64496, 64497, 64500]);
                assert_eq!(s.set_members, vec!["AS-EXAMPLE-CLIENTS"]);
            }
            other => panic!("expected AsSet, got {other:?}"),
        }
    }

    const SAMPLE_WITH_COMMENTS: &str = "\
# This is a comment
# Another comment

aut-num:        AS3356
as-name:        LEVEL3
source:         RIPE
";

    #[test]
    fn test_skip_comments() {
        let mut objects = Vec::new();
        let stats = parse_dump_from_reader(
            Cursor::new(SAMPLE_WITH_COMMENTS),
            DumpFormat::SplitFiles,
            &mut |obj| objects.push(obj),
        )
        .unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(stats.extracted, 1);
    }

    const SAMPLE_SKIP_UNKNOWN: &str = "\
person:         John Doe
nic-hdl:        JD-RIPE
source:         RIPE

aut-num:        AS1299
as-name:        TWELVE99
source:         RIPE
";

    #[test]
    fn test_skip_unknown_types() {
        let mut objects = Vec::new();
        let stats = parse_dump_from_reader(
            Cursor::new(SAMPLE_SKIP_UNKNOWN),
            DumpFormat::WholeDb,
            &mut |obj| objects.push(obj),
        )
        .unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(stats.total_objects, 2);
        assert_eq!(stats.extracted, 1);
        assert_eq!(stats.skipped, 1);
    }
}
