//! Load RIPE NCC historical RPKI VRP dump using JSON format.
//!
//! RIPE NCC provides historical RPKI data archives for all 5 RIRs:
//! - AFRINIC: <https://ftp.ripe.net/rpki/afrinic.tal/>
//! - APNIC: <https://ftp.ripe.net/rpki/apnic.tal/>
//! - ARIN: <https://ftp.ripe.net/rpki/arin.tal/>
//! - LACNIC: <https://ftp.ripe.net/rpki/lacnic.tal/>
//! - RIPE NCC: <https://ftp.ripe.net/rpki/ripencc.tal/>

use crate::Result;
use crate::rpki::rpki_client::RpkiClientData;
use crate::rpki::{Rir, RpkiFile, RpkiTrie};
use chrono::{Datelike, NaiveDate, Utc};
use tracing::info;

/// List available RIPE output.json.xz files for a given date from all RIRs.
pub fn list_ripe_files(date: NaiveDate) -> Result<Vec<RpkiFile>> {
    let mut files = vec![];

    for rir in [
        Rir::AFRINIC,
        Rir::APNIC,
        Rir::ARIN,
        Rir::LACNIC,
        Rir::RIPENCC,
    ] {
        let base_url = rir.to_ripe_ftp_root_url();
        let url = format!(
            "{}/{:04}/{:02}/{:02}/output.json.xz",
            base_url,
            date.year(),
            date.month(),
            date.day()
        );

        files.push(RpkiFile {
            url,
            // We don't have exact timestamp for RIPE files, use midnight of the date
            timestamp: date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .fixed_offset()
                .with_timezone(&Utc),
            size: None,
            rir: Some(rir),
            collector: None,
        });
    }

    Ok(files)
}

impl RpkiTrie {
    /// Load RPKI data from RIPE NCC historical archives for a specific date.
    ///
    /// This loads data from all 5 RIRs (AFRINIC, APNIC, ARIN, LACNIC, RIPENCC)
    /// using the output.json.xz format which contains ROAs and ASPAs.
    pub fn from_ripe_historical(date: NaiveDate) -> Result<Self> {
        let mut trie = RpkiTrie::new(Some(date));

        for rir in [
            Rir::AFRINIC,
            Rir::APNIC,
            Rir::ARIN,
            Rir::LACNIC,
            Rir::RIPENCC,
        ] {
            let url = format!(
                "{}/{:04}/{:02}/{:02}/output.json.xz",
                rir.to_ripe_ftp_root_url(),
                date.year(),
                date.month(),
                date.day()
            );
            info!("loading {} ROAs from {}", rir, url);

            let data = RpkiClientData::from_url(&url)?;
            trie.merge_rpki_client_data(data);
        }

        Ok(trie)
    }

    /// Load RPKI data from specific RIPE file URLs.
    pub fn from_ripe_files(urls: &[String], date: Option<NaiveDate>) -> Result<Self> {
        let mut trie = RpkiTrie::new(date);

        for url in urls {
            info!("loading ROAs from {}", url);
            let data = RpkiClientData::from_url(url)?;
            trie.merge_rpki_client_data(data);
        }

        Ok(trie)
    }
}
