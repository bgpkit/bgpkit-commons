//! IRR source registry.
//!
//! Defines the known IRR registries, their bulk dump URLs, and which transport
//! (HTTPS/FTP) and format (split files vs whole-DB) each one uses.

use crate::irr::IrrObjectType;

/// The transport used to fetch a dump file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Https,
    Ftp,
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transport::Https => write!(f, "HTTPS"),
            Transport::Ftp => write!(f, "FTP"),
        }
    }
}

/// How a registry publishes its data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpFormat {
    /// Per-object-type split files (e.g. `ripe.db.aut-num.gz`).
    /// Only RIPE and APNIC offer this.
    SplitFiles,
    /// Single whole-DB file containing all object types (e.g. `arin.db.gz`).
    WholeDb,
}

/// A single dump-file URL for a specific object type from a registry.
#[derive(Debug, Clone)]
pub struct IrrDumpUrl {
    /// The full URL to download.
    pub url: String,
    /// Transport protocol.
    pub transport: Transport,
    /// Whether this is a split file or whole-DB.
    pub format: DumpFormat,
}

/// An IRR registry source.
///
/// Each entry describes where to fetch bulk RPSL dump files for a registry,
/// what transport and format are used, and which object types are available.
#[derive(Debug, Clone)]
pub struct IrrSource {
    /// Short registry name (e.g. `"RIPE"`, `"RADB"`).
    pub name: &'static str,
    /// Full registry display name.
    pub display_name: &'static str,
    /// Whether the registry is an authoritative RIR database (vs third-party).
    pub authoritative: bool,
    /// Transport for the primary dump URLs.
    pub transport: Transport,
    /// Dump format.
    pub format: DumpFormat,
}

impl IrrSource {
    /// Returns the download URL(s) for a given object type from this registry.
    ///
    /// For `SplitFiles` registries (RIPE, APNIC), this returns a single URL
    /// targeting the requested type. For `WholeDb` registries, this returns
    /// a single URL for the whole database — the caller must filter by type
    /// during parsing.
    pub fn dump_urls(&self, object_type: IrrObjectType) -> Vec<IrrDumpUrl> {
        match (self.format, self.name) {
            (DumpFormat::SplitFiles, "RIPE") => {
                let filename = match object_type {
                    IrrObjectType::AutNum => "ripe.db.aut-num.gz",
                    IrrObjectType::Route => "ripe.db.route.gz",
                    IrrObjectType::Route6 => "ripe.db.route6.gz",
                    IrrObjectType::AsSet => "ripe.db.as-set.gz",
                    IrrObjectType::RouteSet => "ripe.db.route-set.gz",
                    IrrObjectType::Mntner => "ripe.db.mntner.gz",
                    IrrObjectType::Organisation => "ripe.db.organisation.gz",
                };
                vec![IrrDumpUrl {
                    url: format!("https://ftp.ripe.net/ripe/dbase/split/{filename}"),
                    transport: Transport::Https,
                    format: DumpFormat::SplitFiles,
                }]
            }
            (DumpFormat::SplitFiles, "APNIC") => {
                let filename = match object_type {
                    IrrObjectType::AutNum => "apnic.db.aut-num.gz",
                    IrrObjectType::Route => "apnic.db.route.gz",
                    IrrObjectType::Route6 => "apnic.db.route6.gz",
                    IrrObjectType::AsSet => "apnic.db.as-set.gz",
                    IrrObjectType::RouteSet => "apnic.db.route-set.gz",
                    IrrObjectType::Mntner => "apnic.db.mntner.gz",
                    IrrObjectType::Organisation => "apnic.db.organisation.gz",
                };
                vec![IrrDumpUrl {
                    url: format!("https://ftp.apnic.net/apnic/whois/{filename}"),
                    transport: Transport::Https,
                    format: DumpFormat::SplitFiles,
                }]
            }
            (DumpFormat::WholeDb, "ARIN") => {
                whole_db("https://ftp.arin.net/pub/rr/arin.db.gz", Transport::Https)
            }
            (DumpFormat::WholeDb, "LACNIC") => {
                whole_db("https://irr.lacnic.net/lacnic.db.gz", Transport::Https)
            }
            (DumpFormat::WholeDb, "AFRINIC") => whole_db(
                "https://ftp.afrinic.net/pub/dbase/afrinic.db.gz",
                Transport::Https,
            ),
            (DumpFormat::WholeDb, "NTTCOM") => whole_db(
                "https://rr1.ntt.net/nttcomRR/nttcom.db.gz",
                Transport::Https,
            ),
            (DumpFormat::WholeDb, "RADB") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/radb.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "ALTDB") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/altdb.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "BELL") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/bell.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "BBOI") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/bboi.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "JPIRR") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/jpirr.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "TC") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/tc.db.gz", Transport::Ftp)
            }
            (DumpFormat::WholeDb, "CANARIE") => whole_db(
                "https://whois.canarie.ca/dbase/canarie.db.gz",
                Transport::Https,
            ),
            (DumpFormat::WholeDb, "REACH") => {
                whole_db("ftp://ftp.radb.net/radb/dbase/reach.db.gz", Transport::Ftp)
            }
            _ => vec![],
        }
    }
}

