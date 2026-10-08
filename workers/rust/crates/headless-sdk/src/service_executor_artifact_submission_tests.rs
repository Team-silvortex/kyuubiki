use super::*;

#[test]
fn large_direct_fem_submit_uploads_a_model_artifact_then_sends_its_reference() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let port = listener.local_addr().expect("local addr").port();
    let handle = std::thread::spawn(move || {
        let (mut upload, _) = listener.accept().expect("accept artifact upload");
        let upload_request = read_complete_http_request(&mut upload);
        let upload_text = String::from_utf8_lossy(&upload_request);
        assert!(upload_text.starts_with("POST /api/v1/model-artifacts HTTP/1.1\r\n"));
        assert!(upload_text.contains("Content-Type: application/vnd.kyuubiki.model+json\r\n"));
        assert!(upload_request.len() > MAX_INLINE_JSON_BYTES);
        let split = upload_request
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        let upload_body = &upload_request[split..];
        use sha2::{Digest, Sha256};
        let artifact_id = format!("{:x}", Sha256::digest(upload_body));
        write_test_response(
            &mut upload,
            "201 Created",
            &json!({"artifact":{"schema_version":"kyuubiki.model-artifact-ref/v1",
                "artifact_id":artifact_id,"sha256":artifact_id,"size_bytes":upload_body.len(),
                "media_type":"application/vnd.kyuubiki.model+json","immutable":true}})
            .to_string(),
        );
        drop(upload);

        let (mut submit, _) = listener.accept().expect("accept solve submission");
        let submit_request = read_complete_http_request(&mut submit);
        let submit_text = String::from_utf8_lossy(&submit_request);
        assert!(submit_text.starts_with("POST /api/v1/fem/heat-plane-quad-2d/jobs HTTP/1.1\r\n"));
        assert!(submit_text.contains("\"model_artifact_ref\""));
        assert!(submit_text.contains(&artifact_id));
        assert!(!submit_text.contains("large-model-padding"));
        assert!(submit_text.contains("\"project_id\":\"artifact-project\""));
        assert!(submit_text.contains("\"model_version_id\":\"artifact-version\""));
        assert!(!upload_text.contains("artifact-project"));
        assert!(!upload_text.contains("artifact-version"));
        write_test_response(
            &mut submit,
            "202 Accepted",
            r#"{"job":{"job_id":"artifact-job","status":"queued","progress":0.0,"project_id":"artifact-project","model_version_id":"artifact-version"}}"#,
        );
    });

    let model = json!({
        "nodes": [],
        "elements": [],
        "large-model-padding": "x".repeat(MAX_INLINE_JSON_BYTES)
    });
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"));
    let outcome = executor
        .execute_step("solve_heat_plane_quad_2d", 1,
            &json!({"model":model,"project_id":"artifact-project","model_version_id":"artifact-version"}))
        .expect("large FEM request should use artifact transport");

    handle.join().expect("server thread should finish");
    assert_eq!(outcome.result["job_id"], "artifact-job");
    assert_eq!(
        outcome.result["model_artifact_upload"]["size_bytes"],
        serde_json::to_vec(&model).unwrap().len()
    );
    assert_eq!(outcome.result["model_artifact_upload"]["immutable"], true);
}
