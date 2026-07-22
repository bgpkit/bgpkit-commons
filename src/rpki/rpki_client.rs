//! Internal data structures for parsing rpki-client JSON output format.
//!
//! The `rpki-client` software produces JSON output that is used by multiple RPKI data sources:
//! - Cloudflare RPKI Portal (<https://rpki.cloudflare.com/rpki.json>)
//! - RIPE NCC historical archives (output.json.xz files)
//! - RPKIviews collectors (rpki-client.json inside .tgz files)
//!
//! This module defines the internal data structures for parsing this JSON format.
//! For public access, use the `Roa` and `Aspa` structs from the parent module.

use serde::{Deserialize, Deserializer, Serialize};

/// Custom deserializer for ASN that handles both numeric and string formats.
/// RIPE uses "AS12345" format, while Cloudflare uses numeric 12345.
fn deserialize_asn<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct AsnVisitor;

    impl<'de> Visitor<'de> for AsnVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an ASN as a number or string like 'AS12345'")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| E::custom(format!("ASN {} out of range", value)))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| E::custom(format!("ASN {} out of range", value)))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // Handle "AS12345" or "as12345" format
            let num_str = value
                .strip_prefix("AS")
                .or_else(|| value.strip_prefix("as"))
                .unwrap_or(value);

            num_str
                .parse::<u32>()
                .map_err(|_| E::custom(format!("invalid ASN string: {}", value)))
        }
    }

    deserializer.deserialize_any(AsnVisitor)
}

/// Custom deserializer for expires that handles both i64 and u64.
fn deserialize_expires<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct ExpiresVisitor;

    impl<'de> Visitor<'de> for ExpiresVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a timestamp as a number")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= 0 {
                Ok(value as u64)
            } else {
                Err(E::custom(format!("negative timestamp: {}", value)))
            }
        }
    }

    deserializer.deserialize_any(ExpiresVisitor)
}

/// Custom deserializer for provider list that handles both string array and number array.
/// RIPE uses ["AS123", "AS456"] format, Cloudflare uses [123, 456].
fn deserialize_providers<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};

    struct ProvidersVisitor;

    impl<'de> Visitor<'de> for ProvidersVisitor {
        type Value = Vec<u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a list of ASNs as numbers or strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut providers = Vec::new();

            while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                let asn = match elem {
                    serde_json::Value::Number(n) => n
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| de::Error::custom("invalid ASN number"))?,
                    serde_json::Value::String(s) => {
                        let num_str = s
                            .strip_prefix("AS")
                            .or_else(|| s.strip_prefix("as"))
                            .unwrap_or(&s);
                        num_str
                            .parse::<u32>()
                            .map_err(|_| de::Error::custom(format!("invalid ASN string: {}", s)))?
                    }
                    _ => return Err(de::Error::custom("expected number or string")),
                };
                providers.push(asn);
            }

            Ok(providers)
        }
    }

    deserializer.deserialize_seq(ProvidersVisitor)
}

/// The main rpki-client JSON output structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct RpkiClientData {
    #[serde(default)]
    pub metadata: RpkiClientMetadata,
    #[serde(default)]
    pub roas: Vec<RpkiClientRoaEntry>,
    #[serde(default)]
    pub aspas: Vec<RpkiClientAspaEntry>,
    #[serde(default)]
    pub bgpsec_keys: Vec<RpkiClientBgpsecKeyEntry>,
}

/// Metadata about the rpki-client validation run.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub(crate) struct RpkiClientMetadata {
    pub buildmachine: Option<String>,
    pub buildtime: Option<String>,
    #[serde(default)]
    pub generated: Option<u64>,
    #[serde(rename = "generatedTime", default)]
    pub generated_time: Option<String>,
    pub elapsedtime: Option<u32>,
    pub usertime: Option<u32>,
    pub systemtime: Option<u32>,
    pub roas: Option<u32>,
    pub failedroas: Option<u32>,
    pub invalidroas: Option<u32>,
    pub spls: Option<u32>,
    pub failedspls: Option<u32>,
    pub invalidspls: Option<u32>,
    pub aspas: Option<u32>,
    pub failedaspas: Option<u32>,
    pub invalidaspas: Option<u32>,
    pub bgpsec_pubkeys: Option<u32>,
    pub certificates: Option<u32>,
    pub invalidcertificates: Option<u32>,
    pub taks: Option<u32>,
    pub tals: Option<u32>,
    pub invalidtals: Option<u32>,
    pub talfiles: Option<Vec<String>>,
    pub manifests: Option<u32>,
    pub failedmanifests: Option<u32>,
    pub crls: Option<u32>,
    pub gbrs: Option<u32>,
    pub repositories: Option<u32>,
    pub vrps: Option<u32>,
    pub uniquevrps: Option<u32>,
    pub vsps: Option<u32>,
    pub uniquevsps: Option<u32>,
    pub vaps: Option<u32>,
    pub uniquevaps: Option<u32>,
    pub cachedir_new_files: Option<u32>,
    pub cachedir_del_files: Option<u32>,
    pub cachedir_del_dirs: Option<u32>,
    pub cachedir_superfluous_files: Option<u32>,
    pub cachedir_del_superfluous_files: Option<u32>,
}

