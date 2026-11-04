use crate::Connection;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

const SCHEMA: &str = "kyuubiki.agent-auth/v1";
const ALPN: &[u8] = b"kyuubiki-agent/1";
const MAX_PEM: usize = 64 * 1024;

#[derive(Clone)]
pub struct ApiToken(Zeroizing<String>);

impl std::fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiToken([REDACTED])")
    }
}

impl ApiToken {
    pub fn new(value: String) -> io::Result<Self> {
        let value = Zeroizing::new(value);
        if !(32..=256).contains(&value.len())
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(invalid(
                "Agent API token must be 32..256 URL-safe characters",
            ));
        }
        Ok(Self(value))
    }
}

#[derive(Clone)]
pub struct ClientSecurity {
    config: Arc<rustls::ClientConfig>,
    name: ServerName<'static>,
    token: ApiToken,
}

impl std::fmt::Debug for ClientSecurity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClientSecurity(TLS, [REDACTED])")
    }
}

impl ClientSecurity {
    pub fn from_pem(ca_pem: &str, server_name: &str, token: String) -> io::Result<Self> {
        let token = ApiToken::new(token)?;
        let mut roots = rustls::RootCertStore::empty();
        for certificate in certificates(ca_pem)? {
            roots
                .add(certificate)
                .map_err(|_| invalid("invalid Agent trust certificate"))?;
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| invalid("TLS configuration failed"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![ALPN.to_vec()];
        // No TLS early data or token/session resumption across credential rotation.
        config.resumption = rustls::client::Resumption::disabled();
        let name = ServerName::try_from(server_name.to_owned())
            .map_err(|_| invalid("invalid Agent certificate server name"))?;
        Ok(Self {
            config: Arc::new(config),
            name,
            token,
        })
    }

    pub fn connect(&self, socket: TcpStream, timeout: Duration) -> io::Result<Connection> {
        let tls = rustls::ClientConnection::new(self.config.clone(), self.name.clone())
            .map_err(|_| invalid("TLS client unavailable"))?;
        let mut stream = Connection::handshake(socket, rustls::Connection::Client(tls), timeout)?;
        write_frame(
            &mut stream,
            &Auth {
                schema_version: SCHEMA,
                api_token: &self.token.0,
            },
        )?;
        let reply = read_frame(&mut stream)?;
        let reply: AuthReply = serde_json::from_slice(&reply).map_err(|_| denied())?;
        if reply.schema_version != SCHEMA || !reply.authenticated {
            return Err(denied());
        }
        Ok(stream)
    }
}

pub struct ServerSecurity {
    config: Arc<rustls::ServerConfig>,
    token_digest: [u8; 32],
}

impl ServerSecurity {
    pub fn from_pem(certificate_pem: &str, key_pem: &str, token: String) -> io::Result<Self> {
        let token = ApiToken::new(token)?;
        if key_pem.len() > MAX_PEM {
            return Err(invalid("Agent TLS key exceeds size limit"));
        }
        let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes())
            .map_err(|_| invalid("invalid Agent TLS private key"))?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| invalid("TLS configuration failed"))?
            .with_no_client_auth()
            .with_single_cert(certificates(certificate_pem)?, key)
            .map_err(|_| invalid("Agent certificate and private key do not match"))?;
        config.alpn_protocols = vec![ALPN.to_vec()];
        Ok(Self {
            config: Arc::new(config),
            token_digest: Sha256::digest(token.0.as_bytes()).into(),
        })
    }

    pub fn accept(&self, socket: TcpStream, timeout: Duration) -> io::Result<Connection> {
        let tls = rustls::ServerConnection::new(self.config.clone())
            .map_err(|_| invalid("TLS server unavailable"))?;
        let mut stream = Connection::handshake(socket, rustls::Connection::Server(tls), timeout)?;
        let bytes = read_frame(&mut stream)?;
        let auth: OwnedAuth = serde_json::from_slice(&bytes).map_err(|_| denied())?;
        let token = ApiToken::new(auth.api_token).map_err(|_| denied())?;
        let digest: [u8; 32] = Sha256::digest(token.0.as_bytes()).into();
        if auth.schema_version != SCHEMA || !bool::from(self.token_digest.ct_eq(&digest)) {
            return Err(denied());
        }
        write_frame(
            &mut stream,
            &AuthReply {
                schema_version: SCHEMA.to_owned(),
                authenticated: true,
            },
        )?;
        Ok(stream)
    }
}

#[derive(Serialize)]
struct Auth<'a> {
    schema_version: &'a str,
    api_token: &'a str,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedAuth {
    schema_version: String,
    api_token: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthReply {
    schema_version: String,
    authenticated: bool,
}

fn certificates(pem: &str) -> io::Result<Vec<CertificateDer<'static>>> {
    if pem.len() > MAX_PEM {
        return Err(invalid("Agent certificate exceeds size limit"));
    }
    let certificates = CertificateDer::pem_slice_iter(pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("invalid Agent certificate PEM"))?;
    if certificates.is_empty() {
        return Err(invalid("Agent trust certificate is required"));
    }
    Ok(certificates)
}

fn read_frame(stream: &mut Connection) -> io::Result<Zeroizing<Vec<u8>>> {
    let mut header = [0; 4];
    stream.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if !(1..=1024).contains(&length) {
        return Err(denied());
    }
    let mut bytes = Zeroizing::new(vec![0; length]);
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn write_frame(stream: &mut Connection, value: &impl Serialize) -> io::Result<()> {
    let bytes = Zeroizing::new(
        serde_json::to_vec(value).map_err(|_| invalid("Agent authentication encoding failed"))?,
    );
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Agent authentication failed; no task dispatched",
    )
}
