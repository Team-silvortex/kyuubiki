from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from .errors import ModelResearchBootstrapError
from .model_collaboration import build_model_headless_plan


MODEL_RESEARCH_BOOTSTRAP_SCHEMA_VERSION = "kyuubiki.model-research-bootstrap/v1"
MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION = (
    "kyuubiki.model-research-readiness-report/v1"
)
_SDKS = {"rust", "python", "elixir"}
ResourceResolver = Callable[[str], bool]


def inspect_model_research_bootstrap(
    bootstrap: Mapping[str, Any],
    sdk: str,
    resource_exists: ResourceResolver,
) -> dict[str, Any]:
    """Fail-closed preflight for the document-to-research planning boundary."""
    if sdk not in _SDKS:
        raise ModelResearchBootstrapError(
            ["selected_sdk must be rust, python, or elixir"]
        )
    if not isinstance(bootstrap, Mapping):
        return _empty_report(sdk, "bootstrap must be a JSON object")

    blockers: list[str] = []
    if bootstrap.get("schema_version") != MODEL_RESEARCH_BOOTSTRAP_SCHEMA_VERSION:
        blockers.append(
            f"schema_version must be {MODEL_RESEARCH_BOOTSTRAP_SCHEMA_VERSION}"
        )
    version_line = _text(bootstrap, "version_line") or "unknown"
    entrypoint = _text(bootstrap, "entrypoint") or "unknown"
    first = _mapping(bootstrap.get("first_research"))
    if first is None:
        blockers.append("first_research must be a JSON object")
    workflow_id = _text(first or {}, "workflow_id") or "unknown"
    if _text(first or {}, "reliability_posture") != "screening_only":
        blockers.append("first_research.reliability_posture must be screening_only")

    hard_rules = _string_list(bootstrap.get("hard_rules"))
    if len(hard_rules) < 8:
        blockers.append("hard_rules must contain at least 8 non-empty rules")
    stop_conditions = _string_list(bootstrap.get("stop_conditions"))
    if len(stop_conditions) < 4:
        blockers.append("stop_conditions must contain at least 4 non-empty rules")
    completion = _completion_contract(bootstrap, blockers)
    protocol = bootstrap.get("research_protocol")
    if not isinstance(protocol, list) or len(protocol) < 6:
        blockers.append("research_protocol must contain at least 6 stages")

    resources: set[str] = set()
    _add_path(entrypoint, "entrypoint", resources, blockers)
    _add_document_paths(bootstrap, resources, blockers)
    selected_surface = _selected_surface(bootstrap, sdk, resources, blockers)
    _add_execution_resources(bootstrap, resources, blockers)
    _add_first_resources(first, resources, blockers)
    _add_preflight_resources(bootstrap, resources, blockers)

    required_resources = sorted(resources)
    if not callable(resource_exists):
        blockers.append("resource_exists must be callable")
        missing_resources = required_resources
    else:
        missing_resources = [
            path for path in required_resources if not _resource_exists(resource_exists, path)
        ]
    blockers.extend(f"missing required resource: {path}" for path in missing_resources)
    blockers = sorted(set(blockers))

    return {
        "schema_version": MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION,
        "selected_sdk": sdk,
        "ready_for_planning": not blockers and selected_surface is not None,
        "execution_authority": "none_preflight_only",
        "version_line": version_line,
        "entrypoint": entrypoint,
        "workflow_id": workflow_id,
        "selected_surface": selected_surface,
        "required_resources": required_resources,
        "missing_resources": missing_resources,
        "blockers": blockers,
        "hard_rules": hard_rules,
        "stop_conditions": stop_conditions,
        "completion_contract": completion,
    }


def build_bootstrapped_model_headless_plan(
    readiness: Mapping[str, Any],
    session: Mapping[str, Any],
    proposal: Mapping[str, Any],
) -> dict[str, Any]:
    """Build the first plan only after a Python-bound bootstrap preflight."""
    _validate_readiness_for_plan(readiness)
    if not isinstance(session, Mapping) or session.get("workflow_id") != readiness.get(
        "workflow_id"
    ):
        _fail("collaboration session workflow_id does not match readiness report")
    plan = build_model_headless_plan(session, proposal)
    if not plan["ok"]:
        _fail(*(f"bootstrapped plan: {issue}" for issue in plan["issues"]))
    return plan


