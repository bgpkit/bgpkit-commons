//! TLS negotiation for the loader's PostgreSQL connections.
//!
//! The connection string controls encryption the same way libpq's `sslmode`
//! does, so a deployment can move between plaintext-over-the-overlay and TLS
//! without a rebuild:
//!
//! | `sslmode` | behavior |
//! |---|---|
//! | `disable` | never negotiate TLS |
//! | `prefer` (default) | use TLS when the server offers it, plaintext otherwise; certificates are not verified |
//! | `require` | TLS mandatory; certificates are not verified unless `sslrootcert` is given, in which case the chain is verified |
//! | `verify-ca` | TLS mandatory; chain verified against `sslrootcert` or the system roots; hostname not checked |
//! | `verify-full` | TLS mandatory; chain and hostname verified |
//!
//! `sslmode=allow` is rejected: it needs a plaintext-first retry that
//! tokio-postgres does not expose, and `prefer` covers the "encrypt when
//! possible" case. `prefer` and `require` protect against passive observers,
//! not against an active man in the middle; use the `verify-*` modes when the
//! server certificate must be authenticated.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, Error as RustlsError, RootCertStore,
    SignatureScheme,
};
use tokio_postgres::{Client, NoTls};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::info;

const SSLMODE: &str = "sslmode";
const SSLROOTCERT: &str = "sslrootcert";

/// Non-TLS modes need no connector, so the table is exhaustive here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl SslMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "disable" => Ok(Self::Disable),
            "prefer" => Ok(Self::Prefer),
            "require" => Ok(Self::Require),
            "verify-ca" => Ok(Self::VerifyCa),
            "verify-full" => Ok(Self::VerifyFull),
            "allow" => Err("sslmode=allow is not supported; use sslmode=prefer".to_string()),
            other => Err(format!(
                "unsupported sslmode '{other}' (expected disable, prefer, require, verify-ca, or verify-full)"
            )),
        }
    }

    /// Value handed to tokio-postgres, which only implements these three.
    /// Every verification mode requires TLS, so it maps onto `require`; the
    /// certificate policy is enforced by the connector's verifier.
    fn negotiation(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require | Self::VerifyCa | Self::VerifyFull => "require",
        }
    }

    fn verifies_chain(self, has_root_cert: bool) -> bool {
        match self {
            Self::VerifyCa | Self::VerifyFull => true,
            Self::Require => has_root_cert,
            Self::Disable | Self::Prefer => false,
        }
    }

    fn verifies_hostname(self) -> bool {
        matches!(self, Self::VerifyFull)
    }
}

/// Connection string with the TLS policy extracted, plus the negotiation value
/// tokio-postgres understands.
#[derive(Debug)]
pub struct ConnectionSettings {
    pub url: String,
    pub mode: SslMode,
    pub root_cert: Option<PathBuf>,
}

impl ConnectionSettings {
    pub fn parse(connection_string: &str) -> Result<Self, String> {
        let (rest, mode_value, root_cert) = if connection_string.contains("://") {
            split_url_form(connection_string)?
        } else {
            split_keyword_form(connection_string)?
        };
        let mode = match mode_value.as_deref() {
            // libpq's default: encrypt when the server offers it.
            None => SslMode::Prefer,
            Some(value) => SslMode::parse(value)?,
        };
        let url = if connection_string.contains("://") {
            append_url_param(&rest, SSLMODE, mode.negotiation())
        } else {
            format!("{rest} {SSLMODE}={}", mode.negotiation())
        };
        Ok(Self {
            url,
            mode,
            root_cert: root_cert.map(PathBuf::from),
        })
    }
}

/// Split `postgresql://host/db?sslmode=x&other=y`, returning the string without
/// the TLS parameters.
fn split_url_form(input: &str) -> Result<(String, Option<String>, Option<String>), String> {
    let (base, query) = match input.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (input, None),
    };
    let mut kept: Vec<String> = Vec::new();
    let mut mode = None;
    let mut root_cert = None;
    if let Some(query) = query {
        for item in query.split('&').filter(|item| !item.is_empty()) {
            let (key, value) = item.split_once('=').unwrap_or((item, ""));
            match key {
                SSLMODE => mode = Some(percent_decode(value)),
                SSLROOTCERT => root_cert = Some(percent_decode(value)),
                _ => kept.push(item.to_string()),
            }
        }
    }
    let rest = if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept.join("&"))
    };
    Ok((rest, mode, root_cert))
}

fn append_url_param(url: &str, key: &str, value: &str) -> String {
    if url.contains('?') {
        format!("{url}&{key}={value}")
    } else {
        format!("{url}?{key}={value}")
    }
}

