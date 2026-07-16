from __future__ import annotations

from typing import Any

_RECEIPT_KEYS = ("failure_receipt", "operator_task_failure_receipt")
_RECEIPT_SCHEMAS = (
    "kyuubiki.headless-operator-task-failure/v1",
    "kyuubiki.agent-operator-task-failure/v1",
    "kyuubiki.control-plane-operator-task-failure/v1",
)


def extract_operator_task_failure_receipts(payload: Any) -> list[dict[str, Any]]:
    """Return all Operator TaskIR failure receipts nested in a response payload."""

    receipts: list[dict[str, Any]] = []
    _collect_failure_receipts(payload, receipts)
    return _unique_receipts(receipts)


def operator_task_failure_actions(payload: Any) -> list[str]:
    """Return unique recovery actions from receipts and resume plans."""

    actions: list[str] = []
    for receipt in extract_operator_task_failure_receipts(payload):
        _append_recovery_action(actions, receipt.get("recovery", {}).get("required_action"))
    _collect_recovery_actions(payload, actions)
    return actions


def operator_task_recovery_summary(payload: Any) -> dict[str, Any]:
    """Return a compact recovery summary for checkpoint/resume automation."""

    receipts = extract_operator_task_failure_receipts(payload)
    return {
        "next_action": _first_string_value(payload, "next_action"),
        "target_case_ids": _first_string_list_value(payload, "target_case_ids"),
        "blocked_case_ids": _first_string_list_value(payload, "blocked_case_ids"),
        "recovery_actions": operator_task_failure_actions(payload),
        "failure_receipt_count": len(receipts),
        "failure_receipts": receipts,
    }


def _collect_failure_receipts(value: Any, receipts: list[dict[str, Any]]) -> None:
    if isinstance(value, dict):
        for key in _RECEIPT_KEYS:
            candidate = value.get(key)
            if _is_failure_receipt(candidate):
                receipts.append(candidate)
        if _is_failure_receipt(value):
            receipts.append(value)
        for child in value.values():
            _collect_failure_receipts(child, receipts)
    elif isinstance(value, list):
        for child in value:
            _collect_failure_receipts(child, receipts)


def _collect_recovery_actions(value: Any, actions: list[str]) -> None:
    if isinstance(value, dict):
        candidates = value.get("recovery_actions")
        if isinstance(candidates, list):
            for action in candidates:
                _append_recovery_action(actions, action)
        for child in value.values():
            _collect_recovery_actions(child, actions)
    elif isinstance(value, list):
        for child in value:
            _collect_recovery_actions(child, actions)


def _append_recovery_action(actions: list[str], action: Any) -> None:
    if isinstance(action, str) and action and action not in actions:
        actions.append(action)


def _first_string_value(value: Any, key: str) -> str | None:
    if isinstance(value, dict):
        candidate = value.get(key)
        if isinstance(candidate, str) and candidate:
            return candidate
        for child in value.values():
            found = _first_string_value(child, key)
            if found is not None:
                return found
    elif isinstance(value, list):
        for child in value:
            found = _first_string_value(child, key)
            if found is not None:
                return found
    return None


def _first_string_list_value(value: Any, key: str) -> list[str]:
    if isinstance(value, dict):
        candidate = value.get(key)
        if isinstance(candidate, list):
            return [item for item in candidate if isinstance(item, str)]
        for child in value.values():
            found = _first_string_list_value(child, key)
            if found:
                return found
    elif isinstance(value, list):
        for child in value:
            found = _first_string_list_value(child, key)
            if found:
                return found
    return []


def _is_failure_receipt(value: Any) -> bool:
    return isinstance(value, dict) and value.get("schema_version") in _RECEIPT_SCHEMAS


def _unique_receipts(receipts: list[dict[str, Any]]) -> list[dict[str, Any]]:
    unique: list[dict[str, Any]] = []
    seen: set[tuple[Any, ...]] = set()
    for receipt in receipts:
        key = (
            receipt.get("schema_version"),
            receipt.get("failure_stage"),
            receipt.get("reason_code"),
            receipt.get("task_id"),
            receipt.get("operator_id"),
            receipt.get("task_digest"),
        )
        if key in seen:
            continue
        seen.add(key)
        unique.append(receipt)
    return unique
