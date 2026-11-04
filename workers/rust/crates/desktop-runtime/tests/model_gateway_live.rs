use serde_json::{Value, json};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct NativeFrontend {
    child: Option<Child>,
    root: PathBuf,
    origin: String,
}

impl Drop for NativeFrontend {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn client(timeout: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(timeout)))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

impl NativeFrontend {
    fn start() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "test port unavailable")?;
        let port = listener
            .local_addr()
            .map_err(|_| "test port unavailable")?
            .port();
        let root =
            std::env::temp_dir().join(format!("kyuubiki-model-live-{}-{port}", std::process::id()));
        fs::create_dir(&root).map_err(|_| "test workspace unavailable")?;
        let mut runtime = Self {
            child: None,
            root,
            origin: format!("http://127.0.0.1:{port}"),
        };
        fs::write(
            runtime.root.join("index.html"),
            "<!doctype html><title>Model gateway test</title>",
        )
        .map_err(|_| "test fixture creation failed")?;
        drop(listener);
        runtime.child = Some(
            Command::new(env!("CARGO_BIN_EXE_kyuubiki-runtime"))
                .args([
                    "serve-frontend",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    &port.to_string(),
                    "--root",
                ])
                .arg(&runtime.root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| "native frontend failed to launch")?,
        );
        for _ in 0..50 {
            if let Ok(mut response) = client(1)
                .get(format!("{}/api/assistant/capabilities", runtime.origin))
                .call()
            {
                if let Ok(value) = response
                    .body_mut()
                    .with_config()
                    .limit(4096)
                    .read_json::<Value>()
                {
                    if value["schema_version"] == "kyuubiki.model-gateway/v1" {
                        return Ok(runtime);
                    }
                }
            }
            if runtime
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(|_| "native frontend status failed")?
                .is_some()
            {
                return Err("native frontend exited before readiness".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err("native frontend readiness timed out".into())
    }

    fn request(&self, key: &str, operation: &str, payload: Option<Value>) -> Result<Value, String> {
        let mut input =
            json!({ "base_url":"https://api.openai.com/v1", "api_key":key, "operation":operation });
        if let Some(payload) = payload {
            input["payload"] = payload;
        }
        let started = Instant::now();
        let mut response = client(70)
            .post(format!("{}/api/assistant/request", self.origin))
            .header("Origin", &self.origin)
            .send_json(&input)
            .map_err(|_| format!("{operation}: local gateway request failed"))?;
        let status = response.status().as_u16();
        println!(
            "operation={operation} http_status={status} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        if status != 200 {
            return Err(format!(
                "{operation}: HTTP {status}; response body intentionally withheld"
            ));
        }
        response
            .body_mut()
            .with_config()
            .limit(1024 * 1024)
            .read_json::<Value>()
            .map_err(|_| format!("{operation}: invalid or oversized response"))
    }
}

#[test]
#[ignore = "Opt-in: reads a user-selected credential file and makes one billable OpenAI request"]
fn openai_model_connection_through_native_gateway() -> Result<(), String> {
    if std::env::var("KYUUBIKI_LIVE_MODEL_API_TEST").as_deref() != Ok("1") {
        return Err("set KYUUBIKI_LIVE_MODEL_API_TEST=1 to authorize a live API test".into());
    }
    let path = std::env::var_os("KYUUBIKI_MODEL_API_KEY_FILE")
        .ok_or("credential file must be explicitly selected")?;
    if fs::metadata(&path)
        .map_err(|_| "credential file unavailable")?
        .len()
        > 8192
    {
        return Err("credential file is oversized".into());
    }
    let mut key = fs::read_to_string(path).map_err(|_| "credential file cannot be read")?;
    key = key.trim().to_string();
    if !key.starts_with("sk-") || !key.bytes().all(|value| value.is_ascii_graphic()) {
        return Err("expected a raw OpenAI API key; credential content withheld".into());
    }
    let model = "gpt-5.4-mini";
    let runtime = NativeFrontend::start()?;
    let models = runtime.request(&key, "models", None)?;
    let entries = models["data"]
        .as_array()
        .ok_or("provider returned an invalid model list")?;
    if !entries.iter().any(|entry| entry["id"] == model) {
        return Err("selected low-cost model was not listed; no inference attempted".into());
    }
    println!("model={model} listed=true");
    let response = runtime.request(&key, "plan", Some(json!({
        "model":model, "store":false, "max_completion_tokens":256, "reasoning_effort":"none",
        "messages":[
            {"role":"system", "content":"Return only JSON with summary (string), rationale (string), suggested_actions (empty array). No tools or actions. Keep each string under 160 characters."},
            {"role":"user", "content":"Synthetic connectivity test only: describe checking mesh convergence before trusting a one-dimensional elastic bar simulation. Do not claim a simulation was run."}
        ]
    })))?;
    drop(key);
    let choice = &response["choices"][0];
    if choice["finish_reason"] != "stop" {
        return Err("completion was truncated or not completed normally".into());
    }
    let content = choice["message"]["content"]
        .as_str()
        .ok_or("completion content missing")?;
    let plan: Value =
        serde_json::from_str(content).map_err(|_| "completion did not return strict plan JSON")?;
    if !plan["summary"].is_string()
        || !plan["rationale"].is_string()
        || !plan["suggested_actions"]
            .as_array()
            .is_some_and(Vec::is_empty)
    {
        return Err("completion failed the non-executing plan contract".into());
    }
    let input_tokens = response["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let output_tokens = response["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    println!(
        "plan_contract=passed actions=0 input_tokens={input_tokens} output_tokens={output_tokens}"
    );
    Ok(())
}
