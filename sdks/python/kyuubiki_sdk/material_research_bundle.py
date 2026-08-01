from __future__ import annotations

from typing import Any


MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION = "kyuubiki.material-research-bundle/v1"
_POSTURE = "screening_research_bundle"
_EXPLORATION_SCHEMA_VERSION = "kyuubiki.material-exploration-run/v1"
_NEXT_ROUND_EXECUTION_SCHEMA_VERSION = (
    "kyuubiki.material-exploration-next-round-execution/v1"
)
_CHAIN_SCHEMA_VERSION = "kyuubiki.material-exploration-chain/v1"
_AUTHORITY_TRACE_SCHEMA_VERSION = "kyuubiki.research-execution-authority-trace/v1"
_EXECUTION_AUTHORITY_SCHEMA_VERSION = "kyuubiki.execution-authority/v1"
_RESEARCH_EVIDENCE_SCHEMA_VERSION = "kyuubiki.material-research-evidence/v1"
_VALIDATION_EVIDENCE_SCHEMA_VERSION = "kyuubiki.material-validation-evidence/v1"


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
    _validate_execution_trace(_require_mapping(bundle.get("execution_trace"), "execution_trace"))
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
    _validate_research_evidence(
        bundle,
        _require_mapping(bundle.get("research_evidence"), "research_evidence"),
        summary,
    )
    _validate_validation_evidence(
        _require_mapping(bundle.get("validation_evidence"), "validation_evidence"),
        _require_mapping(bundle.get("research_evidence"), "research_evidence"),
    )
    _validate_material_card_refs(summary, bundle["research_evidence"])
    return bundle


def _validate_execution_trace(trace: dict[str, Any]) -> None:
    for key in (
        "initial_duration_ms",
        "plan_next_duration_ms",
        "run_next_duration_ms",
        "chain_next_duration_ms",
    ):
        _require_non_negative_int(trace.get(key), f"execution_trace.{key}")
    authority = _require_mapping(trace.get("authority"), "execution_trace.authority")
    _require_equal(
        authority.get("schema_version"),
        _AUTHORITY_TRACE_SCHEMA_VERSION,
        "execution_trace.authority.schema_version",
    )
    assertions = _require_mapping(
        authority.get("assertions"), "execution_trace.authority.assertions"
    )
    for key in ("all_real_solver", "no_mock_execution", "no_fallback"):
        _require_equal(assertions.get(key), True, f"execution_trace.authority.assertions.{key}")
    for key in ("initial", "next"):
        _validate_real_solver_authority(
            _require_mapping(authority.get(key), f"execution_trace.authority.{key}"),
            f"execution_trace.authority.{key}",
        )
    chain = _require_list(authority.get("chain"), "execution_trace.authority.chain")
    if not chain:
        raise ValueError("execution_trace.authority.chain must be non-empty")
    for index, item in enumerate(chain):
        field = f"execution_trace.authority.chain[{index}]"
        _validate_real_solver_authority(_require_mapping(item, field), field)


def _validate_real_solver_authority(authority: dict[str, Any], field: str) -> None:
    _require_equal(
        authority.get("schema_version"),
        _EXECUTION_AUTHORITY_SCHEMA_VERSION,
        f"{field}.schema_version",
    )
    _require_equal(authority.get("execution_class"), "real_solver", f"{field}.execution_class")
    _require_equal(authority.get("mock_execution"), False, f"{field}.mock_execution")
    _require_equal(authority.get("fallback_used"), False, f"{field}.fallback_used")
    _require_equal(authority.get("production_eligible"), True, f"{field}.production_eligible")
    for key in ("executor_id", "runtime", "result_origin", "evidence_statement"):
        _require_string(authority.get(key), f"{field}.{key}")


