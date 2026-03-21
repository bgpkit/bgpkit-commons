//! Load RPKI data from RPKISPOOL archives.
//!
//! The RPKISPOOL format ([draft-snijders-rpkispool-format]) provides historical
//! RPKI data in `.tar.zst` archives containing CCR (Canonical Cache Representation)
//! files ([draft-ietf-sidrops-rpki-ccr]) with validated ROA and ASPA payloads.
//!
//! Each RPKISPOOL archive contains CCR files from multiple vantage points,
//! with multiple snapshots per day. CCR files encode VRPs (Validated ROA Payloads)
//! and VAPs (Validated ASPA Payloads) in DER-encoded ASN.1.
//!
//! RPKISPOOL data is available from three mirrors:
//! - <https://josephine.sobornost.net/rpkidata/rpkispools/> (Netherlands)
//! - <https://dango.attn.jp/rpkidata/rpkispools/> (Japan)
//! - <https://rpkiviews.kerfuffle.net/rpkidata/rpkispools/> (United States)
//!
//! [draft-snijders-rpkispool-format]: https://datatracker.ietf.org/doc/draft-snijders-rpkispool-format/
//! [draft-ietf-sidrops-rpki-ccr]: https://datatracker.ietf.org/doc/draft-ietf-sidrops-rpki-ccr/

use crate::Result;
use crate::rpki::{Aspa, Roa, RpkiFile, RpkiTrie};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use tracing::info;

/// Available RPKISPOOL mirror collectors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpkiSpoolsCollector {
    /// josephine.sobornost.net - A2B Internet (AS51088), Amsterdam, Netherlands
    #[default]
    SobornostNet,
    /// dango.attn.jp - Internet Initiative Japan (AS2497), Tokyo, Japan
    AttnJp,
    /// rpkiviews.kerfuffle.net - Kerfuffle, LLC (AS35008), Fremont, California, United States
    KerfuffleNet,
}

impl RpkiSpoolsCollector {
    /// Get the HTTPS base URL for this collector's RPKISPOOL directory.
    pub fn base_url(&self) -> &'static str {
        match self {
            RpkiSpoolsCollector::SobornostNet => {
                "https://josephine.sobornost.net/rpkidata/rpkispools"
            }
            RpkiSpoolsCollector::AttnJp => "https://dango.attn.jp/rpkidata/rpkispools",
            RpkiSpoolsCollector::KerfuffleNet => {
                "https://rpkiviews.kerfuffle.net/rpkidata/rpkispools"
            }
        }
    }

    /// Get all available collectors
    pub fn all() -> Vec<RpkiSpoolsCollector> {
        vec![
            RpkiSpoolsCollector::SobornostNet,
            RpkiSpoolsCollector::AttnJp,
            RpkiSpoolsCollector::KerfuffleNet,
        ]
    }
}

impl std::fmt::Display for RpkiSpoolsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpkiSpoolsCollector::SobornostNet => write!(f, "sobornost.net"),
            RpkiSpoolsCollector::AttnJp => write!(f, "attn.jp"),
            RpkiSpoolsCollector::KerfuffleNet => write!(f, "kerfuffle.net"),
        }
    }
}

impl FromStr for RpkiSpoolsCollector {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sobornost.net" | "josephine.sobornost.net" => Ok(RpkiSpoolsCollector::SobornostNet),
            "attn.jp" | "dango.attn.jp" => Ok(RpkiSpoolsCollector::AttnJp),
            "kerfuffle.net" | "rpkiviews.kerfuffle.net" => Ok(RpkiSpoolsCollector::KerfuffleNet),
            _ => Err(format!("unknown RPKISPOOL collector: {}", s)),
        }
    }
}

/// Parsed data from an RPKISPOOL CCR file.
pub struct RpkiSpoolsData {
    /// Validated ROA Payloads
    pub roas: Vec<Roa>,
    /// Validated ASPA Payloads
    pub aspas: Vec<Aspa>,
}

/// Build the RPKISPOOL changelog archive URL for a given date.
///
/// The RPKISPOOL archive contains CCR snapshots throughout the day,
/// which is much more efficient to parse than the initstate archive.
pub fn rpkispool_url(collector: RpkiSpoolsCollector, date: NaiveDate) -> String {
    format!(
        "{}/{:04}/{:02}/{:02}/{:04}{:02}{:02}-rpkispool.tar.zst",
        collector.base_url(),
        date.year(),
        date.month(),
        date.day(),
        date.year(),
        date.month(),
        date.day()
    )
}

