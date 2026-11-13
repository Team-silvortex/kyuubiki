use super::{IdentityConfig, IdentityResult, now};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::signature;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    kid: String,
    #[serde(default)]
    typ: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}
#[derive(Deserialize)]
pub(super) struct Claims {
    pub sub: String,
    pub exp: u64,
    iss: String,
    aud: Audience,
    iat: u64,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    azp: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    at_hash: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Keys {
    keys: Vec<Key>,
}
#[derive(Deserialize)]
struct Key {
    kid: String,
    kty: String,
    n: String,
    e: String,
    #[serde(default)]
    alg: Option<String>,
    #[serde(default, rename = "use")]
    usage: Option<String>,
    #[serde(default)]
    key_ops: Option<Vec<String>>,
}

pub(super) fn verify(
    token: &str,
    keys: &Keys,
    config: &IdentityConfig,
    nonce: &str,
    refresh: bool,
    access: &str,
) -> IdentityResult<Claims> {
    let invalid = "invalid_id_token";
    if token.len() > 32768 {
        return Err(invalid);
    }
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(invalid);
    }
    let header: Header =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).map_err(|_| invalid)?)
            .map_err(|_| invalid)?;
    if header.alg != "RS256"
        || header.kid.is_empty()
        || header.kid.len() > 256
        || header.typ.as_deref().is_some_and(|typ| typ != "JWT")
    {
        return Err(invalid);
    }
    let candidates: Vec<_> = keys
        .keys
        .iter()
        .filter(|key| key.kid == header.kid)
        .collect();
    if candidates.len() != 1 {
        return Err(invalid);
    }
    let key = candidates[0];
    if key.kty != "RSA"
        || key.alg.as_deref().is_some_and(|alg| alg != "RS256")
        || key.usage.as_deref().is_some_and(|usage| usage != "sig")
        || key
            .key_ops
            .as_ref()
            .is_some_and(|ops| !ops.iter().any(|op| op == "verify"))
    {
        return Err(invalid);
    }
    let modulus = URL_SAFE_NO_PAD.decode(&key.n).map_err(|_| invalid)?;
    let exponent = URL_SAFE_NO_PAD.decode(&key.e).map_err(|_| invalid)?;
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).map_err(|_| invalid)?;
    signature::RsaPublicKeyComponents {
        n: &modulus,
        e: &exponent,
    }
    .verify(
        &signature::RSA_PKCS1_2048_8192_SHA256,
        &token.as_bytes()[..parts[0].len() + parts[1].len() + 1],
        &signature,
    )
    .map_err(|_| invalid)?;
    let payload = Zeroizing::new(URL_SAFE_NO_PAD.decode(parts[1]).map_err(|_| invalid)?);
    let claims: Claims = serde_json::from_slice(&payload).map_err(|_| invalid)?;
    let audience_ok = match &claims.aud {
        Audience::Single(aud) => aud == &config.client_id,
        Audience::Multiple(aud) => {
            !aud.is_empty()
                && aud.iter().any(|a| a == &config.client_id)
                && (aud.len() == 1 || claims.azp.as_ref() == Some(&config.client_id))
        }
    };
    if claims.iss != config.issuer
        || !audience_ok
        || claims
            .azp
            .as_ref()
            .is_some_and(|azp| azp != &config.client_id)
        || claims.exp <= now()
        || claims.iat > now().saturating_add(60)
        || claims.iat >= claims.exp
        || claims.nbf.is_some_and(|nbf| nbf > now())
        || claims.sub.is_empty()
        || claims.sub.len() > 256
        || claims.sub.chars().any(char::is_control)
        || (!refresh && claims.nonce.as_deref() != Some(nonce))
        || claims.nonce.as_deref().is_some_and(|value| value != nonce)
    {
        return Err(invalid);
    }
    if let Some(hash) = &claims.at_hash {
        let digest = Sha256::digest(access.as_bytes());
        if *hash != URL_SAFE_NO_PAD.encode(&digest[..16]) {
            return Err(invalid);
        }
    }
    Ok(claims)
}
