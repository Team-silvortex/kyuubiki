use super::*;
use crate::store_download::{DownloadRequest, DownloadSelection, DownloadView, StoreDownloads};
use base64::engine::general_purpose::STANDARD;
use std::path::PathBuf;
mod native_install;

const GRANT: &str = "12345678-1234-1234-1234-123456789abc";
const FILE: &str = "macos-aarch64/kyuubiki-operator.json";

pub(super) struct StoreFixture {
    selection: DownloadSelection,
    file: Value,
    roots: Value,
    payload: Value,
    keys: Vec<String>,
    charges: usize,
    renewals: usize,
    drop_grant: bool,
    corrupt: bool,
    foreign_url: bool,
    recalled: bool,
    delay: bool,
    unauthorized: bool,
    download_started: Arc<AtomicBool>,
    native: Vec<(Value, Vec<u8>)>,
    catalog_reply: Option<Value>,
}
impl StoreFixture {
    pub(super) fn new(signing: &Signing) -> Self {
        fn der(tag: u8, bytes: &[u8]) -> Vec<u8> {
            let mut out = vec![tag];
            if bytes.len() < 128 {
                out.push(bytes.len() as u8);
            } else {
                out.extend([0x82, (bytes.len() >> 8) as u8, bytes.len() as u8]);
            }
            out.extend(bytes);
            out
        }
        let mut bits = vec![0];
        bits.extend(signing.key.public_key().as_ref());
        let mut sequence = vec![
            0x30, 13, 6, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 13, 1, 1, 1, 5, 0,
        ];
        sequence.extend(der(3, &bits));
        let spki = der(0x30, &sequence);
        let pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            STANDARD.encode(&spki)
        );
        let payload = json!({"fixture":"transport-only, not an installable operator"});
        let bytes = serde_json::to_vec(&payload).unwrap();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let mut signature = vec![0; signing.key.public().modulus_len()];
        signing
            .key
            .sign(
                &RSA_PKCS1_SHA256,
                &SystemRandom::new(),
                hash.as_bytes(),
                &mut signature,
            )
            .unwrap();
        let selection = DownloadSelection {
            kind: "operator".into(),
            asset_id: "demo".into(),
            version: "1.0.0".into(),
            platform: "macos-aarch64".into(),
            key_id: "transport-test".into(),
            key_sha256: format!("{:x}", Sha256::digest(&spki)),
        };
        let file = json!({"path":FILE,"sha256":hash,"size_bytes":bytes.len(),"media_type":"application/json",
            "signature":{"algorithm":"RS256","scheme":"silvortex-sha256-rs256-v1","signed_payload":"sha256_ascii",
                "key_id":selection.key_id,"value":URL_SAFE_NO_PAD.encode(signature)}});
        let roots = json!({"scheme":"silvortex-sha256-rs256-v1","signature_payload":"ascii_lowercase_sha256",
            "native_package_verification_available":false,"keys":[{"key_id":selection.key_id,"algorithm":"RS256","public_key_pem":pem}]});
        Self {
            selection,
            file,
            roots,
            payload,
            keys: vec![],
            charges: 0,
            renewals: 0,
            drop_grant: false,
            corrupt: false,
            foreign_url: false,
            recalled: false,
            delay: false,
            unauthorized: false,
            download_started: Arc::new(AtomicBool::new(false)),
            native: vec![],
            catalog_reply: None,
        }
    }
    pub(super) fn native_files(&mut self, signing: &Signing, files: Vec<(String, Vec<u8>)>) {
        self.selection.asset_id = "operator.template.summary".into();
        self.selection.version = "0.1.0".into();
        self.selection.platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        for (path, bytes) in files {
            let hash = format!("{:x}", Sha256::digest(&bytes));
            let mut signature = vec![0; signing.key.public().modulus_len()];
            signing
                .key
                .sign(
                    &RSA_PKCS1_SHA256,
                    &SystemRandom::new(),
                    hash.as_bytes(),
                    &mut signature,
                )
                .unwrap();
            let mut metadata = self.file.clone();
            metadata["path"] = json!(path);
            metadata["sha256"] = json!(hash);
            metadata["size_bytes"] = json!(bytes.len());
            metadata["signature"]["value"] = json!(URL_SAFE_NO_PAD.encode(signature));
            self.native.push((metadata, bytes));
        }
    }
    pub(super) fn serve_bytes(
        &mut self,
        path: &str,
        body: &str,
        headers: &str,
        issuer: &str,
    ) -> Option<(u16, Vec<u8>)> {
        if !self.native.is_empty() && path.starts_with("/store/v1/downloads/") {
            assert!(!headers.to_ascii_lowercase().contains("authorization:"));
            let index: usize = path.split("ticket=file-").nth(1).unwrap().parse().unwrap();
            return Some((200, self.native[index].1.clone()));
        }
        self.serve(path, body, headers, issuer)
            .map(|(status, value)| (status, serde_json::to_vec(&value).unwrap()))
    }
    pub(super) fn serve(
        &mut self,
        path: &str,
        body: &str,
        headers: &str,
        issuer: &str,
    ) -> Option<(u16, Value)> {
        let header = |name: &str| {
            headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.trim().to_owned())
        };
        let authenticated =
            path.starts_with("/store/v1/me") || path.starts_with("/store/v1/download-grants");
        assert_eq!(
            header("authorization").is_some(),
            authenticated,
            "wrong credential boundary: {path}"
        );
        if authenticated {
            assert_eq!(header("authorization").unwrap(), "Bearer access-old");
        }
        if self.unauthorized && authenticated {
            return Some((
                401,
                json!({"error":{"code":"unauthorized","message":"never expose secret-ticket"}}),
            ));
        }
        let s = &self.selection;
        let result = match path {
            path if path.starts_with("/store/v1/assets?") => {
                self.catalog_reply.clone().unwrap_or_else(|| json!({"catalog_revision":"r1","items":[{"kind":"operator","asset_id":s.asset_id,"title":"Demo", "summary":"Transport fixture", "domain":"mechanical", "languages":["en"],"latest_version":s.version}],"next_cursor":null}))
            }
            "/store/v1/capabilities" => {
                json!({"api_version":"store/v1","identity":{"issuer":issuer,"mode":"oidc_authorization_code_pkce","public_client_profile":"kyuubiki_desktop"},
                "features":{"downloads":true,"native_package_format_verified":false},
                "policy":{"quota_event":"grant_issued","renewal_charges_quota":false,"metering_policy_confirmed":true,"idempotency_retention_seconds":2592000},
                "artifact_limits":{"maximum_file_bytes":67108864,"maximum_package_bytes":268435456,"maximum_files_per_platform":32}})
            }
            "/store/v1/me" => {
                json!({"issuer":issuer,"subject":"researcher","session_expires_at":now()+600,"access_token_expires_at":now()+600})
            }
            "/store/v1/me/entitlements" => {
                json!({"subject":"researcher","download_kinds":["operator"],"metered_kinds":["operator","workflow_template"],"block_reasons":[],
                "quota":{"limit":1,"used":self.charges,"remaining":1-self.charges,"unlimited":false,"reset_at":now()+3600}})
            }
            "/store/v1/trust-roots" => self.roots.clone(),
            path if path
                == format!(
                    "/store/v1/assets/operator/{}/releases/{}",
                    s.asset_id, s.version
                ) =>
            {
                json!({"kind":s.kind,"asset_id":s.asset_id,"version":s.version,
                "status":if self.recalled {"recalled"} else {"published"},"artifacts":[{"platform":s.platform,"files":if self.native.is_empty() {vec![self.file.clone()]} else {self.native.iter().map(|(m,_)|m.clone()).collect::<Vec<_>>()} }]})
            }
            path if path.starts_with("/store/v1/download-grants") => {
                if path.ends_with("/renew") {
                    assert!(body.is_empty());
                    self.renewals += 1;
                } else {
                    let key = header("idempotency-key").unwrap();
                    assert_eq!(
                        serde_json::from_str::<Value>(body).unwrap(),
                        json!({"kind":s.kind,"asset_id":s.asset_id,"version":s.version,"platform":s.platform,
                        "files":if self.native.is_empty() {vec![json!({"path":self.file["path"],"sha256":self.file["sha256"]})]} else {self.native.iter().map(|(m,_)|json!({"path":m["path"],"sha256":m["sha256"]})).collect::<Vec<_>>()}})
                    );
                    if self.keys.is_empty() {
                        self.charges += 1;
                    } else {
                        assert_eq!(self.keys[0], key);
                    }
                    self.keys.push(key);
                    if self.drop_grant {
                        self.drop_grant = false;
                        return None;
                    }
                }
                let origin = if self.foreign_url {
                    "https://untrusted.invalid"
                } else {
                    issuer
                };
                json!({"grant_id":GRANT,"kind":s.kind,"asset_id":s.asset_id,"version":s.version,"platform":s.platform,
                    "status":"active","created_at":now(),"renew_until":now()+600,"ticket_expires_at":now()+300,
                    "files":if self.native.is_empty() {vec![json!({"path":FILE,"sha256":self.file["sha256"],"size_bytes":self.file["size_bytes"],"ticket_expires_at":now()+300,
                        "download_url":format!("{origin}/store/v1/downloads/{GRANT}?ticket=secret-ticket")})]} else {self.native.iter().enumerate().map(|(i,(m,_))|json!({"path":m["path"],"sha256":m["sha256"],"size_bytes":m["size_bytes"],"ticket_expires_at":now()+300,"download_url":format!("{origin}/store/v1/downloads/{GRANT}?ticket=file-{i}")})).collect::<Vec<_>>()},
                    "quota":{"event":"grant_issued","charged":true,"units":1,"remaining":0,"unlimited":false},"replayed":self.keys.len()>1})
            }
            path if path.starts_with("/store/v1/downloads/") => {
                self.download_started.store(true, Ordering::SeqCst);
                if self.delay {
                    thread::sleep(Duration::from_millis(400));
                }
                if self.corrupt {
                    json!({"fixture":"Transport-only, not an installable operator"})
                } else {
                    self.payload.clone()
                }
            }
            _ => panic!("unexpected store endpoint: {path}"),
        };
        Some((200, result))
    }
}