/// Build the initstate archive URL for a given date.
#[allow(dead_code)]
pub fn initstate_url(collector: RpkiSpoolsCollector, date: NaiveDate) -> String {
    format!(
        "{}/{:04}/{:02}/{:02}/{:04}{:02}{:02}-initstate.tar.zst",
        collector.base_url(),
        date.year(),
        date.month(),
        date.day(),
        date.year(),
        date.month(),
        date.day()
    )
}

/// List available RPKISPOOL files for a given date.
///
/// Returns the RPKISPOOL archive URL (contains CCR files) for the date.
pub fn list_rpkispools_files(
    collector: RpkiSpoolsCollector,
    date: NaiveDate,
) -> Result<Vec<RpkiFile>> {
    let url = rpkispool_url(collector, date);
    let timestamp = date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| DateTime::from_naive_utc_and_offset(dt, Utc).into());

    Ok(vec![RpkiFile {
        url,
        timestamp: timestamp.unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap()),
        size: None,
        rir: None,
        collector: None,
    }])
}

// ============================================================================
// CCR DER Parsing
// ============================================================================

/// Parse a CCR (Canonical Cache Representation) file and extract ROAs and ASPAs.
///
/// The CCR format (draft-ietf-sidrops-rpki-ccr) is a DER-encoded ASN.1 structure:
/// ```text
/// SEQUENCE {
///   OID (id-ct-rpkiCanonicalCacheRepresentation),
///   [0] EXPLICIT SEQUENCE {           -- the CCR content
///     SEQUENCE { OID }                -- hashAlg (SHA-256)
///     GeneralizedTime                 -- producedAt
///     [1] ManifestState OPTIONAL
///     [2] ROAPayloadState OPTIONAL    -- VRPs
///     [3] ASPAPayloadState OPTIONAL   -- VAPs
///     [4] TrustAnchorState OPTIONAL
///     [5] RouterKeyState OPTIONAL
///   }
/// }
/// ```
pub fn parse_ccr(data: &[u8]) -> Result<RpkiSpoolsData> {
    use bcder::Mode;
    use bcder::decode::SliceSource;

    let source = SliceSource::new(data);
    Mode::Der.decode(source, parse_ccr_content).map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKISPOOL",
            format!("Failed to parse CCR: {}", e),
        )
    })
}

/// Parse the outer ContentInfo-like wrapper and extract VRPs/VAPs.
fn parse_ccr_content<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
) -> std::result::Result<RpkiSpoolsData, bcder::decode::DecodeError<S::Error>> {
    use bcder::{Oid, Tag};

    cons.take_sequence(|cons| {
        // OID: id-ct-rpkiCanonicalCacheRepresentation (1.2.840.113549.1.9.16.1.54)
        let _oid = Oid::take_from(cons)?;

        // [0] EXPLICIT - the CCR content
        cons.take_constructed_if(Tag::CTX_0, |cons| {
            cons.take_sequence(|cons| {
                // hashAlg: DigestAlgorithmIdentifier (SEQUENCE { OID })
                cons.take_sequence(|cons| {
                    let _hash_oid = Oid::take_from(cons)?;
                    // Some implementations include NULL parameters
                    cons.take_opt_null()?;
                    Ok(())
                })?;

                // producedAt: GeneralizedTime - skip
                cons.take_value(|_tag, content| {
                    content.as_primitive()?.skip_all()?;
                    Ok(())
                })?;

                let mut roas = Vec::new();
                let mut aspas = Vec::new();

                // [1] ManifestState OPTIONAL - skip
                cons.take_opt_constructed_if(Tag::CTX_1, |cons| {
                    // Skip all manifest content
                    cons.capture_all()?;
                    Ok(())
                })?;

                // [2] ROAPayloadState OPTIONAL - parse
                if let Some(roa_data) =
                    cons.take_opt_constructed_if(Tag::CTX_2, |cons| parse_roa_payload_state(cons))?
                {
                    roas = roa_data;
                }

                // [3] ASPAPayloadState OPTIONAL - parse
                if let Some(aspa_data) =
                    cons.take_opt_constructed_if(Tag::CTX_3, |cons| parse_aspa_payload_state(cons))?
                {
                    aspas = aspa_data;
                }

                // Skip remaining optional fields ([4] TAS, [5] RKS, ...)
                // by consuming all remaining content
                cons.capture_all()?;

                Ok(RpkiSpoolsData { roas, aspas })
            })
        })
    })
}

