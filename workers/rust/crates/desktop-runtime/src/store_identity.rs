//! Native-only, memory-only shared account session. No token is serializable or sent to UI.
mod callback;
mod jwt;
mod protocol;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub type IdentityResult<T> = Result<T, &'static str>;
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum IdentityRequest {
    Status,
    Begin { config: IdentityConfig },
    Cancel,
    Refresh,
    Logout,
}
const CALLBACK_PATH: &str = "/kyuubiki/account/callback";
fn default_port() -> u16 {
    47831
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    pub issuer: String,
    pub client_id: String,
    #[serde(default = "default_port")]
    pub callback_port: u16,
    #[serde(default)]
    pub ca_pem: String,
    #[serde(default)]
    pub ca_sha256: String,
}

impl IdentityConfig {
    pub fn validate(&self) -> IdentityResult<()> {
        super::store_account::store_account_portal_url(&self.issuer, "en")
            .map_err(|_| "invalid_configuration")?;
        if self.issuer.ends_with('/')
            || self.client_id.is_empty()
            || self.client_id.len() > 256
            || !self.client_id.bytes().all(|b| b.is_ascii_graphic())
            || self.callback_port < 1024
            || self.ca_pem.len() > 16384
            || self.ca_sha256.len() > 64
            || self.ca_pem.is_empty() != self.ca_sha256.is_empty()
        {
            return Err("invalid_configuration");
        }
        Ok(())
    }
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}{CALLBACK_PATH}", self.callback_port)
    }
}

#[derive(Clone, Serialize)]
pub struct IdentityStatus {
    pub status: &'static str,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub display_name: Option<String>,
    pub expires_at: Option<u64>,
    pub can_refresh: bool,
    pub error: Option<&'static str>,
}

impl Default for IdentityStatus {
    fn default() -> Self {
        Self {
            status: "signed_out",
            issuer: None,
            subject: None,
            display_name: None,
            expires_at: None,
            can_refresh: false,
            error: None,
        }
    }
}

struct Session {
    provider: protocol::Provider,
    access: Zeroizing<String>,
    refresh: Option<Zeroizing<String>>,
    subject: String,
    nonce: Zeroizing<String>,
    scopes: Vec<String>,
    view: IdentityStatus,
}

#[derive(Default)]
struct State {
    epoch: u64,
    in_flight: bool,
    view: IdentityStatus,
    session: Option<Session>,
}

#[derive(Clone, Default)]
pub struct StoreIdentity(Arc<Mutex<State>>);

// A bounded native capability. It is deliberately neither public API nor serializable.
pub(crate) struct StoreSessionLease {
    owner: StoreIdentity,
    epoch: u64,
    pub(crate) issuer: String,
    pub(crate) subject: String,
    pub(crate) http: ureq::Agent,
    pub(crate) access: Zeroizing<String>,
}
impl StoreSessionLease {
    pub(crate) fn check(&self) -> IdentityResult<()> {
        self.with_current(|| Ok(()))
    }
    pub(crate) fn with_current<T>(
        &self,
        commit: impl FnOnce() -> IdentityResult<T>,
    ) -> IdentityResult<T> {
        let state = self.owner.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.epoch != self.epoch
            || state.session.as_ref().is_none_or(|s| {
                s.access.as_str() != self.access.as_str()
                    || s.view.expires_at.is_none_or(|expiry| expiry <= now())
            })
        {
            return Err("session_changed");
        }
        // Only bounded local receipt commits hold this lock; never network or file streaming.
        commit()
    }
    pub(crate) fn reject(&self) {
        let mut state = self.owner.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.epoch == self.epoch
            && state
                .session
                .as_ref()
                .is_some_and(|s| s.access.as_str() == self.access.as_str())
        {
            state.session = None;
            state.view = IdentityStatus {
                status: "error",
                error: Some("login_required"),
                ..Default::default()
            };
        }
    }
}

struct Flight(Arc<Mutex<State>>);
impl Drop for Flight {
    fn drop(&mut self) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).in_flight = false;
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn random() -> IdentityResult<Zeroizing<String>> {
    let mut bytes = [0; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "random_unavailable")?;
    Ok(Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes)))
}

impl StoreIdentity {
    pub(crate) fn store_lease(&self) -> IdentityResult<StoreSessionLease> {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let session = state.session.as_ref().ok_or("login_required")?;
        if state.in_flight || session.view.expires_at.is_none_or(|expiry| expiry <= now()) {
            return Err("login_required");
        }
        Ok(StoreSessionLease {
            owner: self.clone(),
            epoch: state.epoch,
            issuer: session.provider.config.issuer.clone(),
            subject: session.subject.clone(),
            http: session.provider.http.clone(),
            access: session.access.clone(),
        })
    }
    pub fn handle(&self, request: IdentityRequest) -> IdentityResult<IdentityStatus> {
        match request {
            IdentityRequest::Status => Ok(self.status()),
            IdentityRequest::Begin { config } => self.begin(config),
            IdentityRequest::Cancel => Ok(self.cancel()),
            IdentityRequest::Refresh => self.refresh(),
            IdentityRequest::Logout => Ok(self.logout()),
        }
    }
    pub fn status(&self) -> IdentityStatus {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let mut view = state.view.clone();
        if view.status == "signed_in" && view.expires_at.is_some_and(|expiry| expiry <= now()) {
            view.status = "expired";
        }
        view
    }

