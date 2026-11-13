use super::{Result, types::*};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ring::signature::{RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
pub(super) struct TrustRoots {
    keys: Vec<Key>,
    signature_payload: String,
    scheme: String,
}
#[derive(Deserialize)]
struct Key {
    key_id: String,
    algorithm: String,
    public_key_pem: String,
}

fn item<'a>(bytes: &mut &'a [u8], tag: u8) -> Result<&'a [u8]> {
    if bytes.len() < 2 || bytes[0] != tag {
        return Err("invalid_signing_key");
    }
    let mut len = bytes[1] as usize;
    *bytes = &bytes[2..];
    if len & 128 != 0 {
        let count = len & 127;
        if count == 0 || count > 2 || bytes.len() < count || bytes[0] == 0 {
            return Err("invalid_signing_key");
        }
        len = 0;
        for b in &bytes[..count] {
            len = (len << 8) | *b as usize;
        }
        *bytes = &bytes[count..];
        if len < 128 {
            return Err("invalid_signing_key");
        }
    }
    if bytes.len() < len {
        return Err("invalid_signing_key");
    }
    let value = &bytes[..len];
    *bytes = &bytes[len..];
    Ok(value)
}
fn rsa_der(pem: &str, pin: &str) -> Result<Vec<u8>> {
    if pem.len() > 16384 {
        return Err("invalid_signing_key");
    }
    let body = pem
        .trim()
        .strip_prefix("-----BEGIN PUBLIC KEY-----")
        .and_then(|s| s.strip_suffix("-----END PUBLIC KEY-----"))
        .ok_or("invalid_signing_key")?;
    let body: String = body.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let der = STANDARD.decode(body).map_err(|_| "invalid_signing_key")?;
    if format!("{:x}", Sha256::digest(&der)) != pin {
        return Err("signing_key_mismatch");
    }
    let mut outer = der.as_slice();
    let mut seq = item(&mut outer, 0x30)?;
    if !outer.is_empty() {
        return Err("invalid_signing_key");
    }
    let algorithm = item(&mut seq, 0x30)?;
    if algorithm != [6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0]
        && algorithm != [6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1]
    {
        return Err("invalid_signing_key");
    }
    let bits = item(&mut seq, 3)?;
    if !seq.is_empty() || bits.first() != Some(&0) {
        return Err("invalid_signing_key");
    }
    Ok(bits[1..].to_vec())
}
pub(super) fn verify(
    roots: TrustRoots,
    selection: &DownloadSelection,
    files: &[ArtifactFile],
) -> Result<()> {
    if roots.scheme != "silvortex-sha256-rs256-v1"
        || roots.signature_payload != "ascii_lowercase_sha256"
        || roots.keys.len() > 32
    {
        return Err("invalid_signing_key");
    }
    let keys: Vec<_> = roots
        .keys
        .iter()
        .filter(|k| k.key_id == selection.key_id)
        .collect();
    if keys.len() != 1 || keys[0].algorithm != "RS256" {
        return Err("signing_key_mismatch");
    }
    let der = rsa_der(&keys[0].public_key_pem, &selection.key_sha256)?;
    for file in files {
        let signature = &file.signature;
        if signature.key_id != selection.key_id
            || signature.algorithm != "RS256"
            || signature.scheme != roots.scheme
            || signature.signed_payload != "sha256_ascii"
            || signature.value.len() > 2048
        {
            return Err("signature_invalid");
        }
        let value = URL_SAFE_NO_PAD
            .decode(&signature.value)
            .map_err(|_| "signature_invalid")?;
        UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, &der)
            .verify(file.sha256.as_bytes(), &value)
            .map_err(|_| "signature_invalid")?;
    }
    Ok(())
}