#[test]
fn catalog_selection_is_exact_same_issuer_and_never_authorizes() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let page = crate::store_download::catalog(
        &identity,
        crate::store_download::CatalogRequest {
            query: "a & cursor=evil".into(),
            cursor: None,
        },
    )
    .unwrap();
    assert_eq!(page.issuer, fixture.config.issuer);
    assert_eq!(page.items[0].asset_id, "demo");
    assert_eq!(page.items[0].latest_version, "1.0.0");
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
    assert!(
        fixture
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(path, _)| path == "/store/v1/assets?limit=20&q=a+%26+cursor%3Devil")
    );
}

#[test]
fn catalog_rejects_ambiguous_identity_unbounded_pages_and_invalid_query_without_authorizing() {
    use crate::store_download::{CatalogRequest, catalog};
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let item = json!({"kind":"operator","asset_id":"demo","latest_version":"1.0.0","title":"Demo","summary":"Fixture","domain":"mechanical","languages":["en"]});
    for mode in ["floating", "duplicate", "path", "overflow", "cursor"] {
        let mut page = json!({"catalog_revision":"r1","items":[item.clone()],"next_cursor":null});
        match mode {
            "floating" => page["items"][0]["latest_version"] = json!("latest"),
            "duplicate" => page["items"].as_array_mut().unwrap().push(item.clone()),
            "path" => page["items"][0]["asset_id"] = json!("../elsewhere"),
            "overflow" => page["items"] = json!(vec![item.clone(); 21]),
            _ => page["next_cursor"] = json!("x".repeat(8193)),
        }
        fixture.store.lock().unwrap().catalog_reply = Some(page);
        assert_eq!(
            catalog(
                &identity,
                CatalogRequest {
                    query: String::new(),
                    cursor: None
                }
            )
            .err(),
            Some("invalid_store_response")
        );
    }
    let calls = fixture.calls.lock().unwrap().len();
    assert_eq!(
        catalog(
            &identity,
            CatalogRequest {
                query: "x".repeat(201),
                cursor: None
            }
        )
        .err(),
        Some("invalid_catalog_query")
    );
    assert_eq!(fixture.calls.lock().unwrap().len(), calls);
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
}

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-download-test-{}",
            random().unwrap().as_str()
        ));
        Self(path)
    }
    fn manager(&self) -> StoreDownloads {
        StoreDownloads::with_root(self.0.clone())
    }
    fn record(&self, id: &str) -> Value {
        serde_json::from_slice(&std::fs::read(self.0.join(id).join("record.json")).unwrap())
            .unwrap()
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn login(fixture: &Fixture) -> StoreIdentity {
    let identity = StoreIdentity::default();
    fixture.callback(&fixture.begin(&identity));
    wait_for(&identity, "signed_in");
    identity
}
fn prepared(fixture: &Fixture, identity: &StoreIdentity, manager: &StoreDownloads) -> DownloadView {
    let selection = fixture.store.lock().unwrap().selection.clone();
    manager.prepare(identity, selection).unwrap()
}
fn start(manager: &StoreDownloads, identity: &StoreIdentity, id: &str) {
    manager
        .handle(identity, DownloadRequest::Start { id: id.into() })
        .unwrap();
}
fn finished(manager: &StoreDownloads, identity: &StoreIdentity, state: &str) -> DownloadView {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let view = manager.list(identity).unwrap().remove(0);
        if view.state == state {
            return view;
        }
        assert!(
            Instant::now() < until,
            "expected {state}, got {} / {:?}",
            view.state,
            view.error
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn tls_signed_download_stages_only_and_never_persists_credentials() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    assert_eq!(view.state, "prepared");
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
    start(&manager, &identity, &view.id);
    let done = finished(&manager, &identity, "downloaded");
    let bytes = std::fs::read(PathBuf::from(done.directory.unwrap()).join(FILE)).unwrap();
    assert_eq!(
        bytes,
        serde_json::to_vec(&fixture.store.lock().unwrap().payload).unwrap()
    );
    assert_eq!(done.charged, Some(true));
    assert_eq!(done.remaining, Some(0));
    let record = disk.record(&view.id).to_string();
    for secret in ["access-old", "refresh-old", "secret-ticket", "download_url"] {
        assert!(!record.contains(secret));
    }
    assert!(!disk.0.join(&view.id).join("partial").exists());
}

#[test]
fn lost_grant_reply_restarts_with_same_key_even_when_quota_is_zero() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    fixture.store.lock().unwrap().drop_grant = true;
    start(&manager, &identity, &view.id);
    finished(&manager, &identity, "failed");
    assert_eq!(
        manager
            .handle(
                &identity,
                DownloadRequest::Remove {
                    id: view.id.clone()
                }
            )
            .err(),
        Some("authorization_outcome_unknown")
    );
    let restarted = disk.manager();
    assert_eq!(prepared(&fixture, &identity, &restarted).id, view.id);
    start(&restarted, &identity, &view.id);
    finished(&restarted, &identity, "downloaded");
    let store = fixture.store.lock().unwrap();
    assert_eq!(store.charges, 1);
    assert_eq!(store.keys.len(), 2);
    assert_eq!(store.keys[0], store.keys[1]);
}

#[test]
fn corruption_cleans_partial_and_retry_renews_without_new_charge() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    fixture.store.lock().unwrap().corrupt = true;
    start(&manager, &identity, &view.id);
    assert_eq!(
        finished(&manager, &identity, "failed").error.as_deref(),
        Some("artifact_mismatch")
    );
    assert!(!disk.0.join(&view.id).join("partial").exists());
    fixture.store.lock().unwrap().corrupt = false;
    start(&manager, &identity, &view.id);
    finished(&manager, &identity, "downloaded");
    let store = fixture.store.lock().unwrap();
    assert_eq!(store.charges, 1);
    assert_eq!(store.keys.len(), 1);
    assert_eq!(store.renewals, 1);
}

