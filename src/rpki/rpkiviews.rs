//! Load RPKI data from RPKIviews collectors.
//!
//! RPKIviews provides historical RPKI data from multiple collectors around the world.
//! See: <https://rpkiviews.org/>

use crate::Result;
use crate::rpki::rpki_client::RpkiClientData;
use crate::rpki::{RpkiFile, RpkiTrie};
use chrono::{DateTime, Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::str::FromStr;
use tracing::info;

/// Available RPKIviews collectors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpkiViewsCollector {
    /// josephine.sobornost.net - A2B Internet (AS51088), Amsterdam, Netherlands
    #[default]
    SoborostNet,
    /// amber.massars.net - Massar (AS57777), Lugano, Switzerland
    MassarsNet,
    /// dango.attn.jp - Internet Initiative Japan (AS2497), Tokyo, Japan
    AttnJp,
    /// rpkiviews.kerfuffle.net - Kerfuffle, LLC (AS35008), Fremont, California, United States
    KerfuffleNet,
}

impl RpkiViewsCollector {
    /// Get the HTTPS base URL for this collector
    pub fn base_url(&self) -> &'static str {
        match self {
            RpkiViewsCollector::SoborostNet => "https://josephine.sobornost.net/rpkidata",
            RpkiViewsCollector::MassarsNet => "https://amber.massars.net/rpkidata",
            RpkiViewsCollector::AttnJp => "https://dango.attn.jp/rpkidata",
            RpkiViewsCollector::KerfuffleNet => "https://rpkiviews.kerfuffle.net/rpkidata",
        }
    }

    /// Get the index.txt URL for this collector
    pub fn index_url(&self) -> String {
        format!("{}/index.txt", self.base_url())
    }

    /// Get all available collectors
    pub fn all() -> Vec<RpkiViewsCollector> {
        vec![
            RpkiViewsCollector::SoborostNet,
            RpkiViewsCollector::MassarsNet,
            RpkiViewsCollector::AttnJp,
            RpkiViewsCollector::KerfuffleNet,
        ]
    }
}

impl std::fmt::Display for RpkiViewsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpkiViewsCollector::SoborostNet => write!(f, "sobornost.net"),
            RpkiViewsCollector::MassarsNet => write!(f, "massars.net"),
            RpkiViewsCollector::AttnJp => write!(f, "attn.jp"),
            RpkiViewsCollector::KerfuffleNet => write!(f, "kerfuffle.net"),
        }
    }
}

impl FromStr for RpkiViewsCollector {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sobornost.net" | "josephine.sobornost.net" => Ok(RpkiViewsCollector::SoborostNet),
            "massars.net" | "amber.massars.net" => Ok(RpkiViewsCollector::MassarsNet),
            "attn.jp" | "dango.attn.jp" => Ok(RpkiViewsCollector::AttnJp),
            "kerfuffle.net" | "rpkiviews.kerfuffle.net" => Ok(RpkiViewsCollector::KerfuffleNet),
            _ => Err(format!("unknown RPKIviews collector: {}", s)),
        }
    }
}

/// List available RPKIviews files for a given date from a specific collector.
///
/// This function reads the index.txt file from the collector to find available
/// archives for the specified date. This is a fast operation as it only downloads
/// a small index file.
pub fn list_rpkiviews_files(
    collector: RpkiViewsCollector,
    date: NaiveDate,
) -> Result<Vec<RpkiFile>> {
    let index_url = collector.index_url();
    let base_url = collector.base_url();

    // Format the date path prefix we're looking for (e.g., "2024/01/04/")
    let date_prefix = format!("{:04}/{:02}/{:02}/", date.year(), date.month(), date.day());

    let mut files = vec![];

    for line in oneio::read_lines(&index_url)? {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let path = parts[0];
        let timestamp_secs: i64 = match parts[1].parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let size: u64 = match parts[2].parse() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Check if this file is for the requested date and is a .tgz file
        if path.starts_with(&date_prefix) && path.ends_with(".tgz") && path.contains("/rpki-") {
            let url = format!("{}/{}", base_url, path);
            let timestamp = DateTime::from_timestamp(timestamp_secs, 0)
                .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());

            files.push(RpkiFile {
                url,
                timestamp,
                size: Some(size),
                rir: None,
                collector: Some(collector),
            });
        }
    }

    // Sort by timestamp (oldest first)
    files.sort_by_key(|f| f.timestamp);

    Ok(files)
}