def _validate_readiness_for_plan(readiness: Mapping[str, Any]) -> None:
    if not isinstance(readiness, Mapping):
        _fail("readiness report must be a JSON object")
    surface = _mapping(readiness.get("selected_surface"))
    valid_surface = surface is not None and (
        surface.get("preflight_path")
        == "sdks/python/kyuubiki_sdk/model_research_bootstrap.py"
        and surface.get("inspect") == "inspect_model_research_bootstrap"
        and surface.get("bootstrap_plan")
        == "build_bootstrapped_model_headless_plan"
    )
    if (
        readiness.get("schema_version")
        != MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION
        or readiness.get("selected_sdk") != "python"
        or readiness.get("ready_for_planning") is not True
        or readiness.get("execution_authority") != "none_preflight_only"
        or readiness.get("missing_resources") != []
        or readiness.get("blockers") != []
        or len(_string_list(readiness.get("hard_rules"))) < 8
        or len(_string_list(readiness.get("stop_conditions"))) < 4
        or not isinstance(readiness.get("completion_contract"), Mapping)
        or not valid_surface
    ):
        _fail("readiness report is not valid for Python planning")


def _selected_surface(
    bootstrap: Mapping[str, Any],
    sdk: str,
    resources: set[str],
    blockers: list[str],
) -> dict[str, str] | None:
    collaboration = _nested_mapping(bootstrap, "sdk_surfaces", sdk)
    preflight = _nested_mapping(_mapping(bootstrap.get("preflight")) or {}, "surfaces", sdk)
    execution_root = _mapping(bootstrap.get("execution_contract"))
    if _text(execution_root or {}, "approval_authority") != "caller_only":
        blockers.append("execution_contract.approval_authority must be caller_only")
    execution = _nested_mapping(execution_root or {}, "surfaces", sdk)
    if collaboration is None or preflight is None or execution is None:
        blockers.append(f"selected SDK surface is missing: {sdk}")
        return None

    values: dict[str, str] = {}
    field_sources = {
        "collaboration_path": (collaboration, "path", f"sdk_surfaces.{sdk}.path"),
        "preflight_path": (preflight, "path", f"preflight.surfaces.{sdk}.path"),
        "execution_path": (
            execution,
            "path",
            f"execution_contract.surfaces.{sdk}.path",
        ),
        "approval_path": (
            execution,
            "approval_path",
            f"execution_contract.surfaces.{sdk}.approval_path",
        ),
        "frontier_path": (
            execution,
            "frontier_path",
            f"execution_contract.surfaces.{sdk}.frontier_path",
        ),
        "validation_path": (
            execution,
            "validation_path",
            f"execution_contract.surfaces.{sdk}.validation_path",
        ),
        "request": (collaboration, "request", f"sdk_surfaces.{sdk}.request"),
        "inspect": (preflight, "inspect", f"preflight.surfaces.{sdk}.inspect"),
        "bootstrap_plan": (
            preflight,
            "build_plan",
            f"preflight.surfaces.{sdk}.build_plan",
        ),
        "normalize": (collaboration, "normalize", f"sdk_surfaces.{sdk}.normalize"),
        "plan": (collaboration, "plan", f"sdk_surfaces.{sdk}.plan"),
        "executor": (execution, "executor", f"execution_contract.surfaces.{sdk}.executor"),
        "dispatcher": (execution, "dispatcher", f"execution_contract.surfaces.{sdk}.dispatcher"),
        "approval_verifier": (execution, "approval_verifier", f"execution_contract.surfaces.{sdk}.approval_verifier"),
        "plan_digest": (execution, "plan_digest", f"execution_contract.surfaces.{sdk}.plan_digest"),
        "approval_request": (execution, "approval_request", f"execution_contract.surfaces.{sdk}.approval_request"),
        "frontier_start": (execution, "frontier_start", f"execution_contract.surfaces.{sdk}.frontier_start"),
        "frontier_advance": (execution, "frontier_advance", f"execution_contract.surfaces.{sdk}.frontier_advance"),
        "result_validator": (execution, "result_validator", f"execution_contract.surfaces.{sdk}.result_validator"),
        "receipt_verifier": (execution, "receipt_verifier", f"execution_contract.surfaces.{sdk}.receipt_verifier"),
        "frontier_verifier": (execution, "frontier_verifier", f"execution_contract.surfaces.{sdk}.frontier_verifier"),
    }
    for output_key, (source, source_key, label) in field_sources.items():
        value = _text(source, source_key)
        if value is None:
            blockers.append(f"{label} must be a non-empty string")
        else:
            values[output_key] = value
    if len(values) != len(field_sources):
        return None
    for key in (
        "collaboration_path",
        "preflight_path",
        "execution_path",
        "approval_path",
        "frontier_path",
        "validation_path",
    ):
        _add_path(values[key], f"selected_surface.{key}", resources, blockers)
    return values


