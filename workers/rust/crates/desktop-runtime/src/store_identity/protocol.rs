use super::{IdentityConfig, IdentityResult, IdentityStatus, Session, jwt, now};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::time::Duration;
use ureq::{
    Agent,
    tls::{Certificate, RootCerts, TlsConfig},
};
use zeroize::{Zeroize, Zeroizing};

const SCOPES: &str = "openid profile offline_access";
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    revocation_endpoint: String,
    code_challenge_methods_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    id_token_signing_alg_values_supported: Vec<String>,
    response_types_supported: Vec<String>,
    authorization_response_iss_parameter_supported: bool,
}
#[derive(Clone)]
pub(super) struct Provider {
    pub config: IdentityConfig,
    pub(super) http: Agent,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    token_type: String,
    expires_in: u64,
    id_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}
impl Drop for Tokens {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.id_token.zeroize();
        self.refresh_token.zeroize();
    }
}
#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    #[serde(default)]
    name: Option<String>,
}

fn read<T: DeserializeOwned>(
    response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> IdentityResult<T> {
    let mut response = response.map_err(|_| "identity_connection_failed")?;
    if response.status() != 200 {
        return Err("identity_request_rejected");
    }
    let body = Zeroizing::new(
        response
            .body_mut()
            .with_config()
            .limit(262144)
            .read_to_vec()
            .map_err(|_| "invalid_identity_response")?,
    );
    serde_json::from_slice(&body).map_err(|_| "invalid_identity_response")
}

impl Provider {
    pub(super) fn discover(config: IdentityConfig) -> IdentityResult<Self> {
        config.validate()?;
        let mut builder = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(12)))
            .max_redirects(0)
            .http_status_as_error(false)
            .https_only(true)
            .proxy(None);
        if !config.ca_pem.is_empty() {
            if config.ca_pem.matches("-----BEGIN CERTIFICATE-----").count() != 1
                || config.ca_pem.contains("PRIVATE KEY")
            {
                return Err("invalid_ca");
            }
            let cert = Certificate::from_pem(config.ca_pem.as_bytes()).map_err(|_| "invalid_ca")?;
            let fingerprint = format!("{:x}", Sha256::digest(cert.der()));
            if fingerprint != config.ca_sha256.to_ascii_lowercase() {
                return Err("ca_fingerprint_mismatch");
            }
            builder = builder.tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::new_with_certs(&[cert]))
                    .build(),
            );
        }
        let provider = Self {
            config,
            http: builder.build().new_agent(),
        };
        let info: Discovery = read(
            provider
                .http
                .get(provider.url("/.well-known/openid-configuration"))
                .call(),
        )?;
        let has = |items: &[String], value: &str| items.iter().any(|item| item == value);
        if info.issuer != provider.config.issuer
            || info.authorization_endpoint != provider.url("/account")
            || info.token_endpoint != provider.url("/v1/oidc/token")
            || info.userinfo_endpoint != provider.url("/v1/oidc/userinfo")
            || info.jwks_uri != provider.url("/v1/oidc/jwks")
            || info.revocation_endpoint != provider.url("/v1/oidc/revoke")
            || !info.authorization_response_iss_parameter_supported
            || !has(&info.code_challenge_methods_supported, "S256")
            || !has(&info.token_endpoint_auth_methods_supported, "none")
            || !has(&info.id_token_signing_alg_values_supported, "RS256")
            || !has(&info.response_types_supported, "code")
        {
            return Err("invalid_discovery");
        }
        Ok(provider)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.config.issuer)
    }

    pub(super) fn authorization_url(
        &self,
        state: &str,
        nonce: &str,
        challenge: &str,
    ) -> IdentityResult<String> {
        let mut url =
            url::Url::parse(&self.url("/account")).map_err(|_| "invalid_configuration")?;
        url.query_pairs_mut().extend_pairs([
            ("response_type", "code"),
            ("client_id", &self.config.client_id),
            ("redirect_uri", &self.config.redirect_uri()),
            ("scope", SCOPES),
            ("state", state),
            ("nonce", nonce),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ]);
        Ok(url.into())
    }

    pub(super) fn exchange(
        &self,
        code: &str,
        verifier: &str,
        nonce: Zeroizing<String>,
    ) -> IdentityResult<Session> {
        let tokens: Tokens = read(self.http.post(self.url("/v1/oidc/token")).send_form([
            ("grant_type", "authorization_code"),
            ("client_id", &self.config.client_id),
            ("redirect_uri", &self.config.redirect_uri()),
            ("code", code),
            ("code_verifier", verifier),
        ]))?;
        self.validate_tokens(tokens, nonce, None)
    }

    pub(super) fn renew(&self, old: &Session) -> IdentityResult<Session> {
        let tokens: Tokens = read(self.http.post(self.url("/v1/oidc/token")).send_form([
            ("grant_type", "refresh_token"),
            ("client_id", &self.config.client_id),
            (
                "refresh_token",
                old.refresh.as_deref().ok_or("login_required")?,
            ),
        ]))?;
        self.validate_tokens(tokens, old.nonce.clone(), Some(old))
    }

    fn validate_tokens(
        &self,
        tokens: Tokens,
        nonce: Zeroizing<String>,
        old: Option<&Session>,
    ) -> IdentityResult<Session> {
        let received_at = now();
        let validate = || -> IdentityResult<Session> {
            let valid_token = |token: &str| {
                !token.is_empty()
                    && token.len() <= 16384
                    && token.bytes().all(|b| b.is_ascii_graphic())
            };
            if !tokens.token_type.eq_ignore_ascii_case("bearer")
                || !valid_token(&tokens.access_token)
                || tokens.expires_in == 0
                || tokens.expires_in > 86400
                || tokens
                    .refresh_token
                    .as_deref()
                    .is_some_and(|token| !valid_token(token))
                || old.is_some_and(|session| {
                    tokens.refresh_token.as_deref() == session.refresh.as_ref().map(|v| v.as_str())
                })
            {
                return Err("invalid_identity_response");
            }
            let keys: jwt::Keys = read(self.http.get(self.url("/v1/oidc/jwks")).call())?;
            let claims = jwt::verify(
                &tokens.id_token,
                &keys,
                &self.config,
                &nonce,
                old.is_some(),
                &tokens.access_token,
            )?;
            if old.is_some_and(|session| session.subject != claims.sub) {
                return Err("subject_mismatch");
            }
            let scopes: Vec<String> = tokens
                .scope
                .as_deref()
                .map(|scope| scope.split_whitespace().map(String::from).collect())
                .unwrap_or_else(|| {
                    old.map(|session| session.scopes.clone())
                        .unwrap_or_else(|| SCOPES.split(' ').map(String::from).collect())
                });
            if !scopes.iter().any(|scope| scope == "openid")
                || scopes.iter().any(|scope| {
                    old.map(|session| !session.scopes.contains(scope))
                        .unwrap_or_else(|| !SCOPES.split(' ').any(|allowed| allowed == scope))
                })
            {
                return Err("invalid_scope");
            }
            let user: UserInfo = read(
                self.http
                    .get(self.url("/v1/oidc/userinfo"))
                    .header("Authorization", format!("Bearer {}", tokens.access_token))
                    .call(),
            )?;
            if user.sub != claims.sub {
                return Err("subject_mismatch");
            }
            let name = user
                .name
                .filter(|name| name.len() <= 512 && !name.chars().any(char::is_control));
            let view = IdentityStatus {
                status: "signed_in",
                issuer: Some(self.config.issuer.clone()),
                subject: Some(claims.sub.clone()),
                display_name: name,
                expires_at: Some(
                    claims
                        .exp
                        .min(received_at.saturating_add(tokens.expires_in)),
                ),
                can_refresh: tokens.refresh_token.is_some(),
                error: None,
            };
            Ok(Session {
                provider: self.clone(),
                access: Zeroizing::new(tokens.access_token.clone()),
                refresh: tokens
                    .refresh_token
                    .as_ref()
                    .map(|token| Zeroizing::new(token.clone())),
                subject: claims.sub,
                nonce,
                scopes,
                view,
            })
        };
        let result = validate();
        if result.is_err() {
            if let Some(refresh) = &tokens.refresh_token {
                let _ = self.revoke_token(refresh);
            }
            let _ = self.revoke_token(&tokens.access_token);
        }
        result
    }

    fn revoke_token(&self, token: &str) -> IdentityResult<()> {
        let _: serde_json::Value = read(self.http.post(self.url("/v1/oidc/revoke")).send_form([
            ("client_id", self.config.client_id.as_str()),
            ("token", token),
        ]))?;
        Ok(())
    }

    pub(super) fn revoke(&self, session: &Session) -> IdentityResult<()> {
        let refresh = session
            .refresh
            .as_ref()
            .map_or(Ok(()), |token| self.revoke_token(token));
        let access = self.revoke_token(&session.access);
        refresh.and(access)
    }
}