#[test]
fn signing_pin_recall_and_foreign_ticket_fail_closed() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let mut selection = fixture.store.lock().unwrap().selection.clone();
    selection.key_sha256 = "0".repeat(64);
    assert!(manager.prepare(&identity, selection).is_err());
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
    fixture.store.lock().unwrap().recalled = true;
    let selection = fixture.store.lock().unwrap().selection.clone();
    assert_eq!(
        manager.prepare(&identity, selection).err(),
        Some("release_recalled")
    );
    fixture.store.lock().unwrap().recalled = false;
    let view = prepared(&fixture, &identity, &manager);
    fixture.store.lock().unwrap().foreign_url = true;
    start(&manager, &identity, &view.id);
    assert_eq!(
        finished(&manager, &identity, "failed").error.as_deref(),
        Some("unsafe_download_url")
    );
    assert!(
        !fixture
            .store
            .lock()
            .unwrap()
            .download_started
            .load(Ordering::SeqCst)
    );
}

#[test]
fn logout_during_bytes_prevents_promotion() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    let began = {
        let mut store = fixture.store.lock().unwrap();
        store.delay = true;
        store.download_started.clone()
    };
    start(&manager, &identity, &view.id);
    let until = Instant::now() + Duration::from_secs(5);
    while !began.load(Ordering::SeqCst) {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(5));
    }
    identity.logout();
    while disk.record(&view.id)["state"] != "failed" {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(30));
    assert!(!disk.0.join(&view.id).join("files").exists());
    assert!(!disk.0.join(&view.id).join("partial").exists());
    assert_eq!(disk.record(&view.id)["error"], "session_changed");
}

