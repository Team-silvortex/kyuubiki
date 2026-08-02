from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from .errors import ModelResearchExecutionError
from .material_research_bundle import validate_material_research_bundle
from .model_research_execution import MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION
from .model_research_frontier import validate_model_research_frontier
from .workflow_results import validate_workflow_result_against_graph


MODEL_RESEARCH_VALIDATION_REPORT_SCHEMA_VERSION = (
    "kyuubiki.model-research-validation-report/v2"
)
_CLAIM_BOUNDARY = "screening_only_not_qualification"
Verifier = Callable[[Mapping[str, Any]], bool]


def validate_model_research_frontier_result(
    frontier: Mapping[str, Any],
    result_receipt: Mapping[str, Any],
    graph: dict[str, Any],
    bundle: dict[str, Any] | None,
    frontier_verifier: Verifier,
    receipt_verifier: Verifier,
) -> dict[str, Any]:
    _validate_frontier_binding(frontier)
    _verify(frontier_verifier, frontier, "frontier")
    record = _validate_result_receipt(frontier, result_receipt)
    _verify(receipt_verifier, result_receipt, "receipt")
    if graph.get("id") != frontier["workflow_id"]:
        _fail("workflow graph id does not match research frontier workflow_id")

    validated = validate_workflow_result_against_graph(graph, record["output"])
    runtime = validated["workflow_runtime"]
    if runtime.get("status") != "completed":
        _fail("workflow result runtime status must be completed")
    runtime_workflow_id = runtime.get("workflow_id")
    if runtime_workflow_id is not None and runtime_workflow_id != frontier["workflow_id"]:
        _fail("workflow result runtime workflow_id does not match frontier")
    artifact_keys = sorted(validated["artifacts"])
    if not artifact_keys:
        _fail("workflow result validation produced no retained artifacts")

    if bundle is None:
        stage = "workflow_result_validated"
        bundle_evidence = None
        next_actions = [
            "build_or_attach_material_research_bundle",
            "external_validation_required",
        ]
    else:
        validate_material_research_bundle(bundle)
        readiness = bundle["validation_evidence"]["validation_readiness"]
        next_actions = list(dict.fromkeys(readiness["next_validation_actions"]))
        if "external_validation_required" not in next_actions:
            next_actions.append("external_validation_required")
        stage = "screening_bundle_validated"
        bundle_evidence = {
            "schema_version": bundle["schema_version"],
            "bundle_id": bundle["bundle_id"],
            "study": bundle["study"],
            "reliability_decision": bundle["summary"]["reliability_decision"],
            "validation_readiness_score": readiness["score"],
        }

    return {
        "schema_version": MODEL_RESEARCH_VALIDATION_REPORT_SCHEMA_VERSION,
        "session_id": frontier["session_id"],
        "workflow_id": frontier["workflow_id"],
        "job_id": frontier["job_id"],
        "origin_plan_digest": frontier["origin_plan_digest"],
        "result_plan_digest": result_receipt["plan_digest"],
        "stage": stage,
        "claim_boundary": _CLAIM_BOUNDARY,
        "external_validation_required": True,
        "workflow_result": {
            "graph_id": validated["graph_id"],
            "graph_version": validated["graph_version"],
            "runtime_status": runtime["status"],
            "artifact_keys": artifact_keys,
        },
        "material_bundle": bundle_evidence,
        "next_actions": next_actions,
    }


def _validate_frontier_binding(frontier: Mapping[str, Any]) -> None:
    validate_model_research_frontier(frontier)
    job_id = frontier.get("job_id")
    if (
        frontier.get("stage") != "ready_to_validate"
        or frontier.get("next_action") is not None
        or frontier.get("blocking_reason") is not None
        or not _present(job_id)
    ):
        _fail("research frontier is not ready for result validation")


def _validate_result_receipt(
    frontier: Mapping[str, Any], receipt: Mapping[str, Any]
) -> Mapping[str, Any]:
    records = receipt.get("records")
    record = records[-1] if isinstance(records, list) and records else None
    if (
        receipt.get("schema_version") != MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION
        or receipt.get("execution_authority") != "kyuubiki-headless-sdk"
        or receipt.get("status") != "completed"
        or receipt.get("session_id") != frontier.get("session_id")
        or receipt.get("workflow_id") != frontier.get("workflow_id")
        or receipt.get("plan_digest") != frontier.get("evidence", {}).get("plan_digest")
        or not isinstance(record, Mapping)
        or record.get("action") != "result_fetch"
        or record.get("job_id") != frontier.get("job_id")
        or not _present(record.get("authority"))
        or record.get("output") is None
        or record.get("error") is not None
    ):
        _fail("result receipt does not match the verified research frontier")
    return record


def _verify(verifier: Verifier, value: Mapping[str, Any], kind: str) -> None:
    if not callable(verifier) or not verifier(value):
        _fail(f"caller {kind} verifier rejected research evidence")


def _present(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _fail(message: str) -> None:
    raise ModelResearchExecutionError([message])
