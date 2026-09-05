use super::distribution::{PACKAGE_ID, PACKAGE_VERSION};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};

type RunnerResult<T> = Result<T, String>;

pub(super) fn build(task_id: &str, entrypoint_sha256: &str) -> RunnerResult<Value> {
    let package_ref = format!("orchestra://operator-package/{PACKAGE_ID}");
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": task_id,
        "operator": {
            "id": PACKAGE_ID,
            "family": "template_summary",
            "kind": "extract"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "operator_package_acquisition_qualification",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": { "values": [2.0, 4.0, 8.0] },
        "config": { "qualification": "two_host_bound_orchestra_fetch" },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": PACKAGE_ID,
            "program_family": "template_summary",
            "program_kind": "extract",
            "operator_category_id": null,
            "package_ref": package_ref,
            "package_version": PACKAGE_VERSION,
            "package_integrity": {
                "algorithm": "sha256",
                "digest": entrypoint_sha256
            },
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": PACKAGE_ID,
                "operator_kind": "extract"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {},
        "orchestration_context": { "project_id": "operator-package-acquisition" },
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "cache_scope": "none",
            "agent_fetchable": true,
            "operator_kind": "extract",
            "package_ref": package_ref,
            "package_version": PACKAGE_VERSION,
            "required_capabilities": [],
            "placement_tags": []
        }
    });
    let digest = compute_operator_task_digest(&task)?;
    task["integrity"] = json!({ "task_digest": digest });
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_is_concrete_fetchable_and_disposable() {
        let task = build("task-a", &"a".repeat(64)).expect("task");
        assert_eq!(task["runtime_hints"]["execution_mode"], "orchestra_fetch");
        assert_eq!(task["runtime_hints"]["cache_scope"], "none");
        assert_eq!(
            task["execution_program"]["package_version"],
            PACKAGE_VERSION
        );
        assert_eq!(
            compute_operator_task_digest(&task).expect("digest"),
            task["integrity"]["task_digest"]
        );
    }
}
