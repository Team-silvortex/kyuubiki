from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from .errors import ModelResearchExecutionError
from .model_collaboration import MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION
from .model_plan_approval import _compute_canonical_json_digest
from .model_research_execution import MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION


MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION = "kyuubiki.model-research-frontier/v2"
ReceiptVerifier = Callable[[Mapping[str, Any]], bool]
FrontierVerifier = Callable[[Mapping[str, Any]], bool]
_SUBMIT_ACTIONS = {"fem_submit", "workflow_submit_catalog", "workflow_submit_graph"}


class ModelFrontierDigestVerifier:
    def __init__(self, expected_digest: str) -> None:
        if not _valid_plan_digest(expected_digest):
            _fail("expected research frontier digest is invalid")
        self.expected_digest = expected_digest

    def __call__(self, current: Mapping[str, Any]) -> bool:
        verify_model_research_frontier_digest(current, self.expected_digest)
        return True


def start_model_research_frontier(
    receipt: Mapping[str, Any], receipt_verifier: ReceiptVerifier
) -> dict[str, Any]:
    _validate_receipt(receipt)
    _verify_receipt(receipt, receipt_verifier)
    record = _last_record(receipt)
    if receipt["status"] == "failed":
        return _blocked_frontier(receipt, record, receipt["plan_digest"], None, 1)
    if record.get("action") not in _SUBMIT_ACTIONS:
        _fail("initial research receipt must end with a supported job submission")
    job = record.get("output")
    job_id = job.get("job", {}).get("job_id") if isinstance(job, Mapping) else None
    if not isinstance(job_id, str) or not job_id.strip():
        _fail("job submission receipt did not contain job.job_id")
    return _frontier(
        receipt,
        record,
        origin_plan_digest=receipt["plan_digest"],
        stage="waiting_for_job",
        job_id=job_id,
        next_action="job_wait",
        transition_count=1,
    )


def compute_model_research_frontier_digest(current: Mapping[str, Any]) -> str:
    validate_model_research_frontier(current)
    return _compute_canonical_json_digest(_frontier_digest_projection(current))


def verify_model_research_frontier_digest(
    current: Mapping[str, Any], expected_digest: str
) -> None:
    if not _valid_plan_digest(expected_digest):
        _fail("expected research frontier digest is invalid")
    if compute_model_research_frontier_digest(current) != expected_digest:
        _fail("persisted research frontier digest does not match trusted state")


def advance_model_research_frontier(
    current: Mapping[str, Any],
    receipt: Mapping[str, Any],
    frontier_verifier: FrontierVerifier,
    receipt_verifier: ReceiptVerifier,
) -> dict[str, Any]:
    validate_model_research_frontier(current)
    _verify_frontier(current, frontier_verifier)
    _validate_receipt(receipt)
    _verify_receipt(receipt, receipt_verifier)
    if (
        receipt["session_id"] != current["session_id"]
        or receipt["workflow_id"] != current["workflow_id"]
    ):
        _fail("research receipt does not match frontier session and workflow")
    expected = current.get("next_action")
    if not isinstance(expected, str) or not expected:
        _fail("research frontier has no executable next action")
    record = _last_record(receipt)
    if receipt["status"] == "failed":
        return _blocked_frontier(
            receipt,
            record,
            current["origin_plan_digest"],
            current.get("job_id"),
            current["transition_count"] + 1,
        )
    if record.get("action") != expected:
        _fail(
            f"research receipt ended with {record.get('action')}; "
            f"frontier requires {expected}"
        )
    if record.get("job_id") != current.get("job_id"):
        _fail("research receipt job_id does not match frontier binding")

    if expected == "job_wait":
        return _advance_wait(current, receipt, record)
    if expected == "result_fetch":
        return _frontier(
            receipt,
            record,
            origin_plan_digest=current["origin_plan_digest"],
            stage="ready_to_validate",
            job_id=current["job_id"],
            next_action=None,
            transition_count=current["transition_count"] + 1,
        )
    _fail(f"unsupported frontier next action: {expected}")