/// A validated Route Origin Authorization (ROA) entry from rpki-client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RpkiClientRoaEntry {
    pub prefix: String,
    #[serde(rename = "maxLength")]
    pub max_length: u8,
    #[serde(deserialize_with = "deserialize_asn")]
    pub asn: u32,
    pub ta: String,
    #[serde(default, deserialize_with = "deserialize_expires")]
    pub expires: u64,
}

/// A validated AS Provider Authorization (ASPA) entry from rpki-client.
///
/// Handles both Cloudflare format (customer_asid as number, providers as numbers)
/// and RIPE format (customer as string "AS123", providers as strings ["AS456"]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RpkiClientAspaEntry {
    /// Customer ASN - Cloudflare uses "customer_asid", RIPE uses "customer"
    #[serde(alias = "customer", deserialize_with = "deserialize_asn")]
    pub customer_asid: u32,
    /// Expiry timestamp - may be missing in RIPE format
    #[serde(default)]
    pub expires: i64,
    /// Provider ASNs - can be numbers or strings like "AS123"
    #[serde(deserialize_with = "deserialize_providers")]
    pub providers: Vec<u32>,
}

/// A validated BGPsec router key entry from rpki-client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RpkiClientBgpsecKeyEntry {
    pub asn: u32,
    pub ski: String,
    pub pubkey: String,
    pub ta: String,
    pub expires: i64,
}

/// Result of a successful (non-304) conditional fetch of rpki-client JSON data.
#[derive(Debug)]
pub(crate) struct RpkiClientFetch {
    /// The parsed rpki-client data
    pub data: RpkiClientData,
    /// Value of the response `ETag` header, if present
    pub etag: Option<String>,
    /// Value of the response `Last-Modified` header, if present
    pub last_modified: Option<String>,
}

impl RpkiClientData {
    /// Load rpki-client data from a URL.
    ///
    /// This uses oneio to handle remote URLs and compression (xz, gz, etc.),
    /// then parses the JSON with our custom deserializers.
    pub fn from_url(url: &str) -> crate::Result<Self> {
        let reader = oneio::get_reader(url)?;
        let data: RpkiClientData = serde_json::from_reader(reader)?;
        Ok(data)
    }

