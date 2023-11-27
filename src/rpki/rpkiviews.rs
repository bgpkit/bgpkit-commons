/// RPKI historical archive data from http://www.rpkiviews.org/
use crate::rpki::{Rir, RoaEntry, RpkiTrie};
use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use ipnet::IpNet;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize)]
struct RpkiViewsData {
    roas: Vec<RpkiViewsEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct RpkiViewsEntry {
    prefix: String,
    #[serde(rename = "maxLength")]
    max_length: u8,
    asn: u32,
    ta: String,
}

fn extract_file_names(html: &str) -> Vec<&str> {
    let re = regex::Regex::new(r#"<a\s+href="([^"]+)""#).unwrap();
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .filter(|file_name| file_name.ends_with(".tgz"))
        .collect()
}

fn parse_file_name_timestamp(file_name: &str) -> Result<NaiveDateTime> {
    // Assuming the timestamp format is "rpki-20230101T000545Z.tgz"
    let timestamp_str = &file_name[5..21]; // Extract the timestamp part

    // Parse the timestamp using chrono
    let datetime = NaiveDateTime::parse_from_str(timestamp_str, "%Y%m%dT%H%M%SZ")?;

    Ok(datetime)
}

const DATA_ROOT_URL: &str = "http://josephine.sobornost.net/josephine.sobornost.net/rpkidata";

impl RpkiTrie {
    /// load RPKI historical data from http://www.rpkiviews.org/
    ///
    /// It loads the first data dump for the given date.
    pub fn from_rpkiviews_historical(date: NaiveDate) -> Result<Self> {
        let roas = load_vrp_from_rpkiviews(date)?;
        let mut trie = RpkiTrie::default();
        trie.insert_roas(roas);
        Ok(trie)
    }
}

fn load_vrp_from_rpkiviews(date: NaiveDate) -> Result<Vec<RoaEntry>> {
    let day_link = format!(
        "{}/{:04}/{:02}/{:02}",
        DATA_ROOT_URL,
        date.year(),
        date.month(),
        date.day(),
    );
    let response = reqwest::blocking::get(day_link.as_str())?.text()?;

    let mut link_date_vec = extract_file_names(&response)
        .into_iter()
        .map(|name| {
            let timestamp = parse_file_name_timestamp(name).unwrap();
            let link = format!(
                "{}/{:04}/{:02}/{:02}/{}",
                DATA_ROOT_URL,
                date.year(),
                date.month(),
                date.day(),
                name
            );
            (link, timestamp)
        })
        .collect::<Vec<(String, NaiveDateTime)>>();

    link_date_vec.sort_by(|(_, time1), (_, time2)| time1.cmp(time2));
    let link = link_date_vec.first().unwrap().0.to_owned();

    let reader = oneio::get_reader(link.as_str()).unwrap();
    let mut ar = tar::Archive::new(reader);

    let mut roas: Vec<RoaEntry> = vec![];

    for entry in ar.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        if path.ends_with("rpki-client.json") {
            let res: RpkiViewsData = serde_json::from_reader(entry).unwrap();

            roas = res
                .roas
                .into_iter()
                .map(|roa| RoaEntry {
                    prefix: roa.prefix.parse().unwrap(),
                    asn: roa.asn,
                    max_length: roa.max_length,
                    rir: Some(Rir::from_str(roa.ta.as_str()).unwrap()),
                    not_before: None,
                    not_after: None,
                })
                .collect::<Vec<RoaEntry>>();
            break;
        }
    }

    match roas.len() {
        0 => Err(anyhow::anyhow!("no roas found")),
        _ => Ok(roas),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpki::RpkiValidation;

    #[test]
    fn test_rpkiviews_historical() {
        let date = NaiveDate::from_ymd_opt(2023, 8, 4).unwrap();
        let trie = RpkiTrie::from_rpkiviews_historical(date).unwrap();
        let prefix = IpNet::from_str("1.1.1.0/24").unwrap();
        assert_eq!(trie.validate(&prefix, 13335), RpkiValidation::Valid);
    }
}
