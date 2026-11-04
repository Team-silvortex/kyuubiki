use super::policy::{self, Budget};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub fn retain(root: &Path, name: &str, value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "evidence serialization failed")?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join(name))
        .map_err(|_| "evidence file already exists or cannot be created")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "evidence write failed")?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

pub struct Provider {
    key: String,
    http: ureq::Agent,
    root: PathBuf,
    pub budget: Budget,
    pub usage: Vec<Value>,
}

impl Provider {
    pub fn new(root: &Path, key_file: &Path) -> Result<Self, String> {
        let file = File::open(key_file).map_err(|_| "credential file unavailable")?;
        let metadata = file
            .metadata()
            .map_err(|_| "credential metadata unavailable")?;
        if !metadata.is_file() || !(1..=8192).contains(&metadata.len()) {
            return Err("credential must be a small regular file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("credential file must not be group/world accessible".into());
            }
        }
        let mut raw = String::new();
        file.take(8193)
            .read_to_string(&mut raw)
            .map_err(|_| "credential read failed")?;
        let key = raw.trim().to_owned();
        if raw.len() > 8192
            || !key.starts_with("sk-")
            || !key.is_ascii()
            || key.bytes().any(|b| b.is_ascii_whitespace())
        {
            return Err("credential must contain only the raw API key".into());
        }
        let http = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(240)))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            key,
            http,
            root: root.into(),
            budget: Budget {
                charged: 0.0,
                calls: 0,
            },
            usage: vec![],
        })
    }

    pub fn request(
        &mut self,
        model: &str,
        input: &[Value],
        tools: Value,
    ) -> Result<Vec<Value>, String> {
        let body = json!({"model":model,"input":input,"tools":tools,
            "parallel_tool_calls":false,"store":false,"service_tier":"default",
            "reasoning":{"effort":"high"},"include":["reasoning.encrypted_content"],
            "max_output_tokens":policy::MAX_OUTPUT_TOKENS});
        let encoded = serde_json::to_vec(&body).map_err(|_| "request encoding failed")?;
        let reserved = self.budget.reserve(model, encoded.len())?;
        let call = self.budget.calls;
        retain(
            &self.root,
            &format!("api-{call:02}-reservation.json"),
            &json!({"model":model,"reserved_usd":reserved,"request_bytes":encoded.len(),
                "request_sha256":format!("{:x}",Sha256::digest(&encoded)),"retry":false}),
        )?;
        eprintln!("model call {call}: {model}, reservation ${reserved:.4}");
        // Fixed origin, no redirects, no raw response/error logging, no automatic retry.
        let mut response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send(&encoded)
            .map_err(|_| "provider transport failed; outcome/cost unknown, no retry")?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(format!(
                "provider HTTP {status}; response body withheld, no retry"
            ));
        }
        let value: Value = response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_json()
            .map_err(|_| "provider response invalid/oversized; no retry")?;
        let usage = &value["usage"];
        let input_tokens = usage["input_tokens"]
            .as_u64()
            .ok_or("missing provider input usage")?;
        let output_tokens = usage["output_tokens"]
            .as_u64()
            .ok_or("missing provider output usage")?;
        let cost = self
            .budget
            .settle(reserved, model, input_tokens, output_tokens)?;
        let record = json!({"model":model,"resolved_model":value["model"],"response_id":value["id"],
            "status":value["status"],"usage":usage,"estimated_standard_usd":cost});
        retain(&self.root, &format!("api-{call:02}-usage.json"), &record)?;
        self.usage.push(record);
        eprintln!(
            "model call {call}: input={input_tokens}, output={output_tokens}, estimated ${cost:.4}"
        );
        policy::completed_output(&value)
    }
}
