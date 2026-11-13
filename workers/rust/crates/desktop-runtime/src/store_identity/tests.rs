//! Real TLS + RSA fixtures. Unix tests generate an ephemeral test key with OpenSSL;
//! no private key, token, account or network outside loopback is used or persisted.
use super::*;
use ring::signature::{KeyPair, RSA_PKCS1_SHA256, RsaKeyPair};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

mod downloads;

struct Signing {
    key: RsaKeyPair,
    jwks: Value,
}
impl Signing {
    fn new() -> Self {
        let output = std::process::Command::new("openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-outform",
                "DER",
            ])
            .output()
            .expect("OpenSSL is required for the Unix OIDC integration fixture");
        assert!(output.status.success());
        let der = Zeroizing::new(output.stdout);
        let key = RsaKeyPair::from_pkcs8(&der)
            .or_else(|_| RsaKeyPair::from_der(&der))
            .unwrap();
        fn item<'a>(bytes: &mut &'a [u8], tag: u8) -> &'a [u8] {
            assert_eq!(bytes[0], tag);
            let mut len = bytes[1] as usize;
            *bytes = &bytes[2..];
            if len & 128 != 0 {
                let count = len & 127;
                len = 0;
                for byte in &bytes[..count] {
                    len = (len << 8) | *byte as usize;
                }
                *bytes = &bytes[count..];
            }
            let result = &bytes[..len];
            *bytes = &bytes[len..];
            result
        }
        let mut der = key.public_key().as_ref();
        let mut sequence = item(&mut der, 0x30);
        let n = item(&mut sequence, 2);
        let n = n.strip_prefix(&[0]).unwrap_or(n);
        let e = item(&mut sequence, 2);
        let jwks = json!({"keys":[{"kid":"test-only","kty":"RSA","alg":"RS256","use":"sig",
            "n":URL_SAFE_NO_PAD.encode(n),"e":URL_SAFE_NO_PAD.encode(e)}]});
        Self { key, jwks }
    }
    fn sign(&self, claims: &Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","kid":"test-only"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
        let content = format!("{header}.{payload}");
        let mut signature = vec![0; self.key.public().modulus_len()];
        self.key
            .sign(
                &RSA_PKCS1_SHA256,
                &SystemRandom::new(),
                content.as_bytes(),
                &mut signature,
            )
            .unwrap();
        format!("{content}.{}", URL_SAFE_NO_PAD.encode(signature))
    }
}