def build_model_research_frontier_proposal(
    current: Mapping[str, Any],
    frontier_verifier: FrontierVerifier,
) -> dict[str, Any]:
    validate_model_research_frontier(current)
    _verify_frontier(current, frontier_verifier)
    action = current.get("next_action")
    if not isinstance(action, str) or not action:
        _fail("research frontier has no executable next action")
    job_id = current.get("job_id")
    if not isinstance(job_id, str) or not job_id:
        _fail("research frontier has no bound job_id")
    return {
        "schema_version": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
        "session_id": current["session_id"],
        "summary": f"Advance verified research frontier with {action}",
        "calls": [
            {
                "id": f"frontier-{current['transition_count'] + 1}-{action}",
                "action": action,
                "payload": {"job_id": job_id},
                "reason": "Use the job identifier retained from verified execution evidence",
            }
        ],
    }


def _advance_wait(
    current: Mapping[str, Any],
    receipt: Mapping[str, Any],
    record: Mapping[str, Any],
) -> dict[str, Any]:
    output = record.get("output")
    terminal = output.get("terminal") if isinstance(output, Mapping) else None
    job = terminal.get("job") if isinstance(terminal, Mapping) else None
    status = job.get("status") if isinstance(job, Mapping) else None
    if status == "completed":
        return _frontier(
            receipt,
            record,
            origin_plan_digest=current["origin_plan_digest"],
            stage="ready_to_fetch_result",
            job_id=current["job_id"],
            next_action="result_fetch",
            transition_count=current["transition_count"] + 1,
            job_status=status,
        )
    if status in {"failed", "cancelled"}:
        return _frontier(
            receipt,
            record,
            origin_plan_digest=current["origin_plan_digest"],
            stage="blocked",
            job_id=current["job_id"],
            next_action=None,
            transition_count=current["transition_count"] + 1,
            job_status=status,
            blocking_reason=f"job reached terminal status {status}",
        )
    if not isinstance(status, str):
        _fail("job_wait receipt did not contain terminal.job.status")
    _fail(f"job_wait returned non-terminal status {status}")


def _frontier(
    receipt: Mapping[str, Any],
    record: Mapping[str, Any],
    *,
    origin_plan_digest: str,
    stage: str,
    job_id: str | None,
    next_action: str | None,
    transition_count: int,
    job_status: str | None = None,
    blocking_reason: str | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION,
        "session_id": receipt["session_id"],
        "workflow_id": receipt["workflow_id"],
        "origin_plan_digest": origin_plan_digest,
        "stage": stage,
        "job_id": job_id,
        "next_action": next_action,
        "transition_count": transition_count,
        "evidence": {
            "approval_id": receipt.get("approval_id"),
            "plan_digest": receipt["plan_digest"],
            "action": record.get("action"),
            "record_index": record.get("index"),
            "authority": record.get("authority"),
            "job_status": job_status,
        },
        "blocking_reason": blocking_reason,
    }


def _blocked_frontier(
    receipt: Mapping[str, Any],
    record: Mapping[str, Any],
    origin_plan_digest: str,
    job_id: str | None,
    transition_count: int,
) -> dict[str, Any]:
    reason = record.get("error")
    return _frontier(
        receipt,
        record,
        origin_plan_digest=origin_plan_digest,
        stage="blocked",
        job_id=job_id,
        next_action=None,
        transition_count=transition_count,
        blocking_reason=reason if isinstance(reason, str) else "research execution failed",
    )


def _validate_receipt(receipt: Mapping[str, Any]) -> None:
    if not isinstance(receipt, Mapping):
        _fail("research execution receipt must be a JSON object")
    if (
        receipt.get("schema_version") != MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION
        or receipt.get("execution_authority") != "kyuubiki-headless-sdk"
    ):
        _fail("unsupported or untrusted research execution receipt")
    if not all(
        isinstance(receipt.get(key), str) and receipt[key].strip()
        for key in ("session_id", "workflow_id")
    ) or not _valid_plan_digest(receipt.get("plan_digest")) or not isinstance(
        receipt.get("records"), list
    ) or not receipt["records"]:
        _fail("research execution receipt is incomplete")
    if receipt.get("status") not in {"completed", "failed"}:
        _fail("research execution receipt has an unsupported status")
    final_record = receipt["records"][-1]
    if not isinstance(final_record, Mapping):
        _fail("research execution receipt has an invalid final record")
    if receipt["status"] == "completed" and (
        final_record.get("error") is not None
        or final_record.get("output") is None
        or not isinstance(final_record.get("authority"), str)
    ):
        _fail("completed research receipt has an invalid final record")
    if receipt["status"] == "failed" and not isinstance(final_record.get("error"), str):
        _fail("failed research receipt has no final error")


