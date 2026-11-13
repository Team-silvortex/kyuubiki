#![allow(dead_code)]
use kyuubiki_agent_transport::{ClientSecurity, Connection, SocketIo};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

pub const SERVER_NAME: &str = "kyuubiki-native-test.invalid";
pub const TEST_TOKEN: &str = "test-only-cross-host-token-0123456789abcdef";

pub fn endpoint(value: &str, mode: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| "explicit_numeric_endpoint_required")?;
    if !matches!(mode, "ssh-loopback" | "native-tls")
        || address.port() == 0
        || address.ip().is_unspecified()
        || address.ip().is_multicast()
        || (mode == "ssh-loopback" && !address.ip().is_loopback())
    {
        return Err("explicit_loopback_tunnel_or_native_tls_required".into());
    }
    Ok(address)
}

#[derive(Clone)]
pub struct Transport {
    pub endpoint: SocketAddr,
    pub security: Option<ClientSecurity>,
}

impl Transport {
    pub fn from_env() -> Result<Self, String> {
        let get = |name: &str| {
            std::env::var(name).map_err(|_| "missing_cross_host_configuration".to_owned())
        };
        let address = get("KYUUBIKI_CROSS_HOST_ENDPOINT")?;
        let mode = std::env::var("KYUUBIKI_CROSS_HOST_TRANSPORT")
            .unwrap_or_else(|_| "ssh-loopback".into());
        let optional = |name: &str| std::env::var(name).ok();
        let ca = optional("KYUUBIKI_CROSS_HOST_CA_FILE")
            .map(|p| read_material(Path::new(&p), false))
            .transpose()?;
        let token = optional("KYUUBIKI_CROSS_HOST_TOKEN_FILE")
            .map(|p| read_material(Path::new(&p), true))
            .transpose()?;
        let name = optional("KYUUBIKI_CROSS_HOST_SERVER_NAME");
        Self::configured(&address, &mode, ca.as_deref(), name.as_deref(), token)
    }

    pub fn assert_denials(&self, pem: &str, server_name: &str, token: &str) {
        for security in [
            Some(ClientSecurity::from_pem(pem, server_name, TEST_TOKEN.into()).unwrap()),
            Some(ClientSecurity::from_pem(pem, "wrong-name.invalid", token.into()).unwrap()),
            Some(
                ClientSecurity::from_pem(
                    &rcgen::generate_simple_self_signed(vec![SERVER_NAME.into()])
                        .unwrap()
                        .cert
                        .pem(),
                    server_name,
                    token.into(),
                )
                .unwrap(),
            ),
            None,
        ] {
            self.request("describe_agent", json!({}))
                .expect("valid control connection must remain available");
            let encrypted = security.is_some();
            let denied = Self {
                endpoint: self.endpoint,
                security,
            };
            let error = denied.request("describe_agent", json!({})).unwrap_err();
            if encrypted {
                assert_eq!(error, "cross_host_authentication_failed");
            } else {
                assert!(matches!(
                    error.as_str(),
                    "cross_host_read_failed" | "cross_host_write_failed"
                ));
            }
        }
        self.request("describe_agent", json!({}))
            .expect("denials must not poison valid control");
    }
    pub fn configured(
        address: &str,
        mode: &str,
        pem: Option<&str>,
        name: Option<&str>,
        token: Option<String>,
    ) -> Result<Self, String> {
        let endpoint = endpoint(address, mode)?;
        let security = match (mode, pem, name, token) {
            ("ssh-loopback", None, None, None) => None,
            ("native-tls", Some(pem), Some(name), Some(token)) => {
                Some(ClientSecurity::from_pem(pem, name, token).map_err(|e| e.to_string())?)
            }
            _ => return Err("explicit_complete_transport_configuration_required".into()),
        };
        Ok(Self { endpoint, security })
    }

    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        self.request_named("cross-host-probe", method, params)
    }

    pub fn request_named(&self, id: &str, method: &str, params: Value) -> Result<Value, String> {
        let socket = TcpStream::connect_timeout(&self.endpoint, Duration::from_secs(5))
            .map_err(|_| "cross_host_connect_failed")?;
        let mut stream: Connection = match &self.security {
            Some(security) => security
                .connect(socket, Duration::from_secs(5))
                .map_err(|_| "cross_host_authentication_failed")?,
            None => socket.into(),
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| e.to_string())?;
        let bytes =
            serde_json::to_vec(&json!({"rpc_version":1,"id":id,"method":method,"params":params}))
                .unwrap();
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .map_err(|_| "cross_host_write_failed")?;
        stream
            .write_all(&bytes)
            .map_err(|_| "cross_host_write_failed")?;
        for _ in 0..32 {
            let mut header = [0; 4];
            stream
                .read_exact(&mut header)
                .map_err(|_| "cross_host_read_failed")?;
            let len = u32::from_be_bytes(header) as usize;
            if len == 0 || len > 16 * 1024 * 1024 {
                return Err("invalid_rpc_frame".into());
            }
            let mut bytes = vec![0; len];
            stream
                .read_exact(&mut bytes)
                .map_err(|_| "cross_host_read_failed")?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_rpc_json")?;
            if value.get("ok").is_some() {
                if value["id"] != id || value["ok"] != true {
                    return Err(format!("RPC rejected: {}", value["error"]["code"]));
                }
                return Ok(value["result"].clone());
            }
        }
        Err("rpc_event_limit".into())
    }
}

pub fn read_material(path: &Path, private: bool) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "missing_test_material")?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Err("bounded_regular_test_material_required".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if private && (metadata.mode() & 0o077 != 0 || metadata.uid() != unsafe { libc::geteuid() })
        {
            return Err("private_owned_test_material_required".into());
        }
    }
    #[cfg(not(unix))]
    let _ = private;
    fs::read_to_string(path).map_err(|_| "invalid_test_material".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_qualification_requires_explicit_complete_tls_without_plaintext_fallback() {
        let pem = rcgen::generate_simple_self_signed(vec![SERVER_NAME.into()])
            .unwrap()
            .cert
            .pem();
        assert!(
            Transport::configured(
                "192.0.2.1:5001",
                "native-tls",
                Some(&pem),
                Some(SERVER_NAME),
                Some(TEST_TOKEN.into())
            )
            .is_ok()
        );
        for mode in ["ssh-loopback", "", "auto", "tls"] {
            assert!(Transport::configured("192.0.2.1:5001", mode, None, None, None).is_err());
        }
        for address in [
            "0.0.0.0:5001",
            "192.0.2.1:0",
            "224.0.0.1:5001",
            "localhost:5001",
        ] {
            assert!(endpoint(address, "native-tls").is_err());
        }
        assert!(Transport::configured("127.0.0.1:5001", "native-tls", None, None, None).is_err());
        assert!(
            Transport::configured(
                "127.0.0.1:5001",
                "ssh-loopback",
                Some(&pem),
                Some(SERVER_NAME),
                Some(TEST_TOKEN.into())
            )
            .is_err()
        );
        assert!(Transport::configured("127.0.0.1:5001", "ssh-loopback", None, None, None).is_ok());
    }
}