/// Parse ROAPayloadState:
/// ```text
/// ROAPayloadState ::= SEQUENCE {
///   rps   SEQUENCE OF ROAPayloadSet,
///   hash  Digest }
/// ```
fn parse_roa_payload_state<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
) -> std::result::Result<Vec<Roa>, bcder::decode::DecodeError<S::Error>> {
    // ROAPayloadState is a SEQUENCE
    cons.take_sequence(|cons| {
        // rps: SEQUENCE OF ROAPayloadSet
        let roas = cons.take_sequence(|cons| {
            let mut all_roas = Vec::new();
            // Each ROAPayloadSet
            while let Some(set_roas) = cons.take_opt_sequence(|cons| parse_roa_payload_set(cons))? {
                all_roas.extend(set_roas);
            }
            Ok(all_roas)
        })?;

        // hash: Digest (OCTET STRING) - skip
        cons.capture_all()?;

        Ok(roas)
    })
}

/// Parse ROAPayloadSet:
/// ```text
/// ROAPayloadSet ::= SEQUENCE {
///   asID          ASID,
///   ipAddrBlocks  SEQUENCE (SIZE(1..2)) OF ROAIPAddressFamily }
/// ```
fn parse_roa_payload_set<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
) -> std::result::Result<Vec<Roa>, bcder::decode::DecodeError<S::Error>> {
    let asn = cons.take_u32()?;

    // ipAddrBlocks: SEQUENCE (SIZE(1..2)) OF ROAIPAddressFamily
    let roas = cons.take_sequence(|cons| {
        let mut roas = Vec::new();
        // Each ROAIPAddressFamily
        while let Some(family_roas) =
            cons.take_opt_sequence(|cons| parse_roa_ip_address_family(cons, asn))?
        {
            roas.extend(family_roas);
        }
        Ok(roas)
    })?;

    Ok(roas)
}

/// Parse ROAIPAddressFamily (from RFC 9582):
/// ```text
/// ROAIPAddressFamily ::= SEQUENCE {
///   addressFamily OCTET STRING (SIZE (2..3)),
///   addresses     SEQUENCE OF ROAIPAddress }
///
/// ROAIPAddress ::= SEQUENCE {
///   address   IPAddress,    -- BIT STRING
///   maxLength INTEGER OPTIONAL }
/// ```
fn parse_roa_ip_address_family<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
    asn: u32,
) -> std::result::Result<Vec<Roa>, bcder::decode::DecodeError<S::Error>> {
    // addressFamily: OCTET STRING (2 bytes for AFI)
    let family_bytes = bcder::OctetString::take_from(cons)?;
    let family_slice = family_bytes.to_bytes();

    let is_ipv4 = match family_slice.as_ref() {
        [0, 1] => true,     // IPv4
        [0, 2] => false,    // IPv6
        [0, 1, _] => true,  // IPv4 with SAFI
        [0, 2, _] => false, // IPv6 with SAFI
        _ => {
            return Err(cons.content_err("unknown address family in ROAIPAddressFamily"));
        }
    };

    // addresses: SEQUENCE OF ROAIPAddress
    let roas = cons.take_sequence(|cons| {
        let mut roas = Vec::new();
        while let Some(roa) =
            cons.take_opt_sequence(|cons| parse_roa_ip_address(cons, asn, is_ipv4))?
        {
            roas.push(roa);
        }
        Ok(roas)
    })?;

    Ok(roas)
}

