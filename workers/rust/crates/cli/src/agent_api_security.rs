use kyuubiki_agent_transport::{Connection, ServerSecurity};
use kyuubiki_protocol::AgentConnectionAccess;
use std::fs::OpenOptions;
use std::io::Read;
use std::net::TcpStream;
use std::sync::OnceLock;
use std::time::Duration;
use zeroize::Zeroizing;

static SECURITY: OnceLock<Option<ServerSecurity>> = OnceLock::new();
const TOKEN: &str = "KYUUBIKI_AGENT_API_TOKEN";
const CERT: &str = "KYUUBIKI_AGENT_TLS_CERT_FILE";
const KEY: &str = "KYUUBIKI_AGENT_TLS_KEY_FILE";

pub(crate) fn enabled() -> bool {
    SECURITY.get().is_some_and(Option::is_some)
}

pub(crate) fn configure() -> Result<(), String> {
    let read = |name: &str| {
        std::env::var(name).map(Some).or_else(|error| match error {
            std::env::VarError::NotPresent => Ok(None),
            _ => Err(format!("{name} must be valid Unicode")),
        })
    };
    let config = match (read(TOKEN)?, read(CERT)?, read(KEY)?) {
        (None, None, None) => None,
        (Some(token), Some(certificate), Some(key)) => Some(ServerSecurity::from_pem(
            &read_pem(&certificate, false)?, &read_pem(&key, true)?, token,
        ).map_err(|error| error.to_string())?),
        _ => return Err("Agent API mode requires token, TLS certificate and private key together; no plaintext fallback".into()),
    };
    SECURITY
        .set(config)
        .map_err(|_| "Agent API security already configured".into())
}

pub(crate) fn accept(socket: TcpStream) -> Result<Connection, String> {
    match SECURITY.get().and_then(Option::as_ref) {
        Some(security) => security
            .accept(socket, Duration::from_secs(10))
            .map_err(|_| "Agent TLS/authentication rejected; no request dispatched".into()),
        None => Ok(socket.into()),
    }
}

pub(crate) fn mark_connection_access(reply: &mut crate::transport::AgentReply) {
    let crate::transport::AgentReply::Stream(_, response) = reply;
    if let Some(result) = response
        .result
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
        && let Some(value) = result.get_mut("connection_access")
        && let Ok(access) = serde_json::from_value::<AgentConnectionAccess>(value.clone())
    {
        *value =
            serde_json::to_value(access.with_tls_token()).expect("connection access serializes");
    }
}

fn read_pem(path: &str, private: bool) -> Result<Zeroizing<String>, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| "cannot open Agent TLS material")?;
    let metadata = file
        .metadata()
        .map_err(|_| "cannot inspect Agent TLS material")?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Err("Agent TLS material must be a bounded regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if private && (metadata.mode() & 0o077 != 0 || metadata.uid() != unsafe { libc::geteuid() })
        {
            return Err("Agent TLS private key must be owned by the current user with no group/other permissions".into());
        }
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut pem = Zeroizing::new(String::new());
    file.take(64 * 1024 + 1)
        .read_to_string(&mut pem)
        .map_err(|_| "cannot read Agent TLS material")?;
    if pem.len() > 64 * 1024 {
        return Err("Agent TLS material exceeds size limit".into());
    }
    Ok(pem)
}
