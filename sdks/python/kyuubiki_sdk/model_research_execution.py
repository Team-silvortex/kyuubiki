from __future__ import annotations

import copy
from collections.abc import Callable, Mapping
from typing import Any, Protocol

from .errors import ModelResearchExecutionError
from .model_collaboration import MODEL_HEADLESS_PLAN_SCHEMA_VERSION
from .model_plan_approval import (
    MODEL_PLAN_APPROVAL_SCHEMA_VERSION,
    compute_model_headless_plan_digest,
)
from .session import KyuubikiSession


MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION = "kyuubiki.model-research-execution-receipt/v2"
ApprovalVerifier = Callable[[Mapping[str, Any], Mapping[str, Any]], bool]


class ModelActionDispatcher(Protocol):
    def dispatch_model_action(
        self, action: str, payload: Mapping[str, Any]
    ) -> dict[str, Any]: ...


class SessionModelActionDispatcher:
    def __init__(
        self,
        session: KyuubikiSession,
        *,
        poll_interval_s: float = 0.5,
        timeout_s: float = 300.0,
    ) -> None:
        if poll_interval_s <= 0 or timeout_s <= 0 or poll_interval_s > timeout_s:
            _fail("model dispatcher wait bounds require 0 < poll_interval_s <= timeout_s")
        if timeout_s > 86_400:
            _fail("model dispatcher timeout cannot exceed 24 hours")
        self.session = session
        self.poll_interval_s = poll_interval_s
        self.timeout_s = timeout_s

    def dispatch_model_action(
        self, action: str, payload: Mapping[str, Any]
    ) -> dict[str, Any]:
        control_plane = self.session.control_plane
        if action == "direct_solver_rpc":
            output = self.session.solve_direct(
                _required_string(payload, "solve_kind"),
                _required_mapping(payload, "payload"),
            )
            return {"authority": "solver_rpc", "output": output}
        if control_plane is None:
            raise RuntimeError("control plane client is not configured")

        if action == "service_health":
            output = control_plane.health()
        elif action == "protocol_describe":
            output = control_plane.protocol()
        elif action == "agents_describe":
            output = control_plane.agents()
        elif action == "workflow_catalog_list":
            output = control_plane.list_workflow_catalog()
        elif action == "operator_catalog_list":
            output = control_plane.list_workflow_operators()
        elif action == "fem_submit":
            output = control_plane.submit_fem_job(
                _required_string(payload, "solve_kind"),
                _required_mapping(payload, "payload"),
            )
        elif action == "workflow_submit_catalog":
            output = control_plane.submit_workflow_catalog_job(
                _required_string(payload, "workflow_id"),
                _required_mapping(payload, "input_artifacts"),
            )
        elif action == "workflow_submit_graph":
            output = control_plane.submit_workflow_graph_job(
                _required_mapping(payload, "graph"),
                _required_mapping(payload, "input_artifacts"),
            )
        elif action == "operator_task_prepare":
            output = control_plane.prepare_operator_task(_required_mapping(payload, "task"))
        elif action == "operator_task_execute":
            output = control_plane.execute_operator_task(_required_mapping(payload, "task"))
        elif action == "operator_task_batch_prepare":
            output = control_plane.prepare_operator_task_batch(
                _required_mapping(payload, "batch")
            )
        elif action == "operator_task_batch_execute":
            output = control_plane.execute_operator_task_batch(
                _required_mapping(payload, "batch")
            )
        elif action == "job_wait":
            output = self.session.wait_for_job(
                _required_string(payload, "job_id"),
                poll_interval_s=self.poll_interval_s,
                timeout_s=self.timeout_s,
            )
        elif action == "result_fetch":
            output = control_plane.fetch_result(_required_string(payload, "job_id"))
        elif action == "result_chunk_fetch":
            output = control_plane.fetch_result_chunk(
                _required_string(payload, "job_id"),
                _required_string(payload, "kind"),
                offset=_optional_unsigned(payload, "offset"),
                limit=_optional_unsigned(payload, "limit"),
            )
        elif action == "job_cancel":
            output = control_plane.cancel_job(_required_string(payload, "job_id"))
        else:
            _fail(f"unsupported model action: {action}")
        return {"authority": "control_plane", "output": output}


def execute_model_headless_plan(
    dispatcher: ModelActionDispatcher,
    plan: Mapping[str, Any],
    approval: Mapping[str, Any] | None,
    approval_verifier: ApprovalVerifier,
) -> dict[str, Any]:
    plan_digest = _validate_execution_request(plan, approval)
    if approval is not None and not approval_verifier(plan, approval):
        _fail("caller approval verifier rejected approval")
    execution_plan = copy.deepcopy(plan)
    if compute_model_headless_plan_digest(execution_plan) != plan_digest:
        _fail("model plan changed after approval verification")

    records: list[dict[str, Any]] = []
    for step in execution_plan["steps"]:
        job_id = _action_job_id(step["action"], step["payload"])
        try:
            dispatched = dispatcher.dispatch_model_action(step["action"], step["payload"])
            records.append(
                {
                    "index": step["index"],
                    "action": step["action"],
                    "job_id": job_id,
                    "authority": dispatched["authority"],
                    "output": dispatched["output"],
                    "error": None,
                }
            )
        except Exception as error:
            records.append(
                {
                    "index": step["index"],
                    "action": step["action"],
                    "job_id": job_id,
                    "authority": None,
                    "output": None,
                    "error": _bounded_error(error),
                }
            )
            return _receipt(
                execution_plan, plan_digest, approval, "failed", step["index"], records
            )
    return _receipt(execution_plan, plan_digest, approval, "completed", None, records)