#[test]
fn expired_idempotency_window_never_posts_again_or_forgets_the_key() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    fixture.store.lock().unwrap().drop_grant = true;
    start(&manager, &identity, &view.id);
    finished(&manager, &identity, "failed");
    let mut record = disk.record(&view.id);
    record["replay_until"] = json!(now() - 1);
    std::fs::write(
        disk.0.join(&view.id).join("record.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    start(&manager, &identity, &view.id);
    assert_eq!(
        finished(&manager, &identity, "failed").error.as_deref(),
        Some("authorization_recovery_expired")
    );
    assert_eq!(fixture.store.lock().unwrap().keys.len(), 1);
    assert_eq!(
        manager
            .handle(&identity, DownloadRequest::Remove { id: view.id })
            .err(),
        Some("authorization_outcome_unknown")
    );
}

#[test]
fn cancellation_retains_original_grant_and_cleans_owned_partial() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    let began = {
        let mut store = fixture.store.lock().unwrap();
        store.delay = true;
        store.download_started.clone()
    };
    start(&manager, &identity, &view.id);
    let until = Instant::now() + Duration::from_secs(5);
    while !began.load(Ordering::SeqCst) {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(5));
    }
    manager
        .handle(
            &identity,
            DownloadRequest::Cancel {
                id: view.id.clone(),
            },
        )
        .unwrap();
    let cancelled = finished(&manager, &identity, "cancelled");
    assert_eq!(cancelled.charged, Some(true));
    assert!(!disk.0.join(&view.id).join("partial").exists());
    assert!(!disk.0.join(&view.id).join("files").exists());
    fixture.store.lock().unwrap().delay = false;
    start(&manager, &identity, &view.id);
    finished(&manager, &identity, "downloaded");
    assert_eq!(fixture.store.lock().unwrap().charges, 1);
    assert_eq!(fixture.store.lock().unwrap().renewals, 1);
}

