use std::env;

use kyuubiki_headless_sdk::{
    ControlPlaneClient, KyuubikiAuth, SdkResult, material_study_envelope_catalog_request,
};

fn main() -> SdkResult<()> {
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into());
    let auth = env::var("KYUUBIKI_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);

    let client = ControlPlaneClient::new_with_auth(&base_url, auth)?;
    let request = material_study_envelope_catalog_request(None);
    let job = client.submit_workflow_catalog_job(
        request["workflow_id"]
            .as_str()
            .expect("material envelope workflow id"),
        &request["input_artifacts"],
    )?;

    println!("{}", serde_json::to_string_pretty(&job)?);

    Ok(())
}
