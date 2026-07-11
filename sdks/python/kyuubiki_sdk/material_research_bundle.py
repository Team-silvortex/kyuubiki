from __future__ import annotations

from typing import Any


MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION = "kyuubiki.material-research-bundle/v1"
_POSTURE = "screening_research_bundle"
_EXPLORATION_SCHEMA_VERSION = "kyuubiki.material-exploration-run/v1"
_NEXT_ROUND_EXECUTION_SCHEMA_VERSION = (
    "kyuubiki.material-exploration-next-round-execution/v1"
)
_CHAIN_SCHEMA_VERSION = "kyuubiki.material-exploration-chain/v1"


def validate_material_research_bundle(bundle: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(bundle, dict):
        raise ValueError("material research bundle must be an object")
    _require_equal(
        bundle.get("schema_version"),
        MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION,
        "schema_version",
    )
    _require_equal(bundle.get("posture"), _POSTURE, "posture")
    _require_string(bundle.get("bundle_id"), "bundle_id")
    _require_string(bundle.get("generated_at_utc"), "generated_at_utc")
    _require_string(bundle.get("study"), "study")
    _validate_checksums(_require_mapping(bundle.get("artifact_checksums"), "artifact_checksums"))
    _validate_reproducibility(_require_mapping(bundle.get("reproducibility"), "reproducibility"))
    _require_artifact_schema(
        bundle.get("initial_exploration"),
        _EXPLORATION_SCHEMA_VERSION,
        "initial_exploration",
    )
    _require_artifact_schema(
        bundle.get("next_round_execution_plan"),
        _NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
        "next_round_execution_plan",
    )
    _require_artifact_schema(
        bundle.get("next_exploration"),
        _EXPLORATION_SCHEMA_VERSION,
        "next_exploration",
    )
    _require_artifact_schema(bundle.get("chain"), _CHAIN_SCHEMA_VERSION, "chain")
    summary = _require_mapping(bundle.get("summary"), "summary")
    _validate_summary_artifact_consistency(bundle, summary)
    _require_string(summary.get("winner_candidate_id"), "summary.winner_candidate_id")
    _require_string(summary.get("reliability_decision"), "summary.reliability_decision")
    _require_string(summary.get("next_round_decision"), "summary.next_round_decision")
    _require_string(summary.get("chain_stop_reason"), "summary.chain_stop_reason")
    return bundle


def _validate_summary_artifact_consistency(
    bundle: dict[str, Any], summary: dict[str, Any]
) -> None:
    plan = _require_mapping(
        bundle.get("next_round_execution_plan"),
        "next_round_execution_plan",
    )
    next_exploration = _require_mapping(bundle.get("next_exploration"), "next_exploration")
    chain = _require_mapping(bundle.get("chain"), "chain")
    _require_equal(
        plan.get("decision"),
        summary.get("next_round_decision"),
        "next_round_execution_plan.decision",
    )
    if "runnable_next_step_count" in summary:
        _require_equal(
            plan.get("runnable_step_count"),
            summary.get("runnable_next_step_count"),
            "next_round_execution_plan.runnable_step_count",
        )
    if "next_iteration" in summary:
        _require_equal(
            plan.get("iteration"),
            summary.get("next_iteration"),
            "next_round_execution_plan.iteration",
        )
        _require_equal(
            next_exploration.get("iteration"),
            summary.get("next_iteration"),
            "next_exploration.iteration",
        )
    _require_equal(
        chain.get("stop_reason"),
        summary.get("chain_stop_reason"),
        "chain.stop_reason",
    )


def _validate_checksums(checksums: dict[str, Any]) -> None:
    for key in (
        "initial_exploration_sha256",
        "next_round_execution_plan_sha256",
        "next_exploration_sha256",
        "chain_sha256",
    ):
        _require_sha256(checksums.get(key), f"artifact_checksums.{key}")


def _validate_reproducibility(reproducibility: dict[str, Any]) -> None:
    _require_string(reproducibility.get("workspace"), "reproducibility.workspace")
    for key in (
        "initial_command",
        "plan_next_command_template",
        "run_next_command_template",
        "chain_next_command_template",
    ):
        _require_argv(reproducibility.get(key), f"reproducibility.{key}")


def _require_artifact_schema(value: Any, expected: str, field: str) -> None:
    artifact = _require_mapping(value, field)
    _require_equal(artifact.get("schema_version"), expected, f"{field}.schema_version")


def _require_equal(value: Any, expected: str, field: str) -> None:
    if value != expected:
        raise ValueError(f"{field} must be {expected}, got {value!r}")


def _require_mapping(value: Any, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{field} must be an object")
    return value


def _require_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty string")
    return value


def _require_argv(value: Any, field: str) -> None:
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise ValueError(f"{field} must be a non-empty argv array")


def _require_sha256(value: Any, field: str) -> None:
    if not isinstance(value, str) or len(value) != 64 or any(
        char not in "0123456789abcdef" for char in value
    ):
        raise ValueError(f"{field} must be a lowercase SHA-256 hex digest")