def _validate_research_evidence(
    bundle: dict[str, Any], evidence: dict[str, Any], summary: dict[str, Any]
) -> None:
    _require_equal(
        evidence.get("schema_version"),
        _RESEARCH_EVIDENCE_SCHEMA_VERSION,
        "research_evidence.schema_version",
    )
    ranked = _require_string_list(
        evidence.get("ranked_candidate_ids"), "research_evidence.ranked_candidate_ids"
    )
    _require_unique(ranked, "research_evidence.ranked_candidate_ids")
    candidate_count = _require_positive_int(
        evidence.get("candidate_count"), "research_evidence.candidate_count"
    )
    if candidate_count != len(ranked):
        raise ValueError("research_evidence.candidate_count must match ranked_candidate_ids")
    winner = _require_string(
        evidence.get("winner_candidate_id"), "research_evidence.winner_candidate_id"
    )
    _require_equal(winner, summary.get("winner_candidate_id"), "research_evidence.winner_candidate_id")
    if winner not in ranked:
        raise ValueError("research_evidence winner must be present in ranked candidates")
    focus = _require_string_list(
        evidence.get("focus_candidate_ids"), "research_evidence.focus_candidate_ids"
    )
    _require_unique(focus, "research_evidence.focus_candidate_ids")
    unknown = [candidate for candidate in focus if candidate not in ranked]
    if unknown:
        raise ValueError(
            f"research_evidence.focus_candidate_ids contains unknown candidate {unknown[0]!r}"
        )
    metrics = _require_string_list(
        evidence.get("primary_metric_ids"), "research_evidence.primary_metric_ids"
    )
    _require_unique(metrics, "research_evidence.primary_metric_ids")
    metric_count = _require_positive_int(
        evidence.get("metric_objective_count"), "research_evidence.metric_objective_count"
    )
    if metric_count != len(metrics):
        raise ValueError("research_evidence.metric_objective_count must match primary_metric_ids")
    _require_string_list(
        evidence.get("violated_quality_gate_ids"),
        "research_evidence.violated_quality_gate_ids",
        allow_empty=True,
    )
    _require_equal(
        evidence.get("quality_gate_decision"),
        summary.get("reliability_decision"),
        "research_evidence.quality_gate_decision",
    )
    _require_equal(
        evidence.get("plan_decision"),
        summary.get("next_round_decision"),
        "research_evidence.plan_decision",
    )
    _require_non_negative_int(evidence.get("plan_step_count"), "research_evidence.plan_step_count")
    if "runnable_next_step_count" in summary:
        _require_equal(
            evidence.get("plan_step_count"),
            summary.get("runnable_next_step_count"),
            "research_evidence.plan_step_count",
        )
    _require_positive_int(evidence.get("chain_round_count"), "research_evidence.chain_round_count")
    if "chain_round_count" in summary:
        _require_equal(
            evidence.get("chain_round_count"),
            summary.get("chain_round_count"),
            "research_evidence.chain_round_count",
        )
    trace_count = _require_positive_int(
        evidence.get("chain_trace_round_count"), "research_evidence.chain_trace_round_count"
    )
    trace = _require_mapping(bundle.get("chain"), "chain").get("optimization_trace")
    if isinstance(trace, list) and trace_count != len(trace):
        raise ValueError(
            "research_evidence.chain_trace_round_count must match chain.optimization_trace"
        )
    final_winner = _require_string(
        evidence.get("final_winner_candidate_id"),
        "research_evidence.final_winner_candidate_id",
    )
    chain_final_winner = bundle["chain"].get("final_winner_candidate_id")
    if chain_final_winner is not None:
        _require_equal(
            final_winner,
            chain_final_winner,
            "research_evidence.final_winner_candidate_id",
        )


def _validate_validation_evidence(
    validation: dict[str, Any], research: dict[str, Any]
) -> None:
    _require_equal(
        validation.get("schema_version"),
        _VALIDATION_EVIDENCE_SCHEMA_VERSION,
        "validation_evidence.schema_version",
    )
    _require_equal(
        validation.get("validation_posture"),
        "screening_validation",
        "validation_evidence.validation_posture",
    )
    _validate_object_list(
        validation.get("baseline_refs"),
        "validation_evidence.baseline_refs",
        ("baseline_id", "kind", "status", "scope"),
    )
    _validate_confidence_counts(
        validation.get("candidate_confidence_counts"),
        "validation_evidence.candidate_confidence_counts",
    )
    sensitivity = _require_mapping(
        validation.get("sensitivity_summary"),
        "validation_evidence.sensitivity_summary",
    )
    _require_equal(
        sensitivity.get("schema_version"),
        "kyuubiki.material-sensitivity-summary/v1",
        "validation_evidence.sensitivity_summary.schema_version",
    )
    for key in ("method", "winner_stability_state"):
        _require_string(sensitivity.get(key), f"validation_evidence.sensitivity_summary.{key}")
    for key in ("primary_metric_ids", "focus_candidate_ids"):
        actual = _require_string_list(
            sensitivity.get(key), f"validation_evidence.sensitivity_summary.{key}"
        )
        _require_equal(actual, research.get(key), f"validation_evidence.sensitivity_summary.{key}")
    _require_equal(
        sensitivity.get("chain_trace_round_count"),
        research.get("chain_trace_round_count"),
        "validation_evidence.sensitivity_summary.chain_trace_round_count",
    )
    _validate_object_list(
        validation.get("acceptance_criteria"),
        "validation_evidence.acceptance_criteria",
        ("criterion_id", "metric_id", "operator", "status"),
    )
    _validate_uncertainty_summary(validation)
    _validate_validation_readiness(validation)
    _require_string_list(
        validation.get("external_validation_plan"),
        "validation_evidence.external_validation_plan",
    )
    gates = _require_string_list(
        validation.get("violated_quality_gate_ids"),
        "validation_evidence.violated_quality_gate_ids",
        allow_empty=True,
    )
    _require_equal(
        gates,
        research.get("violated_quality_gate_ids"),
        "validation_evidence.violated_quality_gate_ids",
    )