    /// Conditionally load rpki-client data from a URL using HTTP validators.
    ///
    /// Sends `If-None-Match` (when `etag` is given) and `If-Modified-Since`
    /// (when `last_modified` is given) request headers. Returns `Ok(None)` when
    /// the server responds with `304 Not Modified`, meaning the caller's cached
    /// data is still current and no re-download/re-parse is needed.
    ///
    /// The request advertises `Accept-Encoding: gzip` and the response body is
    /// transparently decompressed, which significantly reduces transfer size
    /// for large JSON payloads (e.g. ~97 MB to ~4.6 MB for Cloudflare's
    /// `rpki.json`).
    ///
    /// On a `200 OK` response, returns the parsed data along with the response's
    /// `ETag` and `Last-Modified` validator values for use in subsequent
    /// conditional requests.
    pub fn from_url_conditional(
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> crate::Result<Option<RpkiClientFetch>> {
        let client = reqwest::blocking::Client::new();
        let mut request = client.get(url);
        if let Some(etag) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = last_modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }

        let response = request.send()?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        let response = response.error_for_status()?;

        let header_str = |name: reqwest::header::HeaderName| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        };
        let etag = header_str(reqwest::header::ETAG);
        let last_modified = header_str(reqwest::header::LAST_MODIFIED);
        let data: RpkiClientData = serde_json::from_reader(response)?;
        Ok(Some(RpkiClientFetch {
            data,
            etag,
            last_modified,
        }))
    }

    /// Load rpki-client data from a JSON string.
    pub fn from_json(json: &str) -> crate::Result<Self> {
        let data: RpkiClientData = serde_json::from_str(json)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a minimal HTTP/1.1 server that answers one canned response per
    /// incoming connection, in order. Returns the server URL and a handle that
    /// yields the raw request texts it received.
    fn mock_server(responses: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                loop {
                    let n = stream.read(&mut tmp).unwrap();
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                requests.push(String::from_utf8_lossy(&buf).to_string());
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (format!("http://{}", addr), handle)
    }

    fn json_response(body: &str, etag: Option<&str>) -> String {
        let etag_header = match etag {
            Some(e) => format!(
                "ETag: {}\r\nLast-Modified: Wed, 01 Jan 2025 00:00:00 GMT\r\n",
                e
            ),
            None => String::new(),
        };
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            etag_header,
            body.len(),
            body
        )
    }

    const TEST_JSON: &str = r#"{
        "roas": [
            {
                "prefix": "192.0.2.0/24",
                "maxLength": 24,
                "asn": 64496,
                "ta": "apnic",
                "expires": 1704067200
            }
        ]
    }"#;

    #[test]
    fn test_from_url_conditional_full_load() {
        let (url, server) = mock_server(vec![json_response(TEST_JSON, Some("\"v1\""))]);

        let fetch = RpkiClientData::from_url_conditional(&url, None, None)
            .unwrap()
            .expect("unconditional request should return data");

        assert_eq!(fetch.data.roas.len(), 1);
        assert_eq!(fetch.data.roas[0].asn, 64496);
        assert_eq!(fetch.etag.as_deref(), Some("\"v1\""));
        assert_eq!(
            fetch.last_modified.as_deref(),
            Some("Wed, 01 Jan 2025 00:00:00 GMT")
        );

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(!requests[0].to_lowercase().contains("if-none-match:"));
        assert!(!requests[0].to_lowercase().contains("if-modified-since:"));
    }

    #[test]
    fn test_from_url_conditional_not_modified() {
        let (url, server) = mock_server(vec![
            json_response(TEST_JSON, Some("\"v1\"")),
            "HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_string(),
        ]);

        let fetch = RpkiClientData::from_url_conditional(&url, None, None)
            .unwrap()
            .unwrap();
        let result =
            RpkiClientData::from_url_conditional(&url, fetch.etag.as_deref(), None).unwrap();
        assert!(result.is_none(), "304 should map to Ok(None)");

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[1].to_lowercase().contains("if-none-match: \"v1\""),
            "second request should carry If-None-Match, got: {}",
            requests[1]
        );
    }

    #[test]
    fn test_from_url_conditional_sends_last_modified() {
        let (url, server) = mock_server(vec![json_response(TEST_JSON, Some("\"v2\""))]);

        let fetch =
            RpkiClientData::from_url_conditional(&url, None, Some("Wed, 01 Jan 2025 00:00:00 GMT"))
                .unwrap()
                .unwrap();
        assert_eq!(fetch.etag.as_deref(), Some("\"v2\""));

        let requests = server.join().unwrap();
        assert!(
            requests[0]
                .to_lowercase()
                .contains("if-modified-since: wed, 01 jan 2025 00:00:00 gmt"),
            "request should carry If-Modified-Since, got: {}",
            requests[0]
        );
    }

    #[test]
    fn test_deserialize_empty() {
        let json = r#"{}"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert!(data.roas.is_empty());
        assert!(data.aspas.is_empty());
        assert!(data.bgpsec_keys.is_empty());
    }

    #[test]
    fn test_deserialize_roa_numeric_asn() {
        let json = r#"{
            "roas": [
                {
                    "prefix": "192.0.2.0/24",
                    "maxLength": 24,
                    "asn": 64496,
                    "ta": "apnic",
                    "expires": 1704067200
                }
            ]
        }"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert_eq!(data.roas.len(), 1);
        assert_eq!(data.roas[0].prefix, "192.0.2.0/24");
        assert_eq!(data.roas[0].max_length, 24);
        assert_eq!(data.roas[0].asn, 64496);
        assert_eq!(data.roas[0].ta, "apnic");
    }

    #[test]
    fn test_deserialize_roa_string_asn() {
        // RIPE format uses "AS12345" string format
        let json = r#"{
            "roas": [
                {
                    "prefix": "1.178.112.0/20",
                    "maxLength": 24,
                    "asn": "AS12975",
                    "ta": "ripencc"
                }
            ]
        }"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert_eq!(data.roas.len(), 1);
        assert_eq!(data.roas[0].prefix, "1.178.112.0/20");
        assert_eq!(data.roas[0].max_length, 24);
        assert_eq!(data.roas[0].asn, 12975);
        assert_eq!(data.roas[0].ta, "ripencc");
    }

    #[test]
    fn test_deserialize_roa_lowercase_asn() {
        let json = r#"{
            "roas": [
                {
                    "prefix": "10.0.0.0/8",
                    "maxLength": 8,
                    "asn": "as64496",
                    "ta": "arin"
                }
            ]
        }"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert_eq!(data.roas[0].asn, 64496);
    }

    #[test]
    fn test_deserialize_aspa() {
        let json = r#"{
            "aspas": [
                {
                    "customer_asid": 64496,
                    "expires": 1704067200,
                    "providers": [64497, 64498]
                }
            ]
        }"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert_eq!(data.aspas.len(), 1);
        assert_eq!(data.aspas[0].customer_asid, 64496);
        assert_eq!(data.aspas[0].providers, vec![64497, 64498]);
    }

    #[test]
    fn test_deserialize_ripe_metadata() {
        let json = r#"{
            "metadata": {
                "generated": 1717215759,
                "generatedTime": "2024-06-01T04:22:39Z"
            }
        }"#;
        let data: RpkiClientData = serde_json::from_str(json).unwrap();
        assert_eq!(data.metadata.generated, Some(1717215759));
        assert_eq!(
            data.metadata.generated_time,
            Some("2024-06-01T04:22:39Z".to_string())
        );
    }
}
