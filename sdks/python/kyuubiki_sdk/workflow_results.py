from __future__ import annotations

from typing import Any

from .errors import WorkflowContractValidationError
from .workflow_contracts import validate_workflow_graph


def build_workflow_output_manifest(graph: dict[str, Any]) -> dict[str, Any]:
    validated = validate_workflow_graph(graph)
    node_map = {
        node["id"]: node
        for node in validated["nodes"]
        if isinstance(node, dict) and isinstance(node.get("id"), str)
    }
    outputs: list[dict[str, Any]] = []
    for node_id in validated.get("output_nodes") or []:
        node = node_map.get(node_id)
        if not isinstance(node, dict):
            continue
        for port in node.get("inputs", []):
            if not isinstance(port, dict):
                continue
            port_id = port.get("id")
            if not isinstance(port_id, str) or not port_id.strip():
                continue
            outputs.append(
                {
                    "key": f"{node_id}.{port_id}",
                    "node_id": node_id,
                    "port_id": port_id,
                    "artifact_type": port["artifact_type"],
                    "dataset_value": port.get("dataset_value"),
                    "required": port.get("required", True),
                }
            )
    return {
        "graph_id": validated["id"],
        "graph_version": validated["version"],
        "outputs": outputs,
    }


def validate_workflow_result_against_graph(
    graph: dict[str, Any],
    payload: dict[str, Any],
) -> dict[str, Any]:
    manifest = build_workflow_output_manifest(graph)
    artifacts = _extract_artifacts(payload)
    normalized: dict[str, Any] = {}
    errors: list[str] = []

    for output in manifest["outputs"]:
        artifact = _find_artifact(artifacts, output)
        if artifact is None:
            if output["required"]:
                errors.append(
                    f"workflow result is missing required artifact for output {output['key']!r}"
                )
            continue
        if isinstance(artifact, dict):
            artifact_type = artifact.get("artifact_type")
            if artifact_type is not None and artifact_type != output["artifact_type"]:
                errors.append(
                    f"workflow result artifact {output['key']!r} has mismatched artifact_type"
                )
            dataset_value = output.get("dataset_value")
            if dataset_value is not None:
                actual_dataset_value = artifact.get("dataset_value")
                if actual_dataset_value is not None and actual_dataset_value != dataset_value:
                    errors.append(
                        f"workflow result artifact {output['key']!r} has mismatched dataset_value"
                    )
        normalized[output["key"]] = artifact

    if errors:
        raise WorkflowContractValidationError(errors)

    return {
        "graph_id": manifest["graph_id"],
        "graph_version": manifest["graph_version"],
        "manifest": manifest,
        "workflow_runtime": normalize_workflow_runtime(payload),
        "artifacts": normalized,
    }


def normalize_workflow_runtime(payload: dict[str, Any]) -> dict[str, Any]:
    runtime = _extract_workflow_runtime(payload)
    current_node = runtime.get("current_node")
    if current_node is not None and not isinstance(current_node, str):
        raise WorkflowContractValidationError(["workflow runtime current_node must be a string"])
    completed_nodes = runtime.get("completed_nodes", [])
    if not isinstance(completed_nodes, list):
        raise WorkflowContractValidationError(["workflow runtime completed_nodes must be a list"])
    progress_events = runtime.get("progress_events", [])
    if not isinstance(progress_events, list):
        raise WorkflowContractValidationError(["workflow runtime progress_events must be a list"])
    failure = runtime.get("failure")
    if failure is not None and not isinstance(failure, dict):
        raise WorkflowContractValidationError(["workflow runtime failure must be an object"])
    workflow_id = runtime.get("workflow_id")
    if workflow_id is not None and not isinstance(workflow_id, str):
        raise WorkflowContractValidationError(["workflow runtime workflow_id must be a string"])
    run_id = runtime.get("run_id")
    if run_id is not None and not isinstance(run_id, str):
        raise WorkflowContractValidationError(["workflow runtime run_id must be a string"])
    status = runtime.get("status")
    if status is not None and not isinstance(status, str):
        raise WorkflowContractValidationError(["workflow runtime status must be a string"])
    return {
        "workflow_id": workflow_id,
        "run_id": run_id,
        "status": status,
        "current_node": current_node,
        "completed_nodes": completed_nodes,
        "progress_events": progress_events,
        "failure": failure,
    }


def normalize_workflow_progression(
    history: list[dict[str, Any]],
    result_payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    snapshots: list[dict[str, Any]] = []
    for index, payload in enumerate(history):
        if not isinstance(payload, dict):
            continue
        job = payload.get("job")
        if not isinstance(job, dict):
            continue
        current_node = job.get("current_node")
        completed_nodes = job.get("completed_nodes", [])
        progress_events = job.get("progress_events", [])
        if current_node is not None and not isinstance(current_node, str):
            raise WorkflowContractValidationError(["workflow progression current_node must be a string"])
        if not isinstance(completed_nodes, list):
            raise WorkflowContractValidationError(["workflow progression completed_nodes must be a list"])
        if not isinstance(progress_events, list):
            raise WorkflowContractValidationError(["workflow progression progress_events must be a list"])
        snapshots.append(
            {
                "index": index,
                "job_id": job.get("job_id"),
                "status": job.get("status"),
                "progress": job.get("progress"),
                "current_node": current_node,
                "completed_nodes": completed_nodes,
                "progress_events": progress_events,
            }
        )
    latest_runtime = normalize_workflow_runtime(result_payload) if isinstance(result_payload, dict) else None
    latest_snapshot = snapshots[-1] if snapshots else None
    return {
        "snapshots": snapshots,
        "latest": latest_runtime or latest_snapshot,
    }


def _extract_artifacts(payload: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise WorkflowContractValidationError(["workflow result payload must be an object"])
    artifacts = payload.get("artifacts")
    if isinstance(artifacts, dict):
        return artifacts
    result = payload.get("result")
    if isinstance(result, dict) and isinstance(result.get("artifacts"), dict):
        return result["artifacts"]
    raise WorkflowContractValidationError(
        ["workflow result payload must include an 'artifacts' object"]
    )


def _extract_workflow_runtime(payload: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise WorkflowContractValidationError(["workflow result payload must be an object"])
    if isinstance(payload.get("result"), dict):
        result = payload["result"]
        if any(
            key in result
            for key in (
                "workflow_id",
                "run_id",
                "status",
                "current_node",
                "completed_nodes",
                "progress_events",
                "failure",
            )
        ):
            return result
    return payload


def _find_artifact(artifacts: dict[str, Any], output: dict[str, Any]) -> Any | None:
    for key in (output["key"], output.get("dataset_value"), output["artifact_type"]):
        if isinstance(key, str) and key in artifacts:
            return artifacts[key]
    return None
