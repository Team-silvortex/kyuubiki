use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kyuubiki_headless_sdk::{
    HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION, HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION,
    HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION, HeadlessExecutionBatch,
    HeadlessExecutionBatchStep, HeadlessParameterChange, HeadlessParameterPatch,
    HeadlessResearchMetricObjective, HeadlessResearchMetricSpec, HeadlessResearchRoundSpec,
    HeadlessRisk, HeadlessRunReport, apply_parameter_patch, build_headless_research_round_evidence,
    run_batch_dry,
};
use kyuubiki_kcore::{
    ContractBinding, ExportArtifact, ExportSpec, HEADLESS_RESEARCH_CONTRACT_NAME, Manifest,
    Producer, RESEARCH_BATCH_ROLE, RESEARCH_PATCH_ROLE, RESEARCH_ROUND_ROLE, RESEARCH_RUN_ROLE,
    SchemaReference, export_spec, verify_path,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

struct Fixture {
    root: PathBuf,
    spec: ExportSpec,
    second_report: PathBuf,
}

fn fixture(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kyuubiki-kcore-research-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn write_json(path: &Path, value: &impl Serialize) {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize fixture"),
    )
    .expect("write fixture");
}

fn batch() -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow_id: "research.kcore-thermal".to_string(),
        template_id: None,
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "service_health".to_string(),
            risk: HeadlessRisk::Normal,
            payload: json!({"research_input": 10.0}),
        }],
        warnings: vec![],
    }
}

fn report(batch: &HeadlessExecutionBatch, score: f64) -> HeadlessRunReport {
    let mut report = run_batch_dry(batch, false, false);
    assert!(report.validation.ok);
    report.mode = "execute:service".to_string();
    report.steps[0].status = "executed".to_string();
    report.steps[0].result_preview = json!({"result": {"score": score}});
    report
}

fn round_spec(round_id: &str, iteration: u64) -> HeadlessResearchRoundSpec {
    HeadlessResearchRoundSpec {
        schema_version: HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION.to_string(),
        round_id: round_id.to_string(),
        workflow_id: "research.kcore-thermal".to_string(),
        iteration,
        primary_metric_ids: vec!["score".to_string()],
        metrics: vec![HeadlessResearchMetricSpec {
            metric_id: "score".to_string(),
            pointer: "/steps/0/result_preview/result/score".to_string(),
            unit: "1".to_string(),
            objective: HeadlessResearchMetricObjective::Minimize,
        }],
    }
}

fn artifact(id: &str, role: &str, source: &str, schema: &str) -> ExportArtifact {
    ExportArtifact {
        id: id.to_string(),
        role: role.to_string(),
        media_type: "application/json".to_string(),
        source: source.to_string(),
        name: None,
        schema_ref: Some(SchemaReference {
            schema: schema.to_string(),
            version: "v1".to_string(),
        }),
        encoding: Some("json".to_string()),
        shape: vec![],
        unit: None,
        metadata: BTreeMap::new(),
    }
}