struct Fixture {
    config: IdentityConfig,
    nonce: Arc<Mutex<String>>,
    calls: Arc<Mutex<Vec<(String, String)>>>,
    drop_refresh: Arc<AtomicBool>,
    reuse_access: Arc<AtomicBool>,
    store: Arc<Mutex<downloads::StoreFixture>>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        Self::with_native_files(None)
    }
    fn with_native_files(files: Option<Vec<(String, Vec<u8>)>>) -> Self {
        let signing = Signing::new();
        let certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate.cert.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der())
                .into(),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let callback = TcpListener::bind("127.0.0.1:0").unwrap();
        let config = IdentityConfig {
            issuer: format!(
                "https://127.0.0.1:{}",
                listener.local_addr().unwrap().port()
            ),
            client_id: "registered-test-client".into(),
            callback_port: callback.local_addr().unwrap().port(),
            ca_pem: certificate.cert.pem(),
            ca_sha256: format!("{:x}", Sha256::digest(certificate.cert.der())),
        };
        drop(callback);
        let issuer = config.issuer.clone();
        let client_id = config.client_id.clone();
        let nonce = Arc::new(Mutex::new(String::new()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let drop_refresh = Arc::new(AtomicBool::new(false));
        let reuse_access = Arc::new(AtomicBool::new(false));
        let reuse = reuse_access.clone();
        let mut store_fixture = downloads::StoreFixture::new(&signing);
        if let Some(files) = files {
            store_fixture.native_files(&signing, files);
        }
        let store = Arc::new(Mutex::new(store_fixture));
        let server_store = store.clone();
        let (n, c, s, d) = (
            nonce.clone(),
            calls.clone(),
            stop.clone(),
            drop_refresh.clone(),
        );
        let tls = Arc::new(tls);
        let handle = thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                let Ok((socket, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut stream = rustls::StreamOwned::new(
                    rustls::ServerConnection::new(tls.clone()).unwrap(),
                    socket,
                );
                let mut data = Vec::new();
                let mut byte = [0];
                while !data.ends_with(b"\r\n\r\n") && data.len() < 16384 {
                    if stream.read_exact(&mut byte).is_err() {
                        break;
                    }
                    data.push(byte[0]);
                }
                if !data.ends_with(b"\r\n\r\n") {
                    continue;
                }
                let headers = String::from_utf8(data).unwrap();
                let path = headers.split_whitespace().nth(1).unwrap().to_owned();
                let len = headers
                    .lines()
                    .filter_map(|l| l.split_once(':'))
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    .map(|(_, v)| v.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                let mut body = vec![0; len];
                stream.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                c.lock().unwrap().push((path.clone(), body.clone()));
                let form: HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes())
                    .into_owned()
                    .collect();
                if path.starts_with("/store/") {
                    let result = server_store
                        .lock()
                        .unwrap()
                        .serve_bytes(&path, &body, &headers, &issuer);
                    if let Some((status, bytes)) = result {
                        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
                        stream.write_all(&bytes).unwrap();
                        stream.conn.send_close_notify();
                        let _ = stream.flush();
                    }
                    continue;
                }
                let status = 200;
                let response = match path.as_str() {
                    "/.well-known/openid-configuration" => json!({"issuer":issuer,
                        "authorization_endpoint":format!("{issuer}/account"),"token_endpoint":format!("{issuer}/v1/oidc/token"),
                        "userinfo_endpoint":format!("{issuer}/v1/oidc/userinfo"),"jwks_uri":format!("{issuer}/v1/oidc/jwks"),
                        "revocation_endpoint":format!("{issuer}/v1/oidc/revoke"),
                        "code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none"],
                        "id_token_signing_alg_values_supported":["RS256"],"response_types_supported":["code"],
                        "authorization_response_iss_parameter_supported":true}),
                    "/v1/oidc/jwks" => signing.jwks.clone(),
                    "/v1/oidc/userinfo" => json!({"sub":"researcher","name":"Test Researcher"}),
                    "/v1/oidc/revoke" => json!({}),
                    "/v1/oidc/token" => {
                        let refresh = form.get("grant_type").unwrap() == "refresh_token";
                        if refresh && d.load(Ordering::SeqCst) {
                            continue;
                        }
                        let mut claims = json!({"iss":issuer,"aud":client_id,"sub":"researcher","iat":now(),"exp":now()+600});
                        if !refresh {
                            claims["nonce"] = json!(*n.lock().unwrap());
                        }
                        json!({"token_type":"Bearer","expires_in":600,"access_token":if refresh && !reuse.load(Ordering::SeqCst) {"access-new"} else {"access-old"},
                            "refresh_token":if refresh {"refresh-new"} else {"refresh-old"},"id_token":signing.sign(&claims),
                            "scope":"openid profile offline_access"})
                    }
                    _ => panic!("unexpected endpoint"),
                };
                let json = serde_json::to_vec(&response).unwrap();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", json.len()).unwrap();
                stream.write_all(&json).unwrap();
                stream.conn.send_close_notify();
                let _ = stream.flush();
            }
        });
        Self {
            config,
            nonce,
            calls,
            drop_refresh,
            reuse_access,
            store,
            stop,
            handle: Some(handle),
        }
    }
    fn begin(&self, manager: &StoreIdentity) -> url::Url {
        let mut authorization = None;
        manager
            .begin_with_browser(self.config.clone(), |target| {
                let url = url::Url::parse(target).unwrap();
                *self.nonce.lock().unwrap() = url
                    .query_pairs()
                    .find(|(k, _)| k == "nonce")
                    .unwrap()
                    .1
                    .to_string();
                authorization = Some(url);
                Ok(())
            })
            .unwrap();
        authorization.unwrap()
    }
    fn callback(&self, url: &url::Url) {
        let state = url
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .to_string();
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("state", state.as_str()),
                ("iss", &self.config.issuer),
                ("code", "one-time-code"),
            ])
            .finish();
        let mut stream = TcpStream::connect(("127.0.0.1", self.config.callback_port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        write!(
            stream,
            "GET {CALLBACK_PATH}?{query} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
            self.config.callback_port
        )
        .unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        assert!(result.starts_with("HTTP/1.1 200"));
        assert!(!result.contains(&state) && !result.contains("one-time-code"));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.handle.take().unwrap().join().unwrap();
    }
}
fn wait_for(manager: &StoreIdentity, status: &str) {
    let until = Instant::now() + Duration::from_secs(5);
    while manager.status().status != status || manager.0.lock().unwrap().in_flight {
        assert!(
            Instant::now() < until,
            "expected {status}, received {}",
            manager.status().status
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn real_tls_pkce_callback_rotation_redaction_and_revocation() {
    let fixture = Fixture::new();
    let manager = StoreIdentity::default();
    let auth = fixture.begin(&manager);
    fixture.callback(&auth);
    wait_for(&manager, "signed_in");
    let query: HashMap<_, _> = auth.query_pairs().into_owned().collect();
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["redirect_uri"], fixture.config.redirect_uri());
    let body = fixture
        .calls
        .lock()
        .unwrap()
        .iter()
        .find(|(path, _)| path == "/v1/oidc/token")
        .unwrap()
        .1
        .clone();
    let form: HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(
        query["code_challenge"],
        URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()))
    );
    assert!(!form.contains_key("client_secret") && !form.contains_key("scope"));
    assert_eq!(manager.status().subject.as_deref(), Some("researcher"));
    let view = serde_json::to_string(&manager.status()).unwrap();
    for secret in [
        "access-old",
        "refresh-old",
        &query["nonce"],
        &form["code_verifier"],
    ] {
        assert!(!view.contains(secret));
    }
    manager.refresh().unwrap();
    assert_eq!(
        manager
            .0
            .lock()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .access
            .as_str(),
        "access-new"
    );
    assert_eq!(manager.logout().status, "signed_out");
    let requests = fixture.calls.lock().unwrap();
    assert!(
        requests
            .iter()
            .any(|(p, b)| p == "/v1/oidc/revoke" && b.contains("refresh-new"))
    );
    assert!(
        requests
            .iter()
            .any(|(p, b)| p == "/v1/oidc/revoke" && b.contains("access-new"))
    );
}

