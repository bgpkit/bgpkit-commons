//! Streaming parser for IRR bulk dump files.
//!
//! Reads gzipped RPSL text from a URL (via oneio), splits it into
//! per-object chunks at blank lines, parses each with `rpsl::parse_object`,
//! and extracts typed [`IrrObject`]s via [`crate::irr::extract`].
//!
//! For `WholeDb` dumps, objects whose first attribute doesn't match any
//! supported type are skipped cheaply (a string comparison on the first
//! token of each object).

use std::io::{BufRead, BufReader};

use rpsl::parse_object;
use tracing::{info, warn};

use crate::Result;
use crate::irr::extract;
use crate::irr::sources::{DumpFormat, IrrDumpUrl};
use crate::irr::types::IrrObject;

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
    let reader = oneio::get_reader(&dump_url.url)?;
    parse_dump_from_reader(reader, dump_url.format, &mut handler)
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
    let buf_reader = BufReader::new(reader);
    let mut stats = ParseStats::default();

    // RPSL objects are separated by blank lines.
    // Comment lines (starting with # or %) are outside objects.
    // Continuation lines start with whitespace (tab/space+).
    //
    // We use read_until instead of lines() because IRR dumps occasionally
    // contain non-UTF-8 bytes (latin-1 etc.). We lossily convert to String.
    let mut chunk = String::new();
    let mut in_object = false;

    let mut buf = Vec::new();
    let mut reader = buf_reader;

    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        // Lossily convert to handle non-UTF-8 bytes in legacy IRR data
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        if line.trim().is_empty() {
            // Blank line: end of current object
            if in_object && !chunk.is_empty() {
                process_chunk(&chunk, format, handler, &mut stats);
                chunk.clear();
                in_object = false;
            }
            continue;
        }

        if line.starts_with('#') || line.starts_with('%') {
            // Comment line: only ends an object if we're in one
            if in_object && !chunk.is_empty() {
                process_chunk(&chunk, format, handler, &mut stats);
                chunk.clear();
                in_object = false;
            }
            continue;
        }

        in_object = true;
        chunk.push_str(line);
        chunk.push('\n');
    }

    // Process trailing object (no blank line at EOF)
    if !chunk.is_empty() {
        process_chunk(&chunk, format, handler, &mut stats);
    }

    info!("IRR dump parsed: {}", stats);
    Ok(stats)
}

fn process_chunk<F>(chunk: &str, _format: DumpFormat, handler: &mut F, stats: &mut ParseStats)
where
    F: FnMut(IrrObject),
{
    stats.total_objects += 1;

    // Quick check: skip comment-only chunks
    let trimmed = chunk.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        stats.skipped += 1;
        return;
    }

    // rpsl::parse_object requires a trailing blank line to delimit the object.
    // Ensure the chunk ends with "\n\n".
    let owned;
    let text = if chunk.ends_with("\n\n") {
        chunk
    } else {
        owned = format!("{chunk}\n");
        &owned
    };

    let parsed = match parse_object(text) {
        Ok(obj) => obj,
        Err(e) => {
            stats.errors += 1;
            // Only log first few chars for debugging
            let preview: String = chunk.chars().take(80).collect();
            warn!("RPSL parse error: {e} (preview: {preview}...)");
            return;
        }
    };

    match extract::extract(&parsed) {
        Ok(Some(typed)) => {
            stats.extracted += 1;
            handler(typed);
        }
        Ok(None) => {
            stats.skipped += 1;
        }
        Err(e) => {
            stats.errors += 1;
            warn!("IRR extract error: {e}");
        }
    }
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