fn build_fixture(label: &str) -> Fixture {
    let root = fixture(label);
    fs::create_dir_all(&root).expect("create fixture");
    let first_batch = batch();
    let first_report = report(&first_batch, 48.0);
    let first_evidence = build_headless_research_round_evidence(
        &first_batch,
        &first_report,
        &round_spec("thermal-round-1", 1),
        None,
        None,
    )
    .expect("first evidence");

    let patch = HeadlessParameterPatch {
        schema_version: HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION.to_string(),
        patch_id: "thermal-input-round-2".to_string(),
        workflow_id: first_batch.workflow_id.clone(),
        template_id: None,
        changes: vec![HeadlessParameterChange {
            path: "/steps/0/payload/research_input".to_string(),
            expected: json!(10.0),
            value: json!(12.0),
        }],
    };
    let mut second_batch = first_batch.clone();
    let receipt = apply_parameter_patch(&mut second_batch, &patch).expect("patch batch");
    let second_report = report(&second_batch, 44.0);
    let second_evidence = build_headless_research_round_evidence(
        &second_batch,
        &second_report,
        &round_spec("thermal-round-2", 2),
        Some(&receipt),
        Some(&first_evidence),
    )
    .expect("second evidence");

    write_json(&root.join("round-1.batch.json"), &first_batch);
    write_json(&root.join("round-1.run.json"), &first_report);
    write_json(&root.join("round-1.evidence.json"), &first_evidence);
    write_json(&root.join("round-2.patch.json"), &patch);
    write_json(&root.join("round-2.batch.json"), &second_batch);
    let second_report_path = root.join("round-2.run.json");
    write_json(&second_report_path, &second_report);
    write_json(&root.join("round-2.evidence.json"), &second_evidence);

    let artifacts = vec![
        artifact(
            "round-1-batch",
            RESEARCH_BATCH_ROLE,
            "round-1.batch.json",
            "kyuubiki.headless-execution-batch",
        ),
        artifact(
            "round-1-run",
            RESEARCH_RUN_ROLE,
            "round-1.run.json",
            "kyuubiki.headless-execution-run",
        ),
        artifact(
            "round-1-evidence",
            RESEARCH_ROUND_ROLE,
            "round-1.evidence.json",
            "kyuubiki.headless-research-round-evidence",
        ),
        artifact(
            "round-2-patch",
            RESEARCH_PATCH_ROLE,
            "round-2.patch.json",
            "kyuubiki.headless-parameter-patch",
        ),
        artifact(
            "round-2-batch",
            RESEARCH_BATCH_ROLE,
            "round-2.batch.json",
            "kyuubiki.headless-execution-batch",
        ),
        artifact(
            "round-2-run",
            RESEARCH_RUN_ROLE,
            "round-2.run.json",
            "kyuubiki.headless-execution-run",
        ),
        artifact(
            "round-2-evidence",
            RESEARCH_ROUND_ROLE,
            "round-2.evidence.json",
            "kyuubiki.headless-research-round-evidence",
        ),
    ];
    let spec = ExportSpec {
        schema_version: "kyuubiki.kcore-export/v1".to_string(),
        core_id: "thermal-research-series".to_string(),
        title: "Thermal research series".to_string(),
        kind: "research-round-series".to_string(),
        producer: Producer {
            name: "kyuubiki-test".to_string(),
            version: "1".to_string(),
            runtime: Some("rust-native".to_string()),
        },
        artifacts,
        contracts: vec![ContractBinding {
            name: HEADLESS_RESEARCH_CONTRACT_NAME.to_string(),
            schema_version: HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION.to_string(),
            artifact_id: "round-2-evidence".to_string(),
            purpose: Some("self-contained research lineage".to_string()),
        }],
        entrypoints: vec!["round-2-evidence".to_string()],
        created_at: None,
        provenance: json!({"execution": "service"}),
        metadata: BTreeMap::new(),
    };
    Fixture {
        root,
        spec,
        second_report: second_report_path,
    }
}

#[test]
fn exports_and_reverifies_a_self_contained_two_round_research_series() {
    let fixture = build_fixture("roundtrip");
    let output = fixture.root.join("research.kcore");
    let export = export_spec(fixture.spec, &fixture.root, &output).expect("export research KCore");
    assert_eq!(export.semantic.contract_count, 1);
    assert_eq!(export.semantic.research_round_count, 2);

    let verification = verify_path(&output).expect("verify research KCore");
    assert_eq!(verification.semantic.contract_count, 1);
    assert_eq!(verification.semantic.research_round_count, 2);
    fs::remove_dir_all(fixture.root).expect("clean fixture");
}

#[test]
fn rejects_a_series_that_omits_the_reconstructing_parameter_patch() {
    let mut fixture = build_fixture("missing-patch");
    fixture
        .spec
        .artifacts
        .retain(|artifact| artifact.role != RESEARCH_PATCH_ROLE);
    let output = fixture.root.join("missing-patch.kcore");
    let error = export_spec(fixture.spec, &fixture.root, &output)
        .expect_err("missing patch must fail before export");
    assert!(error.contains("missing parameter patch"));
    assert!(!output.exists());
    fs::remove_dir_all(fixture.root).expect("clean fixture");
}