/// Information about a file entry found within a .tgz archive.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TgzFileEntry {
    /// Path of the file within the archive
    pub path: String,
    /// Size of the file in bytes
    pub size: u64,
}

/// List files within a remote .tgz archive by streaming only the tar headers.
#[allow(dead_code)]
///
/// This function streams the .tgz archive and reads tar headers to enumerate
/// the files contained within. It skips reading the actual file content, which
/// makes it much faster than downloading the entire archive.
///
/// **Important**: Due to the nature of gzip compression, we still need to decompress
/// the data sequentially, but we skip over file content rather than buffering it.
/// If `max_entries` is provided, we stop early after finding that many entries.
///
/// # Arguments
/// * `url` - URL of the .tgz file
/// * `max_entries` - Optional maximum number of entries to return (for early termination)
pub fn list_files_in_tgz(url: &str, max_entries: Option<usize>) -> Result<Vec<TgzFileEntry>> {
    info!("listing files in tgz archive: {}", url);

    // Spawn gunzip process
    let mut gunzip = Command::new("gunzip")
        .arg("-c")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to spawn gunzip: {}", e),
            )
        })?;

    let mut gunzip_stdin = gunzip.stdin.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdin")
    })?;

    let gunzip_stdout = gunzip.stdout.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdout")
    })?;

    // Flag to signal early termination to writer thread
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_writer = should_stop.clone();

    // Stream the .tgz file using reqwest in a separate thread
    let url_owned = url.to_string();
    let writer_thread = std::thread::spawn(move || -> Result<()> {
        let response = reqwest::blocking::get(&url_owned).map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to fetch {}: {}", url_owned, e),
            )
        })?;

        if !response.status().is_success() {
            return Err(crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("HTTP error {} for {}", response.status(), url_owned),
            ));
        }

        let mut reader = response;
        let mut buffer = [0u8; 65536];
        loop {
            // Check if we should stop early
            if should_stop_writer.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let n = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break, // Connection closed or error, stop gracefully
            };

            if gunzip_stdin.write_all(&buffer[..n]).is_err() {
                // Pipe closed (reader side done), stop gracefully
                break;
            }
        }
        drop(gunzip_stdin);
        Ok(())
    });

    // Read tar entries from gunzip stdout - only reading headers, skipping content
    let mut archive = tar::Archive::new(gunzip_stdout);
    let mut entries_list = Vec::new();

    let entries_iter = archive.entries().map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKIviews",
            format!("Failed to read tar entries: {}", e),
        )
    })?;

    for entry_result in entries_iter {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = match entry.path() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        let size = entry.size();

        // Skip directories (they have size 0 and path ends with /)
        if !path.ends_with('/') {
            entries_list.push(TgzFileEntry { path, size });
        }

        // Check if we've reached max_entries
        if let Some(max) = max_entries {
            if entries_list.len() >= max {
                // Signal writer to stop and break out
                should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
                break;
            }
        }
    }

    // Signal writer to stop (in case we finished iterating)
    should_stop.store(true, std::sync::atomic::Ordering::Relaxed);

    // Don't wait for writer thread - it will terminate when pipe closes
    // Just detach it
    drop(writer_thread);

    // Kill gunzip process to clean up
    let _ = gunzip.kill();
    let _ = gunzip.wait();

    Ok(entries_list)
}

/// Check if a .tgz archive contains a specific file path.
#[allow(dead_code)]
///
/// This is an optimized function that stops as soon as it finds the target file,
/// avoiding the need to decompress the entire archive.
pub fn tgz_contains_file(url: &str, target_path: &str) -> Result<bool> {
    info!(
        "checking if tgz archive contains file: {} in {}",
        target_path, url
    );

    let mut gunzip = Command::new("gunzip")
        .arg("-c")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to spawn gunzip: {}", e),
            )
        })?;

    let mut gunzip_stdin = gunzip.stdin.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdin")
    })?;

    let gunzip_stdout = gunzip.stdout.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdout")
    })?;

    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_writer = should_stop.clone();

    let url_owned = url.to_string();
    let writer_thread = std::thread::spawn(move || -> Result<()> {
        let response = reqwest::blocking::get(&url_owned).map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to fetch {}: {}", url_owned, e),
            )
        })?;

        if !response.status().is_success() {
            return Err(crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("HTTP error {} for {}", response.status(), url_owned),
            ));
        }

        let mut reader = response;
        let mut buffer = [0u8; 65536];
        loop {
            if should_stop_writer.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let n = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };

            if gunzip_stdin.write_all(&buffer[..n]).is_err() {
                break;
            }
        }
        drop(gunzip_stdin);
        Ok(())
    });

    let mut archive = tar::Archive::new(gunzip_stdout);
    let mut found = false;

    if let Ok(entries_iter) = archive.entries() {
        for entry_result in entries_iter {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = match entry.path() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            if path.ends_with(target_path) || path == target_path {
                found = true;
                should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
                break;
            }
        }
    }

    should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    drop(writer_thread);
    let _ = gunzip.kill();
    let _ = gunzip.wait();

    Ok(found)
}

