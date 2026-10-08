#[path = "../../../tests/support/diagnostic_domains.rs"]
mod diagnostic_domains;

use super::run_operator_task_ir;
use diagnostic_domains::DOMAINS;
use serde_json::{Value, json};

fn execute(task: Value) -> Result<Value, crate::operator_task_runtime::OperatorTaskRuntimeError> {
    run_operator_task_ir(&json!({"mode":"execute","task_ir":task}))
}

#[test]
fn domain_task_ir_diagnostics_and_quality_match_engine_without_package_fetch() {
    for domain in DOMAINS {
        let raw = domain.raw();
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, raw.clone(), json!({}))
                .unwrap();
        let diagnostic = execute(domain.task(
            "native-domain",
            domain.diagnostic,
            "extract",
            raw,
            json!({}),
        ))
        .unwrap();
        assert_eq!(
            diagnostic["operator_task_ir_status"], "executed",
            "{}",
            domain.name
        );
        assert_eq!(diagnostic["result"], expected);
        assert_eq!(diagnostic["package_fetch_request"], Value::Null);
        let config = domain.quality_config();
        let expected_quality = kyuubiki_engine::run_transform_operator(
            domain.quality,
            expected.clone(),
            config.clone(),
        )
        .unwrap();
        let quality = execute(domain.task(
            "native-quality",
            domain.quality,
            "transform",
            expected,
            config,
        ))
        .unwrap();
        assert_eq!(quality["operator_task_ir_status"], "executed");
        assert_eq!(quality["result"], expected_quality);
        assert_eq!(quality["package_fetch_request"], Value::Null);
    }
}

#[test]
fn domain_task_ir_rejects_corrupt_records_and_explicit_rerun_recovers() {
    for domain in DOMAINS {
        let raw = domain.raw();
        let bad = execute(domain.task(
            "bad-domain",
            domain.diagnostic,
            "extract",
            domain.corrupt(raw.clone()),
            json!({}),
        ))
        .unwrap_err();
        assert_eq!(bad.code, "operator_task_execution_failed");
        assert!(
            bad.message
                .contains(&format!("payload.nodes[0].{}", domain.scalar)),
            "{}",
            bad.message
        );
        assert_eq!(
            execute(domain.task("bad-domain", domain.diagnostic, "extract", raw, json!({})))
                .unwrap()["operator_task_ir_status"],
            "executed"
        );
    }
}

#[test]
fn domain_native_whitelist_cannot_be_extended_by_kind_or_prefix() {
    for domain in DOMAINS {
        for (id, wrong_kind, required) in [
            (domain.diagnostic, "transform", "extract"),
            (domain.quality, "extract", "transform"),
        ] {
            let error = execute(domain.task("wrong-kind", id, wrong_kind, json!({}), json!({})))
                .unwrap_err();
            assert!(
                error
                    .message
                    .contains(&format!("requires operator kind {required}")),
                "{}",
                error.message
            );
        }
        let task = domain.task(
            "preflight",
            domain.diagnostic,
            "extract",
            json!({}),
            json!({}),
        );
        let preflight = run_operator_task_ir(&json!({"mode":"preflight","task_ir":task})).unwrap();
        assert_ne!(preflight["operator_task_ir_status"], "executed");
        assert!(preflight["result"].is_null());
        let unknown = execute(domain.task(
            "unknown",
            &format!("{}_unknown", domain.diagnostic),
            "extract",
            domain.raw(),
            json!({}),
        ))
        .unwrap();
        assert_ne!(unknown["operator_task_ir_status"], "executed");
        assert!(unknown["result"].is_null());
    }
}