def _action_job_id(action: str, payload: Mapping[str, Any]) -> str | None:
    if action not in {"job_wait", "result_fetch", "result_chunk_fetch", "job_cancel"}:
        return None
    value = payload.get("job_id")
    return value if isinstance(value, str) else None


def _validate_execution_request(
    plan: Mapping[str, Any], approval: Mapping[str, Any] | None
) -> str:
    errors: list[str] = []
    if not isinstance(plan, Mapping):
        _fail("model plan must be a JSON object")
    if plan.get("schema_version") != MODEL_HEADLESS_PLAN_SCHEMA_VERSION:
        errors.append(f"unsupported model plan schema_version: {plan.get('schema_version')}")
    if not plan.get("ok") or plan.get("issues"):
        errors.append("model plan must be valid and issue-free before dispatch")
    steps = plan.get("steps")
    if not isinstance(steps, list) or not steps:
        errors.append("model plan contains no steps")
        steps = []
    if any(
        not isinstance(step, Mapping) or step.get("index") != index
        for index, step in enumerate(steps, start=1)
    ):
        errors.append("model plan step indexes must be contiguous and one-based")

    plan_digest = compute_model_headless_plan_digest(plan)
    gated = {
        (step["index"], step["action"])
        for step in steps
        if isinstance(step, Mapping) and step.get("requires_confirmation")
    }
    approved = (
        _validate_approval(plan, plan_digest, approval, gated, errors) if approval else set()
    )
    for index, action in gated - approved:
        errors.append(f"step {index} ({action}) requires an exact caller-issued approval")
    if errors:
        raise ModelResearchExecutionError(sorted(set(errors)))
    return plan_digest


def _validate_approval(
    plan: Mapping[str, Any],
    plan_digest: str,
    approval: Mapping[str, Any],
    gated: set[tuple[int, str]],
    errors: list[str],
) -> set[tuple[int, str]]:
    if not isinstance(approval, Mapping):
        errors.append("model approval must be a JSON object")
        return set()
    if approval.get("schema_version") != MODEL_PLAN_APPROVAL_SCHEMA_VERSION:
        errors.append(
            f"unsupported model approval schema_version: {approval.get('schema_version')}"
        )
    if (
        approval.get("session_id") != plan.get("session_id")
        or approval.get("workflow_id") != plan.get("workflow_id")
    ):
        errors.append("model approval does not match plan session and workflow")
    if approval.get("plan_digest") != plan_digest:
        errors.append("model approval plan_digest does not match the complete plan")
    for key in ("approval_id", "authority", "issued_at"):
        if not isinstance(approval.get(key), str) or not approval[key].strip():
            errors.append(f"model approval {key} is required")

    approved: set[tuple[int, str]] = set()
    steps = approval.get("approved_steps")
    if not isinstance(steps, list):
        errors.append("model approval approved_steps must be an array")
        return approved
    for step in steps:
        if not isinstance(step, Mapping):
            errors.append("model approval steps must be JSON objects")
            continue
        key = (step.get("index"), step.get("action"))
        if not isinstance(key[0], int) or isinstance(key[0], bool) or not isinstance(key[1], str):
            errors.append("model approval step requires integer index and string action")
            continue
        typed_key = (key[0], key[1])
        if typed_key in approved:
            errors.append(f"model approval repeats step {typed_key[0]} ({typed_key[1]})")
        approved.add(typed_key)
        if typed_key not in gated:
            errors.append(
                f"model approval references a non-gated or mismatched step {typed_key[0]} "
                f"({typed_key[1]})"
            )
    return approved


def _receipt(
    plan: Mapping[str, Any],
    plan_digest: str,
    approval: Mapping[str, Any] | None,
    status: str,
    failed_step: int | None,
    records: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema_version": MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION,
        "plan_schema_version": plan["schema_version"],
        "session_id": plan["session_id"],
        "workflow_id": plan["workflow_id"],
        "plan_digest": plan_digest,
        "status": status,
        "execution_authority": "kyuubiki-headless-sdk",
        "approval_id": approval.get("approval_id") if approval else None,
        "completed_steps": sum(record["error"] is None for record in records),
        "failed_step": failed_step,
        "records": records,
    }


def _required_string(payload: Mapping[str, Any], key: str) -> str:
    value = payload.get(key)
    if not isinstance(value, str) or not value.strip():
        _fail(f"model action payload requires non-empty string {key}")
    return value


def _required_mapping(payload: Mapping[str, Any], key: str) -> dict[str, Any]:
    value = payload.get(key)
    if not isinstance(value, Mapping):
        _fail(f"model action payload {key} must be a JSON object")
    return dict(value)


def _optional_unsigned(payload: Mapping[str, Any], key: str) -> int | None:
    value = payload.get(key)
    if value is None:
        return None
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        _fail(f"model action payload {key} must be an unsigned integer")
    return value


def _bounded_error(error: Exception) -> str:
    message = str(error)
    encoded = message.encode("utf-8")
    if len(encoded) <= 2_048:
        return message
    return f"{encoded[:2_048].decode('utf-8', errors='ignore')}..."


def _fail(message: str) -> None:
    raise ModelResearchExecutionError([message])