#[test]
fn rejects_research_evidence_that_omits_the_semantic_contract() {
    let mut fixture = build_fixture("missing-contract");
    fixture.spec.contracts.clear();
    let output = fixture.root.join("missing-contract.kcore");
    let error = export_spec(fixture.spec, &fixture.root, &output)
        .expect_err("research evidence without contract must fail");
    assert!(error.contains("requires the headless-research-round contract"));
    assert!(!output.exists());
    fs::remove_dir_all(fixture.root).expect("clean fixture");
}

#[test]
fn rejects_a_run_report_that_no_longer_matches_retained_evidence() {
    let fixture = build_fixture("tampered-report");
    let mut report: Value =
        serde_json::from_slice(&fs::read(&fixture.second_report).expect("read report"))
            .expect("decode report");
    report["steps"][0]["result_preview"]["result"]["score"] = json!(999.0);
    write_json(&fixture.second_report, &report);
    let output = fixture.root.join("tampered-report.kcore");
    let error = export_spec(fixture.spec, &fixture.root, &output)
        .expect_err("tampered report must fail before export");
    assert!(error.contains("missing its execution report"));
    assert!(!output.exists());
    fs::remove_dir_all(fixture.root).expect("clean fixture");
}

#[test]
fn verifier_rejects_a_resealed_but_semantically_mismatched_report() {
    let fixture = build_fixture("resealed-report");
    let original = fixture.root.join("original.kcore");
    export_spec(fixture.spec, &fixture.root, &original).expect("export valid research KCore");
    let resealed = fixture.root.join("resealed.kcore");
    reseal_with_changed_latest_score(&original, &resealed);

    let error = verify_path(&resealed).expect_err("semantic mismatch must fail verification");
    assert!(error.contains("missing its execution report"));
    fs::remove_dir_all(fixture.root).expect("clean fixture");
}

fn reseal_with_changed_latest_score(source: &Path, output: &Path) {
    let mut archive = ZipArchive::new(File::open(source).expect("open archive")).expect("archive");
    let mut manifest: Manifest = {
        let mut entry = archive.by_name("manifest.json").expect("manifest entry");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read manifest");
        serde_json::from_slice(&bytes).expect("decode manifest")
    };
    let artifact = manifest
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.id == "round-2-run")
        .expect("latest report artifact");
    let old_path = artifact.object_path.clone();
    let mut report: Value = {
        let mut entry = archive.by_name(&old_path).expect("report entry");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read report");
        serde_json::from_slice(&bytes).expect("decode report")
    };
    report["steps"][0]["result_preview"]["result"]["score"] = json!(999.0);
    let changed = serde_json::to_vec_pretty(&report).expect("encode changed report");
    artifact.byte_length = changed.len() as u64;
    artifact.sha256 = format!("{:x}", Sha256::digest(&changed));
    artifact.object_path = Manifest::object_path(&artifact.sha256);
    let changed_path = artifact.object_path.clone();
    manifest.seal().expect("reseal manifest");

    let mut writer = ZipWriter::new(File::create(output).expect("create resealed archive"));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("source entry");
        let name = entry.name().to_string();
        if name == "manifest.json" || name == old_path {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read source entry");
        let method = if name == "mimetype" {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(method),
            )
            .expect("start source entry");
        writer.write_all(&bytes).expect("write source entry");
    }
    writer
        .start_file(
            "manifest.json",
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start manifest");
    writer
        .write_all(&serde_json::to_vec_pretty(&manifest).expect("encode manifest"))
        .expect("write manifest");
    writer
        .start_file(
            changed_path,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start changed report");
    writer.write_all(&changed).expect("write changed report");
    writer.finish().expect("finish resealed archive");
}