def _validate_uncertainty_summary(validation: dict[str, Any]) -> None:
    uncertainty = _require_mapping(
        validation.get("uncertainty_summary"),
        "validation_evidence.uncertainty_summary",
    )
    _require_equal(
        uncertainty.get("schema_version"),
        "kyuubiki.material-uncertainty-summary/v1",
        "validation_evidence.uncertainty_summary.schema_version",
    )
    _require_string_list(
        uncertainty.get("known_limitations"),
        "validation_evidence.uncertainty_summary.known_limitations",
    )
    _require_equal(
        uncertainty.get("external_validation_required"),
        True,
        "validation_evidence.uncertainty_summary.external_validation_required",
    )
    _validate_confidence_counts(
        uncertainty.get("candidate_confidence_counts"),
        "validation_evidence.uncertainty_summary.candidate_confidence_counts",
    )
    _require_equal(
        uncertainty.get("candidate_confidence_counts"),
        validation.get("candidate_confidence_counts"),
        "validation_evidence.candidate_confidence_counts",
    )


def _validate_validation_readiness(validation: dict[str, Any]) -> None:
    readiness = _require_mapping(
        validation.get("validation_readiness"),
        "validation_evidence.validation_readiness",
    )
    _require_equal(
        readiness.get("schema_version"),
        "kyuubiki.material-validation-readiness/v1",
        "validation_evidence.validation_readiness.schema_version",
    )
    _require_equal(
        readiness.get("decision"),
        "screening_only",
        "validation_evidence.validation_readiness.decision",
    )
    score = readiness.get("score")
    if isinstance(score, bool) or not isinstance(score, (int, float)) or not 0 <= score <= 1:
        raise ValueError(
            "validation_evidence.validation_readiness.score must be between 0 and 1"
        )
    reasons = _require_string_list(
        readiness.get("blocking_reasons"),
        "validation_evidence.validation_readiness.blocking_reasons",
    )
    if "external_validation_required" not in reasons:
        raise ValueError(
            "validation_evidence.validation_readiness.blocking_reasons must include external_validation_required"
        )
    if validation.get("violated_quality_gate_ids") and "violated_quality_gates" not in reasons:
        raise ValueError(
            "validation_evidence.validation_readiness.blocking_reasons must include violated_quality_gates when gates are violated"
        )
    if validation["candidate_confidence_counts"].get("low", 0) > 0 and "low_confidence_material_cards" not in reasons:
        raise ValueError(
            "validation_evidence.validation_readiness.blocking_reasons must include low_confidence_material_cards when low-confidence cards exist"
        )
    _require_string_list(
        readiness.get("next_validation_actions"),
        "validation_evidence.validation_readiness.next_validation_actions",
    )


def _validate_material_card_refs(summary: dict[str, Any], research: dict[str, Any]) -> None:
    refs = _validate_object_list(
        summary.get("material_card_refs"),
        "summary.material_card_refs",
        ("material_card_id", "candidate_id", "confidence", "unit_system", "parameter_scope"),
    )
    count = _require_positive_int(summary.get("material_card_ref_count"), "summary.material_card_ref_count")
    if count != len(refs):
        raise ValueError("summary.material_card_ref_count must match material_card_refs")
    ranked = research.get("ranked_candidate_ids", [])
    for index, ref in enumerate(refs):
        _require_equal(
            ref.get("schema_version"),
            "kyuubiki.material-card/v1",
            f"summary.material_card_refs[{index}].schema_version",
        )
        if ref["candidate_id"] not in ranked:
            raise ValueError(
                f"summary.material_card_refs[{index}].candidate_id must be present in ranked candidates"
            )


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


def _require_equal(value: Any, expected: Any, field: str) -> None:
    if value != expected:
        raise ValueError(f"{field} must be {expected}, got {value!r}")


def _require_mapping(value: Any, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{field} must be an object")
    return value


def _require_list(value: Any, field: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{field} must be an array")
    return value


def _require_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty string")
    return value


def _require_non_negative_int(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return value


def _require_positive_int(value: Any, field: str) -> int:
    value = _require_non_negative_int(value, field)
    if value == 0:
        raise ValueError(f"{field} must be positive")
    return value


def _require_string_list(
    value: Any, field: str, *, allow_empty: bool = False
) -> list[str]:
    items = _require_list(value, field)
    if not allow_empty and not items:
        raise ValueError(f"{field} must be non-empty")
    if any(not isinstance(item, str) or not item for item in items):
        raise ValueError(f"{field} must contain only non-empty strings")
    return items


def _require_unique(values: list[str], field: str) -> None:
    if len(values) != len(set(values)):
        raise ValueError(f"{field} must not contain duplicates")


def _validate_object_list(
    value: Any, field: str, required_strings: tuple[str, ...]
) -> list[dict[str, Any]]:
    items = _require_list(value, field)
    if not items:
        raise ValueError(f"{field} must be non-empty")
    output: list[dict[str, Any]] = []
    for index, item in enumerate(items):
        mapped = _require_mapping(item, f"{field}[{index}]")
        for key in required_strings:
            _require_string(mapped.get(key), f"{field}[{index}].{key}")
        output.append(mapped)
    return output


def _validate_confidence_counts(value: Any, field: str) -> None:
    counts = _require_mapping(value, field)
    for key in ("low", "medium", "high", "unknown"):
        _require_non_negative_int(counts.get(key), f"{field}.{key}")


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