/// Extract a specific file from a remote .tgz archive.
///
/// This function streams the .tgz archive and extracts only the target file,
/// stopping as soon as the file is found and read. This is much faster than
/// downloading and extracting the entire archive.
///
/// # Arguments
/// * `url` - URL of the .tgz file
/// * `target_path` - Path of the file to extract within the archive (e.g., "output/rpki-client.json")
///
/// # Returns
/// The content of the target file as a String, or an error if the file is not found.
pub fn extract_file_from_tgz(url: &str, target_path: &str) -> Result<String> {
    info!("extracting {} from tgz archive: {}", target_path, url);

    let mut gunzip = Command::new("gunzip")
        .arg("-c")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to spawn gunzip: {}", e),
            )
        })?;

    let mut gunzip_stdin = gunzip.stdin.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdin")
    })?;

    let gunzip_stdout = gunzip.stdout.take().ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error("RPKIviews", "Failed to get gunzip stdout")
    })?;

    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_writer = should_stop.clone();

    let url_owned = url.to_string();
    let writer_thread = std::thread::spawn(move || -> Result<()> {
        let response = reqwest::blocking::get(&url_owned).map_err(|e| {
            crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("Failed to fetch {}: {}", url_owned, e),
            )
        })?;

        if !response.status().is_success() {
            return Err(crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!("HTTP error {} for {}", response.status(), url_owned),
            ));
        }

        let mut reader = response;
        let mut buffer = [0u8; 65536];
        loop {
            if should_stop_writer.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let n = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };

            if gunzip_stdin.write_all(&buffer[..n]).is_err() {
                break;
            }
        }
        drop(gunzip_stdin);
        Ok(())
    });

    let mut archive = tar::Archive::new(gunzip_stdout);
    let mut content: Option<String> = None;

    let entries_iter = archive.entries().map_err(|e| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKIviews",
            format!("Failed to read tar entries: {}", e),
        )
    })?;

    for entry_result in entries_iter {
        let mut entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = match entry.path() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Check if this is the target file
        if path.ends_with(target_path) || path == target_path {
            let mut file_content = String::new();
            entry.read_to_string(&mut file_content).map_err(|e| {
                crate::BgpkitCommonsError::data_source_error(
                    "RPKIviews",
                    format!("Failed to read {}: {}", target_path, e),
                )
            })?;
            content = Some(file_content);
            should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
            break;
        }
        // Note: if this is not our target file, the tar iterator automatically
        // skips past the file content when we move to the next entry,
        // so we don't buffer unnecessary data
    }

    should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    drop(writer_thread);
    let _ = gunzip.kill();
    let _ = gunzip.wait();

    content.ok_or_else(|| {
        crate::BgpkitCommonsError::data_source_error(
            "RPKIviews",
            format!("{} not found in archive: {}", target_path, url),
        )
    })
}

/// Stream and extract rpki-client.json from a .tgz URL.
///
/// This is a convenience function that extracts the rpki-client.json file
/// from an RPKIviews archive. It stops streaming as soon as the file is found.
fn stream_tgz_and_extract_json(url: &str) -> Result<RpkiClientData> {
    let json_str = extract_file_from_tgz(url, "output/rpki-client.json")?;
    RpkiClientData::from_json(&json_str)
}