    fn active(&self, epoch: u64) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).epoch == epoch
    }

    fn fail(&self, epoch: u64, error: &'static str) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.epoch == epoch {
            state.session = None;
            state.view = IdentityStatus {
                status: "error",
                error: Some(error),
                ..Default::default()
            };
        }
    }

    pub fn begin(&self, config: IdentityConfig) -> IdentityResult<IdentityStatus> {
        self.begin_with_browser(config, |url| {
            super::store_account::open_browser(url).map_err(|_| "browser_unavailable")
        })
    }

    fn begin_with_browser(
        &self,
        config: IdentityConfig,
        open: impl FnOnce(&str) -> IdentityResult<()>,
    ) -> IdentityResult<IdentityStatus> {
        config.validate()?;
        let epoch = {
            let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
            if state.in_flight || state.session.is_some() {
                return Err("session_busy");
            }
            state.epoch += 1;
            state.in_flight = true;
            state.view = IdentityStatus {
                status: "preparing",
                issuer: Some(config.issuer.clone()),
                ..Default::default()
            };
            state.epoch
        };
        let flight = Flight(self.0.clone());
        let setup = || -> IdentityResult<_> {
            let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, config.callback_port))
                .map_err(|_| "callback_port_unavailable")?;
            listener
                .set_nonblocking(true)
                .map_err(|_| "callback_unavailable")?;
            let provider = protocol::Provider::discover(config)?;
            let state = random()?;
            let nonce = random()?;
            let verifier = random()?;
            let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
            let url = provider.authorization_url(&state, &nonce, &challenge)?;
            if !self.active(epoch) {
                return Err("cancelled");
            }
            open(&url)?;
            Ok((listener, provider, state, nonce, verifier))
        };
        let (listener, provider, state, nonce, verifier) = match setup() {
            Ok(value) => value,
            Err(error) => {
                self.fail(epoch, error);
                return Err(error);
            }
        };
        {
            let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
            if inner.epoch != epoch {
                return Err("cancelled");
            }
            inner.view.status = "awaiting_browser";
        }
        let manager = self.clone();
        std::thread::Builder::new()
            .name("store-login".into())
            .spawn(move || {
                let _flight = flight;
                let result = callback::wait(
                    listener,
                    &provider.config,
                    &state,
                    || manager.active(epoch),
                    Instant::now() + Duration::from_secs(300),
                );
                let result = result.and_then(|code| {
                    let mut inner = manager.0.lock().unwrap_or_else(|e| e.into_inner());
                    if inner.epoch != epoch {
                        return Err("cancelled");
                    }
                    inner.view.status = "exchanging";
                    drop(inner);
                    provider.exchange(&code, &verifier, nonce)
                });
                match result {
                    Ok(session) => manager.accept(epoch, session),
                    Err(error) => manager.fail(epoch, error),
                }
            })
            .map_err(|_| {
                self.fail(epoch, "callback_unavailable");
                "callback_unavailable"
            })?;
        Ok(self.status())
    }

    fn accept(&self, epoch: u64, session: Session) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.epoch == epoch {
            inner.view = session.view.clone();
            inner.session = Some(session);
        } else {
            drop(inner);
            let _ = session.provider.revoke(&session);
        }
    }

    /// Refresh is single-flight, never replayed after an ambiguous response.
    pub fn refresh(&self) -> IdentityResult<IdentityStatus> {
        let (epoch, session) = {
            let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
            if inner.in_flight {
                return Err("session_busy");
            }
            if inner
                .session
                .as_ref()
                .is_none_or(|session| session.refresh.is_none())
            {
                return Err("login_required");
            }
            let session = inner.session.take().ok_or("login_required")?;
            inner.epoch += 1;
            inner.view.status = "refreshing";
            inner.in_flight = true;
            (inner.epoch, session)
        };
        let _flight = Flight(self.0.clone());
        match session.provider.renew(&session) {
            Ok(updated) => self.accept(epoch, updated),
            Err(error) => {
                self.fail(epoch, error);
                // Revocation is safe to repeat; a refresh grant is not.
                let _ = session.provider.revoke(&session);
                return Err(error);
            }
        }
        Ok(self.status())
    }

    /// Logout invalidates callbacks first. Website SSO cookies are intentionally untouched.
    pub fn logout(&self) -> IdentityStatus {
        let (epoch, session) = {
            let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
            state.epoch += 1;
            state.view = IdentityStatus::default();
            (state.epoch, state.session.take())
        };
        if session
            .as_ref()
            .is_some_and(|s| s.provider.revoke(s).is_err())
        {
            let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
            if state.epoch == epoch {
                state.view.error = Some("revocation_unconfirmed");
            }
        }
        self.status()
    }

    pub fn cancel(&self) -> IdentityStatus {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(
            state.view.status,
            "preparing" | "awaiting_browser" | "exchanging"
        ) {
            state.epoch += 1;
            state.view = IdentityStatus::default();
        }
        drop(state);
        self.status()
    }
}

#[cfg(all(test, unix))]
mod tests;