def validate_model_research_frontier(current: Mapping[str, Any]) -> None:
    if not isinstance(current, Mapping):
        _fail("research frontier must be a JSON object")
    if (
        current.get("schema_version") != MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION
        or not isinstance(current.get("session_id"), str)
        or not current["session_id"].strip()
        or not isinstance(current.get("workflow_id"), str)
        or not current["workflow_id"].strip()
        or not _valid_plan_digest(current.get("origin_plan_digest"))
        or not isinstance(current.get("evidence"), Mapping)
        or not _valid_plan_digest(current["evidence"].get("plan_digest"))
        or not _valid_evidence(current["evidence"])
        or not isinstance(current.get("transition_count"), int)
        or isinstance(current["transition_count"], bool)
        or current["transition_count"] < 1
    ):
        _fail("research frontier is incomplete or uses an unsupported schema")
    job_id = current.get("job_id")
    has_job = isinstance(job_id, str) and bool(job_id.strip())
    stage = current.get("stage")
    next_action = current.get("next_action")
    blocking_reason = current.get("blocking_reason")
    valid_state = (
        stage == "waiting_for_job"
        and has_job
        and next_action == "job_wait"
        and blocking_reason is None
    ) or (
        stage == "ready_to_fetch_result"
        and has_job
        and next_action == "result_fetch"
        and blocking_reason is None
    ) or (
        stage == "ready_to_validate"
        and has_job
        and next_action is None
        and blocking_reason is None
    ) or (
        stage == "blocked"
        and next_action is None
        and isinstance(blocking_reason, str)
        and bool(blocking_reason.strip())
    )
    if not valid_state:
        _fail("research frontier stage and next action are inconsistent")


def _verify_receipt(receipt: Mapping[str, Any], verifier: ReceiptVerifier) -> None:
    if not callable(verifier) or not verifier(receipt):
        _fail("caller receipt verifier rejected research execution receipt")


def _valid_plan_digest(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 71
        and value.startswith("sha256:")
        and all(character in "0123456789abcdef" for character in value[7:])
    )


def _valid_evidence(evidence: Mapping[str, Any]) -> bool:
    action = evidence.get("action")
    record_index = evidence.get("record_index")
    approval_id = evidence.get("approval_id")
    authority = evidence.get("authority")
    return (
        isinstance(action, str)
        and bool(action)
        and action[0].islower()
        and action[0].isascii()
        and all(
            character.isascii()
            and (character.islower() or character.isdigit() or character == "_")
            for character in action
        )
        and isinstance(record_index, int)
        and not isinstance(record_index, bool)
        and record_index > 0
        and (approval_id is None or isinstance(approval_id, str))
        and (authority is None or isinstance(authority, str))
        and evidence.get("job_status") in {None, "completed", "failed", "cancelled"}
    )


def _frontier_digest_projection(current: Mapping[str, Any]) -> dict[str, Any]:
    evidence = current["evidence"]
    return {
        "schema_version": current["schema_version"],
        "session_id": current["session_id"],
        "workflow_id": current["workflow_id"],
        "origin_plan_digest": current["origin_plan_digest"],
        "stage": current["stage"],
        "job_id": current.get("job_id"),
        "next_action": current.get("next_action"),
        "transition_count": current["transition_count"],
        "evidence": {
            "approval_id": evidence.get("approval_id"),
            "plan_digest": evidence["plan_digest"],
            "action": evidence.get("action"),
            "record_index": evidence.get("record_index"),
            "authority": evidence.get("authority"),
            "job_status": evidence.get("job_status"),
        },
        "blocking_reason": current.get("blocking_reason"),
    }


def _verify_frontier(current: Mapping[str, Any], verifier: FrontierVerifier) -> None:
    if not callable(verifier) or not verifier(current):
        _fail("caller frontier verifier rejected research frontier")


def _last_record(receipt: Mapping[str, Any]) -> Mapping[str, Any]:
    record = receipt["records"][-1]
    if not isinstance(record, Mapping):
        _fail("research execution receipt has an invalid final record")
    return record


def _fail(message: str) -> None:
    raise ModelResearchExecutionError([message])
