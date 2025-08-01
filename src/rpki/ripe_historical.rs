//! load RIPE NCC historical RPKI VRP dump

use crate::Result;
use crate::rpki::{Rir, RoaEntry, RpkiTrie};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use ipnet::IpNet;
use std::str::FromStr;
use tracing::info;

impl RpkiTrie {
    pub fn from_ripe_historical(date: NaiveDate) -> Result<Self> {
        let mut trie = RpkiTrie::new(Some(date));
        for rir in [
            Rir::AFRINIC,
            Rir::APNIC,
            Rir::ARIN,
            Rir::LACNIC,
            Rir::RIPENCC,
        ] {
            let roas = Self::load_vrp_from_ripe(rir, date)?;
            for roa in roas {
                trie.insert_roa(roa);
            }
        }
        Ok(trie)
    }

    fn load_vrp_from_ripe(rir: Rir, date: NaiveDate) -> Result<Vec<RoaEntry>> {
        let mut roas = vec![];
        let base_url = rir.to_ripe_ftp_root_url();
        let url = format!(
            "{}/{:04}/{:02}/{:02}/roas.csv.xz",
            base_url,
            date.year(),
            date.month(),
            date.day()
        );
        info!("loading {} ROAs from {}", rir, url);
        for line in oneio::read_lines(&url)?.skip(1) {
            let line = line?;
            let mut fields = line.split(',');
            let _uri = fields.next().unwrap();
            let asn = fields
                .next()
                .unwrap()
                .to_lowercase()
                .strip_prefix("as")
                .unwrap()
                .parse::<u32>()?;
            let prefix = IpNet::from_str(fields.next().unwrap())?;
            let max_length = match fields.next().unwrap().parse::<u8>() {
                Ok(l) => l,
                Err(_) => continue,
            };
            let not_before =
                NaiveDateTime::parse_from_str(fields.next().unwrap(), "%Y-%m-%d %H:%M:%S").ok();
            let not_after =
                NaiveDateTime::parse_from_str(fields.next().unwrap(), "%Y-%m-%d %H:%M:%S").ok();
            let roa_entry = RoaEntry {
                prefix,
                asn,
                max_length,
                rir: Some(rir),
                not_before,
                not_after,
            };
            roas.push(roa_entry);
        }
        Ok(roas)
    }
}