/// Parse a single ROAIPAddress:
/// ```text
/// ROAIPAddress ::= SEQUENCE {
///   address   IPAddress,    -- BIT STRING representing prefix
///   maxLength INTEGER OPTIONAL }
/// ```
fn parse_roa_ip_address<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
    asn: u32,
    is_ipv4: bool,
) -> std::result::Result<Roa, bcder::decode::DecodeError<S::Error>> {
    // address: BIT STRING
    let bit_string = bcder::BitString::take_from(cons)?;
    let unused_bits = bit_string.unused();
    let octets = bit_string.octet_bytes();

    let prefix_len = (octets.len() as u8) * 8 - unused_bits;

    let ip_addr: IpAddr = if is_ipv4 {
        let mut addr_bytes = [0u8; 4];
        for (i, &b) in octets.iter().enumerate().take(4) {
            addr_bytes[i] = b;
        }
        IpAddr::V4(Ipv4Addr::from(addr_bytes))
    } else {
        let mut addr_bytes = [0u8; 16];
        for (i, &b) in octets.iter().enumerate().take(16) {
            addr_bytes[i] = b;
        }
        IpAddr::V6(Ipv6Addr::from(addr_bytes))
    };

    // maxLength: INTEGER OPTIONAL
    let max_length = cons.take_opt_u8()?.unwrap_or(prefix_len);

    let prefix_str = format!("{}/{}", ip_addr, prefix_len);
    let prefix: IpNet = prefix_str
        .parse()
        .map_err(|_| cons.content_err(format!("invalid IP prefix: {}", prefix_str)))?;

    Ok(Roa {
        prefix,
        asn,
        max_length,
        rir: None,
        not_before: None,
        not_after: None,
    })
}

/// Parse ASPAPayloadState:
/// ```text
/// ASPAPayloadState ::= SEQUENCE {
///   aps   SEQUENCE OF ASPAPayloadSet,
///   hash  Digest }
///
/// ASPAPayloadSet ::= SEQUENCE {
///   customerASID  ASID,
///   providers     SEQUENCE (SIZE(1..MAX)) OF ASID }
/// ```
fn parse_aspa_payload_state<S: bcder::decode::Source>(
    cons: &mut bcder::decode::Constructed<S>,
) -> std::result::Result<Vec<Aspa>, bcder::decode::DecodeError<S::Error>> {
    cons.take_sequence(|cons| {
        // aps: SEQUENCE OF ASPAPayloadSet
        let aspas = cons.take_sequence(|cons| {
            let mut all_aspas = Vec::new();
            while let Some(aspa) = cons.take_opt_sequence(|cons| {
                let customer_asn = cons.take_u32()?;
                let providers = cons.take_sequence(|cons| {
                    let mut provs = Vec::new();
                    while let Some(p) = cons.take_opt_u32()? {
                        provs.push(p);
                    }
                    Ok(provs)
                })?;
                Ok(Aspa {
                    customer_asn,
                    providers,
                    expires: None,
                })
            })? {
                all_aspas.push(aspa);
            }
            Ok(all_aspas)
        })?;

        // hash: Digest (OCTET STRING) - skip
        cons.capture_all()?;

        Ok(aspas)
    })
}

// ============================================================================
// Streaming tar.zst and extracting CCR files
// ============================================================================

/// Parse an RPKISPOOL archive from a URL, extracting the first CCR file's VRPs and VAPs.
///
/// This streams the `.tar.zst` archive and parses the first CCR file found,
/// which contains all validated ROA and ASPA payloads from one vantage point snapshot.
pub fn parse_rpkispools_archive(url: &str) -> Result<RpkiSpoolsData> {
    info!("streaming RPKISPOOL archive: {}", url);

    let reader = oneio::get_reader_raw(url).map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKISPOOL",
            format!("Failed to fetch {}: {}", url, e),
        )
    })?;

    // Decompress zstd stream
    let decoder = zstd::Decoder::new(reader).map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKISPOOL",
            format!("Failed to create zstd decoder: {}", e),
        )
    })?;

    // Read tar entries
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKISPOOL",
            format!("Failed to read tar entries: {}", e),
        )
    })?;

    for entry_result in entries {
        let mut entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = match entry.path() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Find the first .ccr file
        if path.ends_with(".ccr") {
            info!("parsing CCR file: {} ({} bytes)", path, entry.size());
            let mut ccr_data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut ccr_data).map_err(|e| {
                crate::BgpkitCommonsError::data_source_error(
                    "RPKISPOOL",
                    format!("Failed to read CCR entry {}: {}", path, e),
                )
            })?;

            return parse_ccr(&ccr_data);
        }
    }

    Err(crate::BgpkitCommonsError::data_source_error(
        "RPKISPOOL",
        format!("No CCR file found in archive: {}", url),
    ))
}