#[test]
fn signed_metadata_tampering_and_path_aliases_never_authorize() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let selection = fixture.store.lock().unwrap().selection.clone();
    let original = fixture.store.lock().unwrap().file.clone();
    fixture.store.lock().unwrap().file["sha256"] = json!("0".repeat(64));
    assert_eq!(
        manager.prepare(&identity, selection.clone()).err(),
        Some("signature_invalid")
    );
    for path in [
        "../outside",
        "/absolute",
        "CON/file.json",
        "safe/../../escape",
        "safe\\escape",
    ] {
        fixture.store.lock().unwrap().file = original.clone();
        fixture.store.lock().unwrap().file["path"] = json!(path);
        assert_eq!(
            manager.prepare(&identity, selection.clone()).err(),
            Some("artifact_mismatch")
        );
    }
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
}

#[test]
fn late_download_unauthorized_response_cannot_clear_a_rotated_session() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let stale = identity.store_lease().unwrap();
    identity.refresh().unwrap();
    assert_eq!(stale.check(), Err("session_changed"));
    stale.reject();
    assert_eq!(identity.status().status, "signed_in");
    let current = identity.store_lease().unwrap();
    assert_eq!(current.access.as_str(), "access-new");
    current.reject();
    assert_eq!(identity.store_lease().err(), Some("login_required"));
}

#[test]
fn refresh_retires_old_download_lease_even_when_access_value_is_reused() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let stale = identity.store_lease().unwrap();
    fixture.reuse_access.store(true, Ordering::SeqCst);
    identity.refresh().unwrap();
    assert_eq!(
        identity.store_lease().unwrap().access.as_str(),
        stale.access.as_str()
    );
    assert_eq!(stale.check(), Err("session_changed"));
    stale.reject();
    assert_eq!(identity.status().status, "signed_in");
}

#[test]
fn server_unauthorized_expires_session_and_does_not_authorize_or_expose_message() {
    let fixture = Fixture::new();
    let identity = login(&fixture);
    let disk = Sandbox::new();
    let manager = disk.manager();
    let view = prepared(&fixture, &identity, &manager);
    fixture.store.lock().unwrap().unauthorized = true;
    let started = manager.handle(
        &identity,
        DownloadRequest::Start {
            id: view.id.clone(),
        },
    );
    assert!(matches!(
        started,
        Ok(_) | Err("login_required") | Err("session_changed")
    ));
    let until = Instant::now() + Duration::from_secs(5);
    while disk.record(&view.id)["state"] != "failed" {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(30));
    assert_eq!(identity.status().status, "error");
    assert_eq!(identity.status().error, Some("login_required"));
    let record = disk.record(&view.id);
    assert_eq!(record["error"], "login_required");
    assert!(!record.to_string().contains("secret-ticket"));
    assert_eq!(fixture.store.lock().unwrap().charges, 0);
}