def _add_document_paths(
    bootstrap: Mapping[str, Any], resources: set[str], blockers: list[str]
) -> None:
    documents = bootstrap.get("required_documents")
    if not isinstance(documents, list):
        blockers.append("required_documents must be an array")
        return
    if len(documents) < 4:
        blockers.append("required_documents must contain at least 4 entries")
    for index, document in enumerate(documents):
        path = _text(_mapping(document) or {}, "path") or ""
        _add_path(path, f"required_documents[{index}].path", resources, blockers)


def _add_execution_resources(
    bootstrap: Mapping[str, Any], resources: set[str], blockers: list[str]
) -> None:
    execution = _mapping(bootstrap.get("execution_contract"))
    if execution is None:
        blockers.append("execution_contract must be a JSON object")
        return
    for key in (
        "approval_request_schema",
        "approval_request_fixture",
        "approval_schema",
        "approval_fixture",
        "receipt_schema",
        "frontier_schema",
        "frontier_fixture",
        "validation_report_schema",
        "validation_report_fixture",
    ):
        _add_path(_text(execution, key) or "", f"execution_contract.{key}", resources, blockers)


def _add_first_resources(
    first: Mapping[str, Any] | None, resources: set[str], blockers: list[str]
) -> None:
    if first is None:
        return
    for key in ("session_fixture", "proposal_fixture", "catalog_request_fixture"):
        _add_path(_text(first, key) or "", f"first_research.{key}", resources, blockers)


def _add_preflight_resources(
    bootstrap: Mapping[str, Any], resources: set[str], blockers: list[str]
) -> None:
    preflight = _mapping(bootstrap.get("preflight"))
    if preflight is None:
        blockers.append("preflight must be a JSON object")
        return
    if _text(preflight, "execution_authority") != "none_preflight_only":
        blockers.append("preflight.execution_authority must be none_preflight_only")
    for key in ("report_schema", "report_fixture"):
        _add_path(_text(preflight, key) or "", f"preflight.{key}", resources, blockers)


def _add_path(path: str, label: str, resources: set[str], blockers: list[str]) -> None:
    if _safe_repo_path(path):
        resources.add(path)
    else:
        blockers.append(f"{label} must be a safe project-relative path")


def _safe_repo_path(path: str) -> bool:
    return bool(path) and not path.startswith("/") and "\\" not in path and all(
        part not in {"", ".."} for part in path.split("/")
    )


def _resource_exists(resolver: ResourceResolver, path: str) -> bool:
    try:
        return resolver(path) is True
    except Exception:
        return False


def _mapping(value: Any) -> Mapping[str, Any] | None:
    return value if isinstance(value, Mapping) else None


def _nested_mapping(root: Mapping[str, Any], outer: str, inner: str) -> Mapping[str, Any] | None:
    parent = _mapping(root.get(outer))
    return _mapping(parent.get(inner)) if parent is not None else None


def _text(root: Mapping[str, Any], key: str) -> str | None:
    value = root.get(key)
    return value.strip() if isinstance(value, str) and value.strip() else None


def _string_list(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []
    return [item.strip() for item in value if isinstance(item, str) and item.strip()]


def _completion_contract(
    bootstrap: Mapping[str, Any], blockers: list[str]
) -> dict[str, Any] | None:
    completion = _mapping(bootstrap.get("completion_contract"))
    if completion is None:
        blockers.append("completion_contract must be a JSON object")
        return None
    for key, minimum in (
        ("required_artifacts", 3),
        ("required_claims", 3),
        ("forbidden_claims", 2),
    ):
        if len(_string_list(completion.get(key))) < minimum:
            blockers.append(
                f"completion_contract.{key} must contain at least {minimum} entries"
            )
    return dict(completion)


def _empty_report(sdk: str, blocker: str) -> dict[str, Any]:
    return {
        "schema_version": MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION,
        "selected_sdk": sdk,
        "ready_for_planning": False,
        "execution_authority": "none_preflight_only",
        "version_line": "unknown",
        "entrypoint": "unknown",
        "workflow_id": "unknown",
        "selected_surface": None,
        "required_resources": [],
        "missing_resources": [],
        "blockers": [blocker],
        "hard_rules": [],
        "stop_conditions": [],
        "completion_contract": None,
    }


def _fail(*messages: str) -> None:
    raise ModelResearchBootstrapError(list(messages))