impl RpkiTrie {
    /// Load RPKI data from RPKIviews for a specific date.
    ///
    /// This will use the first (earliest) available file for the given date from the specified collector.
    /// By default, uses the Kerfuffle collector.
    pub fn from_rpkiviews(collector: RpkiViewsCollector, date: NaiveDate) -> Result<Self> {
        let files = list_rpkiviews_files(collector, date)?;

        if files.is_empty() {
            return Err(crate::BgpkitCommonsError::data_source_error(
                "RPKIviews",
                format!(
                    "No RPKIviews files found for date {} from collector {}",
                    date, collector
                ),
            ));
        }

        // Use the first (earliest) file for the date
        let first_file = files.first().unwrap();
        info!(
            "Using RPKIviews file from {} (timestamp: {})",
            collector, first_file.timestamp
        );

        Self::from_rpkiviews_file(&first_file.url, Some(date))
    }

    /// Load RPKI data from a specific RPKIviews .tgz file URL.
    pub fn from_rpkiviews_file(url: &str, date: Option<NaiveDate>) -> Result<Self> {
        let data = stream_tgz_and_extract_json(url)?;
        Self::from_rpki_client_data(data, date)
    }

    /// Load RPKI data from multiple RPKIviews file URLs.
    ///
    /// This allows loading and merging data from multiple files into a single trie.
    pub fn from_rpkiviews_files(urls: &[String], date: Option<NaiveDate>) -> Result<Self> {
        let mut trie = RpkiTrie::new(date);

        for url in urls {
            let data = stream_tgz_and_extract_json(url)?;
            trie.merge_rpki_client_data(data);
        }

        Ok(trie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_urls() {
        assert_eq!(
            RpkiViewsCollector::SoborostNet.base_url(),
            "https://josephine.sobornost.net/rpkidata"
        );
        assert_eq!(
            RpkiViewsCollector::KerfuffleNet.index_url(),
            "https://rpkiviews.kerfuffle.net/rpkidata/index.txt"
        );
    }

    #[test]
    fn test_collector_from_str() {
        assert_eq!(
            RpkiViewsCollector::from_str("sobornost.net").unwrap(),
            RpkiViewsCollector::SoborostNet
        );
        assert_eq!(
            RpkiViewsCollector::from_str("amber.massars.net").unwrap(),
            RpkiViewsCollector::MassarsNet
        );
    }

    #[test]
    fn test_default_collector() {
        assert_eq!(
            RpkiViewsCollector::default(),
            RpkiViewsCollector::SoborostNet
        );
    }

    #[test]
    #[ignore] // Requires network access
    fn test_list_rpkiviews_files() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
        let files = list_rpkiviews_files(RpkiViewsCollector::KerfuffleNet, date).unwrap();
        println!("Found {} files for {}", files.len(), date);
        for file in &files {
            println!("  {} ({} bytes)", file.url, file.size.unwrap_or(0));
        }
        assert!(!files.is_empty());
    }

    #[test]
    #[ignore] // Requires network access - streams partial archive
    fn test_list_files_in_tgz() {
        // List first 10 files in a remote tgz to verify streaming works
        let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
        let files = list_rpkiviews_files(RpkiViewsCollector::KerfuffleNet, date).unwrap();
        assert!(!files.is_empty());

        let tgz_url = &files[0].url;
        println!("Listing files in: {}", tgz_url);

        // Only get first 10 entries to test early termination
        let entries = list_files_in_tgz(tgz_url, Some(10)).unwrap();
        println!("Found {} entries (limited to 10):", entries.len());
        for entry in &entries {
            println!("  {} ({} bytes)", entry.path, entry.size);
        }
        assert!(!entries.is_empty());
        assert!(entries.len() <= 10);
    }

    #[test]
    #[ignore] // Requires network access - streams partial archive
    fn test_tgz_contains_file() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
        let files = list_rpkiviews_files(RpkiViewsCollector::KerfuffleNet, date).unwrap();
        assert!(!files.is_empty());

        let tgz_url = &files[0].url;
        println!("Checking for rpki-client.json in: {}", tgz_url);

        let contains = tgz_contains_file(tgz_url, "output/rpki-client.json").unwrap();
        assert!(contains, "Archive should contain output/rpki-client.json");
        println!("Found rpki-client.json!");
    }

    #[test]
    #[ignore] // Requires network access and takes time to download
    fn test_from_rpkiviews() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
        let trie = RpkiTrie::from_rpkiviews(RpkiViewsCollector::KerfuffleNet, date).unwrap();

        let total_roas: usize = trie.trie.iter().map(|(_, roas)| roas.len()).sum();
        println!("Loaded {} ROAs from RPKIviews", total_roas);
        assert!(total_roas > 0);
    }
}