/// Values in a URL carry percent escapes; the ones we read are mode names and
/// paths, so only `%2F`-style escapes in a path matter.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(byte) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Split libpq's keyword form (`host=x sslmode=y`), honouring quoted values.
fn split_keyword_form(input: &str) -> Result<(String, Option<String>, Option<String>), String> {
    let mut kept: Vec<String> = Vec::new();
    let mut mode = None;
    let mut root_cert = None;
    let mut raw = String::new();
    let mut value = String::new();
    let mut quote: Option<char> = None;

    let mut finish = |raw: &mut String, value: &mut String| {
        if raw.is_empty() {
            return;
        }
        match value.split_once('=') {
            Some((key, val)) if key == SSLMODE => mode = Some(val.to_string()),
            Some((key, val)) if key == SSLROOTCERT => root_cert = Some(val.to_string()),
            _ => kept.push(std::mem::take(raw)),
        }
        raw.clear();
        value.clear();
    };

    for ch in input.chars() {
        match quote {
            Some(q) if ch == q => {
                quote = None;
                value.push(ch);
                raw.push(ch);
            }
            Some(_) => {
                value.push(ch);
                raw.push(ch);
            }
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                value.push(ch);
                raw.push(ch);
            }
            None if ch.is_whitespace() => finish(&mut raw, &mut value),
            None => {
                value.push(ch);
                raw.push(ch);
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated quoted value in connection string".to_string());
    }
    finish(&mut raw, &mut value);
    Ok((kept.join(" "), mode, root_cert))
}

/// Accepts whatever certificate the server presents. This is libpq's behavior
/// for `sslmode=prefer` and for `require` without a root certificate: the link
/// is encrypted but the peer is not authenticated.
#[derive(Debug)]
struct AcceptAnyServerCert {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Verifies the chain against the trust anchors but not the hostname, which is
/// libpq's `verify-ca`. rustls reports the name mismatch as an ordinary
/// certificate error, and only after the chain has been validated, so mapping
/// that one variant onto success yields chain-only verification.
#[derive(Debug)]
struct ChainOnlyVerifier {
    inner: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for ChainOnlyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        match self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Err(RustlsError::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

fn load_roots(root_cert: Option<&Path>) -> Result<RootCertStore, String> {
    let mut roots = RootCertStore::empty();
    match root_cert {
        Some(path) => {
            let pem = std::fs::read(path)
                .map_err(|e| format!("cannot read sslrootcert {}: {e}", path.display()))?;
            let mut added = 0;
            for cert in rustls_pemfile::certs(&mut pem.as_slice()) {
                let cert = cert.map_err(|e| format!("invalid certificate in sslrootcert: {e}"))?;
                roots
                    .add(cert)
                    .map_err(|e| format!("cannot add sslrootcert certificate: {e}"))?;
                added += 1;
            }
            if added == 0 {
                return Err(format!(
                    "sslrootcert {} contains no certificates",
                    path.display()
                ));
            }
        }
        None => {
            let result = rustls_native_certs::load_native_certs();
            for error in &result.errors {
                info!("ignoring a native certificate store error: {error}");
            }
            for cert in result.certs {
                let _ = roots.add(cert);
            }
            if roots.is_empty() {
                return Err("no trust anchors available; set sslrootcert".to_string());
            }
        }
    }
    Ok(roots)
}

fn build_verifier(
    settings: &ConnectionSettings,
    provider: &Arc<CryptoProvider>,
) -> Result<Arc<dyn ServerCertVerifier>, String> {
    if !settings.mode.verifies_chain(settings.root_cert.is_some()) {
        return Ok(Arc::new(AcceptAnyServerCert {
            provider: provider.clone(),
        }));
    }
    let roots = Arc::new(load_roots(settings.root_cert.as_deref())?);
    let webpki = WebPkiServerVerifier::builder_with_provider(roots, provider.clone())
        .build()
        .map_err(|e| format!("cannot build certificate verifier: {e}"))?;
    if settings.mode.verifies_hostname() {
        Ok(webpki)
    } else {
        Ok(Arc::new(ChainOnlyVerifier { inner: webpki }))
    }
}

/// Connect with the TLS policy from the connection string. `prefer` keeps
/// working against a server without TLS: tokio-postgres requests TLS and falls
/// back to plaintext when the server declines.
pub async fn connect(settings: &ConnectionSettings) -> Result<Client, String> {
    info!(
        "connecting to PostgreSQL (sslmode={}) ...",
        settings.mode_name()
    );
    match settings.mode {
        SslMode::Disable => drive(tokio_postgres::connect(&settings.url, NoTls).await).await,
        _ => {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let verifier = build_verifier(settings, &provider)?;
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|e| format!("TLS configuration error: {e}"))?
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth();
            let tls = MakeRustlsConnect::new(config);
            drive(tokio_postgres::connect(&settings.url, tls).await).await
        }
    }
}

impl ConnectionSettings {
    fn mode_name(&self) -> &'static str {
        match self.mode {
            SslMode::Disable => "disable",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCa => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        }
    }
}

async fn drive<S>(
    result: Result<
        (
            Client,
            tokio_postgres::Connection<tokio_postgres::Socket, S>,
        ),
        tokio_postgres::Error,
    >,
) -> Result<Client, String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (client, connection) =
        result.map_err(|e| format!("failed to connect to PostgreSQL: {}", error_chain(&e)))?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("postgres connection error: {e}");
        }
    });
    Ok(client)
}