fn whole_db(url: &str, transport: Transport) -> Vec<IrrDumpUrl> {
    vec![IrrDumpUrl {
        url: url.to_string(),
        transport,
        format: DumpFormat::WholeDb,
    }]
}

/// Returns the list of all known IRR sources.
pub fn all_sources() -> Vec<IrrSource> {
    SOURCES.to_vec()
}

/// Returns the default set of IRR sources.
///
/// These are the authoritative RIR databases plus the most commonly used
/// third-party IRRs. All use HTTPS except RADB which is FTP-only.
pub fn default_sources() -> Vec<IrrSource> {
    DEFAULT_SOURCES.to_vec()
}

/// Look up a source by name (case-sensitive, as used in the RPSL `source:` attribute).
pub fn source_by_name(name: &str) -> Option<IrrSource> {
    SOURCES.iter().find(|s| s.name == name).cloned()
}

const DEFAULT_SOURCES: &[IrrSource] = &[
    IrrSource {
        name: "RIPE",
        display_name: "RIPE NCC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::SplitFiles,
    },
    IrrSource {
        name: "APNIC",
        display_name: "APNIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::SplitFiles,
    },
    IrrSource {
        name: "ARIN",
        display_name: "ARIN (IRR)",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "LACNIC",
        display_name: "LACNIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "AFRINIC",
        display_name: "AFRINIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "NTTCOM",
        display_name: "NTT Communications",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "RADB",
        display_name: "RADB (MERIT)",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
];

const SOURCES: &[IrrSource] = &[
    IrrSource {
        name: "RIPE",
        display_name: "RIPE NCC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::SplitFiles,
    },
    IrrSource {
        name: "APNIC",
        display_name: "APNIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::SplitFiles,
    },
    IrrSource {
        name: "ARIN",
        display_name: "ARIN (IRR)",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "LACNIC",
        display_name: "LACNIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "AFRINIC",
        display_name: "AFRINIC",
        authoritative: true,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "NTTCOM",
        display_name: "NTT Communications",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "RADB",
        display_name: "RADB (MERIT)",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "ALTDB",
        display_name: "ALTDB",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "BELL",
        display_name: "Bell Canada",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "BBOI",
        display_name: "BBOI",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "JPIRR",
        display_name: "JPIRR",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "TC",
        display_name: "TC (Brasil)",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "CANARIE",
        display_name: "CANARIE",
        authoritative: false,
        transport: Transport::Https,
        format: DumpFormat::WholeDb,
    },
    IrrSource {
        name: "REACH",
        display_name: "Telstra Global",
        authoritative: false,
        transport: Transport::Ftp,
        format: DumpFormat::WholeDb,
    },
];