#[test]
fn refresh_transport_loss_is_never_replayed_and_forces_login() {
    let fixture = Fixture::new();
    let manager = StoreIdentity::default();
    let url = fixture.begin(&manager);
    fixture.callback(&url);
    wait_for(&manager, "signed_in");
    fixture.drop_refresh.store(true, Ordering::SeqCst);
    assert!(manager.refresh().is_err());
    assert_eq!(manager.status().status, "error");
    assert!(manager.refresh().is_err());
    assert_eq!(
        fixture
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, b)| p == "/v1/oidc/token" && b.contains("grant_type=refresh_token"))
            .count(),
        1
    );
}

#[test]
fn cancelled_login_closes_listener_and_disallows_late_session_commit() {
    let fixture = Fixture::new();
    let manager = StoreIdentity::default();
    fixture.begin(&manager);
    manager.cancel();
    wait_for(&manager, "signed_out");
    assert!(TcpStream::connect(("127.0.0.1", fixture.config.callback_port)).is_err());
    let provider = protocol::Provider::discover(fixture.config.clone()).unwrap();
    *fixture.nonce.lock().unwrap() = "nonce".into();
    let session = provider
        .exchange("code", "verifier", Zeroizing::new("nonce".into()))
        .unwrap();
    let old = manager.0.lock().unwrap().epoch;
    manager.logout();
    manager.accept(old, session);
    assert_eq!(manager.status().status, "signed_out");
    assert!(
        fixture
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(path, _)| path == "/v1/oidc/revoke")
    );
}

#[test]
fn id_token_signature_claims_and_nonce_are_mandatory() {
    let signer = Signing::new();
    let config: IdentityConfig =
        serde_json::from_value(json!({"issuer":"https://id.test","client_id":"client"})).unwrap();
    let keys = serde_json::from_value(signer.jwks.clone()).unwrap();
    let claims = json!({"iss":config.issuer,"aud":"client","sub":"user","iat":now(),"exp":now()+600,"nonce":"nonce"});
    assert!(
        jwt::verify(
            &signer.sign(&claims),
            &keys,
            &config,
            "nonce",
            false,
            "access"
        )
        .is_ok()
    );
    for (field, value) in [
        ("iss", json!("https://evil.test")),
        ("aud", json!("other-client")),
        ("nonce", json!("other-nonce")),
        ("exp", json!(now() - 1)),
        ("iat", json!(now() + 100)),
        ("sub", json!("")),
        ("azp", json!("other")),
        ("at_hash", json!("bad")),
    ] {
        let mut invalid = claims.clone();
        invalid[field] = value;
        assert!(
            jwt::verify(
                &signer.sign(&invalid),
                &keys,
                &config,
                "nonce",
                false,
                "access"
            )
            .is_err(),
            "{field}"
        );
    }
    let mut no_nonce = claims.clone();
    no_nonce.as_object_mut().unwrap().remove("nonce");
    assert!(
        jwt::verify(
            &signer.sign(&no_nonce),
            &keys,
            &config,
            "nonce",
            false,
            "access"
        )
        .is_err()
    );
    assert!(
        jwt::verify(
            &signer.sign(&no_nonce),
            &keys,
            &config,
            "nonce",
            true,
            "access"
        )
        .is_ok()
    );
    let mut token = signer.sign(&claims);
    token.push('a');
    assert!(jwt::verify(&token, &keys, &config, "nonce", false, "access").is_err());
    let mut duplicate = signer.jwks.clone();
    duplicate["keys"]
        .as_array_mut()
        .unwrap()
        .push(signer.jwks["keys"][0].clone());
    assert!(
        jwt::verify(
            &signer.sign(&claims),
            &serde_json::from_value(duplicate).unwrap(),
            &config,
            "nonce",
            false,
            "access"
        )
        .is_err()
    );
}

#[test]
fn trust_mismatch_and_busy_callback_fail_without_browser_open() {
    let fixture = Fixture::new();
    let manager = StoreIdentity::default();
    let mut wrong = fixture.config.clone();
    wrong.ca_sha256 = "0".repeat(64);
    assert!(
        manager
            .begin_with_browser(wrong, |_| panic!("untrusted destination opened"))
            .is_err()
    );
    let _occupied = TcpListener::bind(("127.0.0.1", fixture.config.callback_port)).unwrap();
    assert_eq!(
        manager
            .begin_with_browser(fixture.config.clone(), |_| panic!(
                "occupied callback opened"
            ))
            .err(),
        Some("callback_port_unavailable")
    );
}