/// tokio-postgres renders every TLS problem as "error performing TLS
/// handshake"; the cause (`server does not support TLS`, a certificate error, a
/// missing hostname) is only in the source chain, so append it.
fn error_chain(error: &tokio_postgres::Error) -> String {
    let mut rendered = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        rendered.push_str(": ");
        rendered.push_str(&cause.to_string());
        source = cause.source();
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tokio_postgres::Config;

    fn parse(url: &str) -> ConnectionSettings {
        ConnectionSettings::parse(url).expect("connection string should parse")
    }

    #[test]
    fn defaults_to_prefer() {
        let settings = parse("postgresql://loader@db.example:5432/bgpkit_broker");
        assert_eq!(settings.mode, SslMode::Prefer);
        assert!(settings.root_cert.is_none());
        assert_eq!(
            settings.url,
            "postgresql://loader@db.example:5432/bgpkit_broker?sslmode=prefer"
        );
    }

    #[test]
    fn keeps_other_url_parameters() {
        let settings = parse(
            "postgresql://loader@db.example:5432/bgpkit_broker?application_name=loader&sslmode=require",
        );
        assert_eq!(settings.mode, SslMode::Require);
        assert_eq!(
            settings.url,
            "postgresql://loader@db.example:5432/bgpkit_broker?application_name=loader&sslmode=require"
        );
    }

    #[test]
    fn verification_modes_negotiate_as_require() {
        for (value, expected) in [
            ("verify-ca", SslMode::VerifyCa),
            ("verify-full", SslMode::VerifyFull),
        ] {
            let settings = parse(&format!(
                "postgresql://loader@db.example:5432/db?sslmode={value}&sslrootcert=%2Fetc%2Fca.crt"
            ));
            assert_eq!(settings.mode, expected);
            assert_eq!(
                settings.root_cert.as_deref(),
                Some(Path::new("/etc/ca.crt"))
            );
            assert!(settings.url.ends_with("sslmode=require"));
            assert!(!settings.url.contains("sslrootcert"));
            // tokio-postgres rejects modes it does not know, so the rewritten
            // string must survive its parser.
            Config::from_str(&settings.url).expect("tokio-postgres must accept the rewritten URL");
        }
    }

    #[test]
    fn disable_is_passed_through() {
        let settings = parse("postgresql://loader@db.example:5432/db?sslmode=disable");
        assert_eq!(settings.mode, SslMode::Disable);
        assert_eq!(
            settings.url,
            "postgresql://loader@db.example:5432/db?sslmode=disable"
        );
    }

    #[test]
    fn keyword_form_is_supported() {
        let settings = parse(
            "host=db.example port=5432 user=loader password='p w' dbname=broker sslmode=verify-full sslrootcert=/etc/ca.crt",
        );
        assert_eq!(settings.mode, SslMode::VerifyFull);
        assert_eq!(
            settings.root_cert.as_deref(),
            Some(Path::new("/etc/ca.crt"))
        );
        assert!(settings.url.contains("password='p w'"));
        assert!(!settings.url.contains("sslrootcert"));
        assert!(settings.url.ends_with("sslmode=require"));
        Config::from_str(&settings.url).expect("tokio-postgres must accept the rewritten string");
    }

    #[test]
    fn allow_is_rejected_with_a_hint() {
        let error = ConnectionSettings::parse("postgresql://loader@db/db?sslmode=allow")
            .expect_err("allow must be rejected");
        assert!(error.contains("prefer"), "unexpected error: {error}");
    }

    #[test]
    fn unknown_mode_is_rejected() {
        let error = ConnectionSettings::parse("postgresql://loader@db/db?sslmode=sorta")
            .expect_err("unknown mode must be rejected");
        assert!(
            error.contains("unsupported sslmode"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn certificate_policy_per_mode() {
        assert!(!SslMode::Prefer.verifies_chain(true));
        assert!(!SslMode::Prefer.verifies_hostname());
        assert!(!SslMode::Require.verifies_chain(false));
        assert!(SslMode::Require.verifies_chain(true));
        assert!(!SslMode::Require.verifies_hostname());
        assert!(SslMode::VerifyCa.verifies_chain(false));
        assert!(!SslMode::VerifyCa.verifies_hostname());
        assert!(SslMode::VerifyFull.verifies_chain(false));
        assert!(SslMode::VerifyFull.verifies_hostname());
    }

    #[test]
    fn missing_root_certificate_is_reported() {
        let settings =
            parse("postgresql://loader@db/db?sslmode=verify-ca&sslrootcert=/nope/ca.crt");
        let error = load_roots(settings.root_cert.as_deref()).expect_err("missing file must fail");
        assert!(
            error.contains("cannot read sslrootcert"),
            "unexpected error: {error}"
        );
    }
}
