from __future__ import annotations

import hashlib
import json
import math
from collections.abc import Mapping
from typing import Any

from .errors import ModelResearchExecutionError
from .model_collaboration import MODEL_HEADLESS_PLAN_SCHEMA_VERSION


MODEL_PLAN_APPROVAL_REQUEST_SCHEMA_VERSION = "kyuubiki.model-plan-approval-request/v1"
MODEL_PLAN_APPROVAL_SCHEMA_VERSION = "kyuubiki.model-plan-approval/v2"


def compute_model_headless_plan_digest(plan: Mapping[str, Any]) -> str:
    if not isinstance(plan, Mapping):
        _fail("model plan must be a JSON object")
    return _compute_canonical_json_digest(plan)


def _compute_canonical_json_digest(value: Any) -> str:
    canonical = _canonical_json(value)
    return f"sha256:{hashlib.sha256(canonical.encode('utf-8')).hexdigest()}"


def build_model_plan_approval_request(plan: Mapping[str, Any]) -> dict[str, Any]:
    _validate_plan(plan)
    required_steps = []
    for step in plan["steps"]:
        if not step.get("requires_confirmation"):
            continue
        reason = step.get("confirmation_reason")
        if not isinstance(reason, str) or not reason.strip():
            _fail(f"gated model plan step {step['index']} requires a confirmation_reason")
        risk = step.get("risk")
        if risk not in {"sensitive", "destructive"}:
            _fail(f"gated model plan step {step['index']} has invalid risk")
        required_steps.append(
            {
                "index": step["index"],
                "action": step["action"],
                "risk": risk,
                "confirmation_reason": reason,
            }
        )
    return {
        "schema_version": MODEL_PLAN_APPROVAL_REQUEST_SCHEMA_VERSION,
        "plan_schema_version": plan["schema_version"],
        "plan_digest": compute_model_headless_plan_digest(plan),
        "session_id": plan["session_id"],
        "workflow_id": plan["workflow_id"],
        "status": "approval_required" if required_steps else "not_required",
        "execution_authority": "none_approval_request_only",
        "approval_schema_version": MODEL_PLAN_APPROVAL_SCHEMA_VERSION,
        "required_steps": required_steps,
    }


def _validate_plan(plan: Mapping[str, Any]) -> None:
    if not isinstance(plan, Mapping):
        _fail("model plan must be a JSON object")
    errors: list[str] = []
    if plan.get("schema_version") != MODEL_HEADLESS_PLAN_SCHEMA_VERSION:
        errors.append(f"unsupported model plan schema_version: {plan.get('schema_version')}")
    if not plan.get("ok") or plan.get("issues"):
        errors.append("model plan must be valid and issue-free before approval")
    if not isinstance(plan.get("session_id"), str) or not plan["session_id"].strip():
        errors.append("model plan session_id and workflow_id are required")
    if not isinstance(plan.get("workflow_id"), str) or not plan["workflow_id"].strip():
        errors.append("model plan session_id and workflow_id are required")
    steps = plan.get("steps")
    if not isinstance(steps, list) or not steps:
        errors.append("model plan contains no steps")
        steps = []
    if any(
        not isinstance(step, Mapping) or step.get("index") != index
        for index, step in enumerate(steps, start=1)
    ):
        errors.append("model plan step indexes must be contiguous and one-based")
    if errors:
        raise ModelResearchExecutionError(sorted(set(errors)))


def _canonical_json(value: Any) -> str:
    if isinstance(value, Mapping):
        if any(not isinstance(key, str) for key in value):
            _fail("model plan JSON object keys must be strings")
        parts = (
            f"{json.dumps(key, ensure_ascii=False)}:{_canonical_json(value[key])}"
            for key in sorted(value)
        )
        return "{" + ",".join(parts) + "}"
    if isinstance(value, list):
        return "[" + ",".join(_canonical_json(item) for item in value) + "]"
    if value is None or isinstance(value, (bool, str)):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if not math.isfinite(value):
            _fail("model plan contains a non-finite JSON number")
        encoded = f"{value:.15f}".rstrip("0")
        return f"{encoded}0" if encoded.endswith(".") else encoded
    _fail(f"model plan contains non-JSON value: {type(value).__name__}")


def _fail(message: str) -> None:
    raise ModelResearchExecutionError([message])
