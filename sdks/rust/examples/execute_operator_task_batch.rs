use std::env;
use std::fs;

use kyuubiki_headless_sdk::{ControlPlaneClient, KyuubikiAuth, SdkResult};

fn main() -> SdkResult<()> {
    let batch_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: cargo run --example execute_operator_task_batch -- batch.json");
        std::process::exit(64);
    });
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into());
    let auth = env::var("KYUUBIKI_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);
    let batch: serde_json::Value = serde_json::from_str(&fs::read_to_string(batch_path)?)?;

    let client = ControlPlaneClient::new_with_auth(&base_url, auth)?;
    let result = client.execute_operator_task_batch(&batch)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