// ============================================================================
// RpkiTrie integration
// ============================================================================

impl RpkiTrie {
    /// Load RPKI data from an RPKISPOOL archive for a specific date.
    ///
    /// This downloads the RPKISPOOL archive for the given date and parses
    /// the first CCR file to extract VRPs and ASPAs.
    pub fn from_rpkispools(collector: RpkiSpoolsCollector, date: NaiveDate) -> Result<Self> {
        let url = rpkispool_url(collector, date);
        info!(
            "loading RPKISPOOL data from {} for date {}",
            collector, date
        );
        Self::from_rpkispools_url(&url, Some(date))
    }

    /// Load RPKI data from a specific RPKISPOOL archive URL.
    pub fn from_rpkispools_url(url: &str, date: Option<NaiveDate>) -> Result<Self> {
        let data = parse_rpkispools_archive(url)?;
        let mut trie = RpkiTrie::new(date);
        trie.insert_roas(data.roas);
        for aspa in data.aspas {
            if !trie
                .aspas
                .iter()
                .any(|a| a.customer_asn == aspa.customer_asn)
            {
                trie.aspas.push(aspa);
            }
        }
        Ok(trie)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_urls() {
        assert_eq!(
            RpkiSpoolsCollector::SobornostNet.base_url(),
            "https://josephine.sobornost.net/rpkidata/rpkispools"
        );
        let date = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
        assert_eq!(
            rpkispool_url(RpkiSpoolsCollector::AttnJp, date),
            "https://dango.attn.jp/rpkidata/rpkispools/2026/03/20/20260320-rpkispool.tar.zst"
        );
        assert_eq!(
            initstate_url(RpkiSpoolsCollector::KerfuffleNet, date),
            "https://rpkiviews.kerfuffle.net/rpkidata/rpkispools/2026/03/20/20260320-initstate.tar.zst"
        );
    }

    #[test]
    fn test_collector_from_str() {
        assert_eq!(
            RpkiSpoolsCollector::from_str("sobornost.net").unwrap(),
            RpkiSpoolsCollector::SobornostNet
        );
        assert_eq!(
            RpkiSpoolsCollector::from_str("dango.attn.jp").unwrap(),
            RpkiSpoolsCollector::AttnJp
        );
    }

    #[test]
    fn test_default_collector() {
        assert_eq!(
            RpkiSpoolsCollector::default(),
            RpkiSpoolsCollector::SobornostNet
        );
    }

    #[test]
    #[ignore] // Requires network access
    fn test_from_rpkispools() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
        let trie = RpkiTrie::from_rpkispools(RpkiSpoolsCollector::AttnJp, date)
            .expect("failed to load RPKISPOOL data");

        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!("loaded {} ROAs from RPKISPOOL for {}", total_roas, date);
        println!("Loaded {} ASPAs", trie.aspas.len());

        assert!(total_roas > 0, "Should have loaded some ROAs");
        assert!(!trie.aspas.is_empty(), "Should have loaded some ASPAs");
    }

    #[test]
    #[ignore] // Requires network access
    fn test_parse_ccr_from_stream() {
        let url = "https://dango.attn.jp/rpkidata/rpkispools/2026/03/20/20260320-rpkispool.tar.zst";
        let data = parse_rpkispools_archive(url).expect("failed to parse RPKISPOOL archive");

        println!("Parsed {} ROAs", data.roas.len());
        println!("Parsed {} ASPAs", data.aspas.len());

        // Print some sample ROAs
        for roa in data.roas.iter().take(5) {
            println!(
                "  ROA: {}/{} AS{} max_length={}",
                roa.prefix,
                roa.prefix.prefix_len(),
                roa.asn,
                roa.max_length
            );
        }

        // Print some sample ASPAs
        for aspa in data.aspas.iter().take(5) {
            println!(
                "  ASPA: AS{} providers={:?}",
                aspa.customer_asn, aspa.providers
            );
        }

        assert!(data.roas.len() > 10000, "Expected many ROAs from CCR");
        assert!(!data.aspas.is_empty(), "Expected some ASPAs from CCR");
    }
}
