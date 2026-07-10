from __future__ import annotations

from copy import deepcopy
from typing import Any


def material_study_catalog() -> list[dict[str, Any]]:
    return [deepcopy(_catalog_entry(study_id)) for study_id in _STUDY_ORDER]


def describe_material_study(study: str) -> dict[str, Any]:
    study_id = _resolve_study_id(study)
    if study_id is None:
        raise ValueError(f"unsupported material report study: {study}")
    return deepcopy(_catalog_entry(study_id))


def extract_material_result_payloads(payload: Any) -> list[dict[str, Any]]:
    if isinstance(payload, list):
        return [_require_mapping(item) for item in payload]
    if not isinstance(payload, dict):
        raise ValueError(
            "material report input must be an array, include a results array, "
            "or be a headless execution run report"
        )
    if payload.get("schema_version") == "kyuubiki.headless-execution-run/v1":
        results = [
            _result_from_preview(step["result_preview"])
            for step in payload.get("steps", [])
            if isinstance(step, dict)
            and step.get("action") == "result_fetch"
            and step.get("status") not in ("blocked", "failed")
            and isinstance(step.get("result_preview"), dict)
        ]
        if not results:
            raise ValueError(
                "headless execution run report does not contain successful result_fetch payloads"
            )
        return results
    for key in ("results", "result_payloads"):
        value = payload.get(key)
        if isinstance(value, list):
            return [_require_mapping(item) for item in value]
    raise ValueError(
        "material report input must be an array, include a results array, "
        "or be a headless execution run report"
    )


def build_material_report(
    study: str,
    result_payloads: list[dict[str, Any]],
    *,
    optimization: dict[str, Any] | None = None,
) -> dict[str, Any]:
    study_id = _resolve_study_id(study)
    if study_id is None:
        raise ValueError(f"unsupported material report study: {study}")
    candidates = _CANDIDATES[study_id]
    if len(result_payloads) != len(candidates):
        raise ValueError(
            f"{study_id} expects {len(candidates)} result payloads, "
            f"received {len(result_payloads)}"
        )
    profile = optimization or _default_optimization(_METRICS[study_id])
    rows = [
        _candidate_report(study_id, candidate, payload, profile)
        for candidate, payload in zip(candidates, result_payloads)
    ]
    rows.sort(key=lambda row: row["score"], reverse=True)
    for index, row in enumerate(rows):
        row["rank"] = index + 1
    warnings = [
        f"{row['candidate_id']} is missing {metric}; ranking used remaining weighted metrics"
        for row in rows
        for metric in row["missing_metrics"]
    ]
    descriptor = _DESCRIPTORS[study_id]
    return {
        "schema_version": descriptor["schema_version"],
        "study": _STUDY_ALIASES[study_id],
        "objective": descriptor["objective"],
        "optimization": profile,
        "reliability": _reliability_envelope(study_id, rows),
        "metric_specs": deepcopy(_METRICS[study_id]),
        "candidates": rows,
        "winner_candidate_id": rows[0]["candidate_id"] if rows else None,
        "warnings": warnings,
    }


def build_material_report_from_payload(
    study: str,
    payload: Any,
    *,
    optimization: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return build_material_report(
        study,
        extract_material_result_payloads(payload),
        optimization=optimization,
    )


def _catalog_entry(study_id: str) -> dict[str, Any]:
    descriptor = _DESCRIPTORS[study_id]
    return {**descriptor, "metric_specs": deepcopy(_METRICS[study_id])}


def _resolve_study_id(study: str) -> str | None:
    normalized = study.strip().replace("-", "_").replace(".", "_").lower()
    for study_id, descriptor in _DESCRIPTORS.items():
        keys = [study_id, *descriptor["aliases"]]
        if any(key.replace("-", "_").replace(".", "_").lower() == normalized for key in keys):
            return study_id
    return None


def _require_mapping(item: Any) -> dict[str, Any]:
    if not isinstance(item, dict):
        raise ValueError("material result payload entries must be objects")
    return item


def _result_from_preview(preview: dict[str, Any]) -> dict[str, Any]:
    result = preview.get("result")
    return result if isinstance(result, dict) else preview


def _candidate_report(
    study_id: str,
    candidate: dict[str, Any],
    payload: dict[str, Any],
    optimization: dict[str, Any],
) -> dict[str, Any]:
    result = _descend_result_payload(payload)
    metrics = _study_metrics(study_id, candidate, result)
    missing = [
        metric["id"]
        for metric in _METRICS[study_id]
        if metric["direction"] != "observe" and metrics.get(metric["id"]) is None
    ]
    terms = _optimization_terms(_METRICS[study_id], metrics, optimization)
    score = sum(term["score"] * term["weight"] for term in terms)
    return {
        "candidate_id": candidate["id"],
        "candidate_label": candidate["label"],
        "rank": 0,
        "score": score,
        "metrics": metrics,
        "optimization_terms": terms,
        "missing_metrics": missing,
    }


def _reliability_envelope(study_id: str, rows: list[dict[str, Any]]) -> dict[str, Any]:
    gates = [_result_completeness_gate(study_id, rows)]
    return {
        "schema_version": "kyuubiki.material-reliability-envelope/v1",
        "posture": "screening_only",
        "material_card_version": f"kyuubiki.material-cards.{study_id}.v1",
        "unit_system": "SI",
        "quality_gates": gates,
        "summary": _reliability_summary(gates),
        "limitations": [
            "Python SDK material reports use screening fixtures and should be rerun through solver-backed studies before qualification.",
        ],
    }


def _result_completeness_gate(
    study_id: str,
    rows: list[dict[str, Any]],
) -> dict[str, Any]:
    complete_count = sum(1 for row in rows if not row["missing_metrics"])
    expected_count = len(rows)
    status = "pass" if complete_count >= expected_count else "violate"
    return {
        "id": "gate.result_completeness",
        "label": "Result payload completeness",
        "metric_id": "complete_candidate_count",
        "operator": ">=",
        "limit": expected_count,
        "actual_value": complete_count,
        "status": status,
        "description": f"Every {study_id} candidate should expose required result metrics before ranking is trusted.",
    }


def _reliability_summary(gates: list[dict[str, Any]]) -> dict[str, Any]:
    pass_count = sum(1 for gate in gates if gate["status"] == "pass")
    violation_count = sum(1 for gate in gates if gate["status"] == "violate")
    unknown_count = sum(1 for gate in gates if gate["status"] == "unknown")
    observe_count = len(gates) - pass_count - violation_count - unknown_count
    blocking_gate_ids = [
        gate["id"] for gate in gates if gate["status"] == "violate"
    ]
    if violation_count:
        decision = "blocked_by_quality_gates"
    elif unknown_count:
        decision = "needs_more_evidence"
    elif observe_count:
        decision = "review_observations"
    else:
        decision = "ready_for_next_round"
    return {
        "decision": decision,
        "total_gate_count": len(gates),
        "pass_count": pass_count,
        "violation_count": violation_count,
        "unknown_count": unknown_count,
        "observe_count": observe_count,
        "blocking_gate_ids": blocking_gate_ids,
    }


def _descend_result_payload(payload: dict[str, Any]) -> dict[str, Any]:
    result = payload.get("result")
    return result if isinstance(result, dict) else payload


def _study_metrics(
    study_id: str,
    candidate: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, float | None]:
    thickness = float(result.get("thickness_m") or result.get("thickness") or 0.002)
    if study_id == "material_heat_spreader_screening":
        return {
            "peak_temperature_c": _float(result.get("max_temperature")),
            "peak_heat_flux_w_m2": _float(result.get("max_heat_flux")),
            "areal_mass_kg_m2": candidate["density_kg_m3"] * thickness,
            "conductivity_density_ratio": candidate["thermal_conductivity_w_mk"]
            / candidate["density_kg_m3"],
        }
    if study_id == "material_dielectric_screening":
        field = _float(result.get("max_electric_field"))
        return {
            "max_electric_field_v_m": field,
            "breakdown_safety_factor": candidate["breakdown_field_v_m"] / field
            if field and field > 0
            else None,
            "dielectric_loss_proxy": candidate["relative_permittivity"]
            * candidate["dissipation_factor"],
            "areal_mass_kg_m2": candidate["density_kg_m3"] * thickness,
            "max_flux_density_c_m2": _float(result.get("max_flux_density")),
        }
    if study_id == "material_thermo_shield_screening":
        return {
            "max_stress_pa": _float(result.get("max_stress")),
            "max_displacement_m": _float(result.get("max_displacement")),
            "areal_mass_kg_m2": candidate["density_kg_m3"] * thickness,
            "max_temperature_delta_k": _float(result.get("max_temperature_delta")),
            "thermal_expansion_1_k": candidate["thermal_expansion_1_k"],
        }
    if study_id == "material_composite_thermo_electric_panel":
        field = _path_float(result, "electrostatic", "max_electric_field")
        return {
            "max_electric_field_v_m": field,
            "max_temperature_c": _path_float(result, "heat", "max_temperature"),
            "max_thermal_stress_pa": _path_float(result, "thermal", "max_stress"),
            "breakdown_safety_factor": candidate["breakdown_field_v_m"] / field
            if field and field > 0
            else None,
            "interface_risk_score": candidate["interface_risk_score"],
            "areal_mass_kg_m2": candidate["areal_mass_kg_m2"],
        }
    stress = _float(result.get("max_stress"))
    return {
        "max_stress_pa": stress,
        "max_displacement_m": _float(result.get("max_displacement")),
        "areal_mass_kg_m2": candidate["density_kg_m3"] * thickness,
        "specific_stiffness_m2_s2": candidate["youngs_modulus_pa"] / candidate["density_kg_m3"],
        "yield_safety_factor": candidate["yield_strength_pa"] / stress
        if stress and stress > 0
        else None,
    }


def _optimization_terms(
    metric_specs: list[dict[str, Any]],
    metrics: dict[str, float | None],
    optimization: dict[str, Any],
) -> list[dict[str, Any]]:
    weights = {
        weight["metric_id"]: float(weight.get("weight", 0.0))
        for weight in optimization.get("weights", [])
        if isinstance(weight, dict)
    }
    terms: list[dict[str, Any]] = []
    for spec in metric_specs:
        metric_id = spec["id"]
        value = metrics.get(metric_id)
        weight = weights.get(metric_id, float(spec.get("default_weight", 0.0)))
        direction = spec["direction"]
        score = _metric_score(value, direction)
        terms.append(
            {
                "metric_id": metric_id,
                "direction": direction,
                "weight": weight,
                "value": value,
                "score": score,
            }
        )
    return terms


def _metric_score(value: float | None, direction: str) -> float:
    if value is None or direction == "observe":
        return 0.0
    if direction == "maximize":
        return value / (1.0 + abs(value))
    return 1.0 / (1.0 + abs(value))


def _default_optimization(metric_specs: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "id": "default",
        "weights": [
            {"metric_id": metric["id"], "weight": metric.get("default_weight", 0.0)}
            for metric in metric_specs
            if metric["direction"] != "observe"
        ],
        "constraints": [],
    }


def _metric(
    metric_id: str,
    label: str,
    unit: str,
    direction: str,
    default_weight: float,
    source: str,
) -> dict[str, Any]:
    return {
        "id": metric_id,
        "label": label,
        "unit": unit,
        "direction": direction,
        "default_weight": default_weight,
        "source": source,
    }


def _float(value: Any) -> float | None:
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _path_float(payload: dict[str, Any], *path: str) -> float | None:
    value: Any = payload
    for key in path:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return _float(value)


_STUDY_ORDER = [
    "material_heat_spreader_screening",
    "material_dielectric_screening",
    "material_thermo_shield_screening",
    "material_structural_panel_screening",
    "material_composite_thermo_electric_panel",
]

_STUDY_ALIASES = {
    "material_heat_spreader_screening": "material.heat_spreader_screening.v1",
    "material_dielectric_screening": "material.dielectric_screening.v1",
    "material_thermo_shield_screening": "material.thermo_shield_screening.v1",
    "material_structural_panel_screening": "material.structural_panel_screening.v1",
    "material_composite_thermo_electric_panel": "material.composite_thermo_electric_panel.v1",
}

_DESCRIPTORS = {
    "material_heat_spreader_screening": {
        "id": "material_heat_spreader_screening",
        "title": "Heat Spreader Material Screening",
        "domain": "thermal",
        "objective": "rank heat spreader candidates by peak temperature, mass, and conductivity efficiency",
        "aliases": ["heat-spreader", "heat_spreader", _STUDY_ALIASES["material_heat_spreader_screening"]],
        "schema_version": "kyuubiki.material-research-report/v1",
        "template_id": "material_heat_spreader_screening",
    },
    "material_dielectric_screening": {
        "id": "material_dielectric_screening",
        "title": "Dielectric Material Screening",
        "domain": "electromagnetic",
        "objective": "rank dielectric candidates by electric-field margin, low loss, and low mass",
        "aliases": ["dielectric-screening", "dielectric_screening", _STUDY_ALIASES["material_dielectric_screening"]],
        "schema_version": "kyuubiki.dielectric-material-report/v1",
        "template_id": "material_dielectric_screening",
    },
    "material_thermo_shield_screening": {
        "id": "material_thermo_shield_screening",
        "title": "Thermo-Mechanical Shield Screening",
        "domain": "thermo_mechanical",
        "objective": "rank thermo-mechanical shield candidates by stress, displacement, thermal expansion, and mass",
        "aliases": ["thermo-shield", "thermo_shield", _STUDY_ALIASES["material_thermo_shield_screening"]],
        "schema_version": "kyuubiki.thermo-material-report/v1",
        "template_id": "material_thermo_shield_screening",
    },
    "material_structural_panel_screening": {
        "id": "material_structural_panel_screening",
        "title": "Structural Panel Material Screening",
        "domain": "structural",
        "objective": "rank structural panel candidates by stress, deflection, mass, stiffness, and yield margin",
        "aliases": ["structural-panel", "structural_panel", _STUDY_ALIASES["material_structural_panel_screening"]],
        "schema_version": "kyuubiki.structural-material-report/v1",
        "template_id": "material_structural_panel_screening",
    },
    "material_composite_thermo_electric_panel": {
        "id": "material_composite_thermo_electric_panel",
        "title": "Composite Thermo-Electric Panel",
        "domain": "multiphysics_materials",
        "objective": "rank mixed-material panel stacks across electric field, heat, thermal stress, interface risk, and mass",
        "aliases": ["composite-thermo-electric-panel", "composite_thermo_electric_panel", _STUDY_ALIASES["material_composite_thermo_electric_panel"]],
        "schema_version": "kyuubiki.composite-panel-report/v1",
        "template_id": "material_composite_thermo_electric_panel",
    },
}

_METRICS = {
    "material_heat_spreader_screening": [
        _metric("peak_temperature_c", "Peak temperature", "C", "minimize", 0.45, "solver.result.max_temperature"),
        _metric("peak_heat_flux_w_m2", "Peak heat flux", "W/m^2", "observe", 0.0, "solver.result.max_heat_flux"),
        _metric("areal_mass_kg_m2", "Areal mass", "kg/m^2", "minimize", 0.25, "candidate.density_kg_m3 * model.thickness"),
        _metric("conductivity_density_ratio", "Conductivity-density ratio", "W*m^2/(kg*K)", "maximize", 0.3, "candidate.thermal_conductivity_w_mk / candidate.density_kg_m3"),
    ],
    "material_dielectric_screening": [
        _metric("max_electric_field_v_m", "Max electric field", "V/m", "minimize", 0.25, "solver.result.max_electric_field"),
        _metric("breakdown_safety_factor", "Breakdown safety factor", "ratio", "maximize", 0.4, "candidate.breakdown_field_v_m / solver.result.max_electric_field"),
        _metric("dielectric_loss_proxy", "Dielectric loss proxy", "relative", "minimize", 0.2, "candidate.relative_permittivity * candidate.dissipation_factor"),
        _metric("areal_mass_kg_m2", "Areal mass", "kg/m^2", "minimize", 0.15, "candidate.density_kg_m3 * model.thickness"),
        _metric("max_flux_density_c_m2", "Max electric flux density", "C/m^2", "observe", 0.0, "solver.result.max_flux_density"),
    ],
    "material_thermo_shield_screening": [
        _metric("max_stress_pa", "Max von Mises stress", "Pa", "minimize", 0.45, "solver.result.max_stress"),
        _metric("max_displacement_m", "Max displacement", "m", "minimize", 0.25, "solver.result.max_displacement"),
        _metric("areal_mass_kg_m2", "Areal mass", "kg/m^2", "minimize", 0.25, "candidate.density_kg_m3 * model.thickness"),
        _metric("max_temperature_delta_k", "Max temperature delta", "K", "observe", 0.0, "solver.result.max_temperature_delta"),
        _metric("thermal_expansion_1_k", "Thermal expansion", "1/K", "minimize", 0.05, "candidate.thermal_expansion_1_k"),
    ],
    "material_structural_panel_screening": [
        _metric("max_stress_pa", "Max equivalent stress", "Pa", "minimize", 0.3, "solver.result.max_stress"),
        _metric("max_displacement_m", "Max displacement", "m", "minimize", 0.25, "solver.result.max_displacement"),
        _metric("areal_mass_kg_m2", "Areal mass", "kg/m^2", "minimize", 0.2, "candidate.density_kg_m3 * model.thickness"),
        _metric("specific_stiffness_m2_s2", "Specific stiffness", "m^2/s^2", "maximize", 0.15, "candidate.youngs_modulus_pa / candidate.density_kg_m3"),
        _metric("yield_safety_factor", "Yield safety factor", "ratio", "maximize", 0.1, "candidate.yield_strength_pa / solver.result.max_stress"),
    ],
    "material_composite_thermo_electric_panel": [
        _metric("max_electric_field_v_m", "Max electric field", "V/m", "minimize", 0.23, "electrostatic.max_electric_field"),
        _metric("max_temperature_c", "Max temperature", "C", "minimize", 0.23, "heat.max_temperature"),
        _metric("max_thermal_stress_pa", "Max thermal stress", "Pa", "minimize", 0.22, "thermal.max_stress"),
        _metric("breakdown_safety_factor", "Breakdown safety factor", "ratio", "maximize", 0.15, "candidate.breakdown_field / electrostatic.max_electric_field"),
        _metric("interface_risk_score", "Interface risk score", "0..1", "minimize", 0.12, "candidate.interface_compatibility"),
        _metric("areal_mass_kg_m2", "Areal mass", "kg/m^2", "minimize", 0.05, "candidate.stack_areal_mass"),
    ],
}

_CANDIDATES = {
    "material_heat_spreader_screening": [
        {"id": "aluminum_6061", "label": "Aluminum 6061", "density_kg_m3": 2700.0, "thermal_conductivity_w_mk": 167.0},
        {"id": "copper_c110", "label": "Copper C110", "density_kg_m3": 8960.0, "thermal_conductivity_w_mk": 385.0},
        {"id": "pyrolytic_graphite_in_plane", "label": "Pyrolytic graphite, in-plane", "density_kg_m3": 2200.0, "thermal_conductivity_w_mk": 1500.0},
    ],
    "material_dielectric_screening": [
        {"id": "polyimide_film", "label": "Polyimide film", "relative_permittivity": 3.4, "breakdown_field_v_m": 300.0e6, "dissipation_factor": 0.002, "density_kg_m3": 1420.0},
        {"id": "alumina_96", "label": "Alumina 96%", "relative_permittivity": 9.8, "breakdown_field_v_m": 130.0e6, "dissipation_factor": 0.0002, "density_kg_m3": 3720.0},
        {"id": "ptfe", "label": "PTFE", "relative_permittivity": 2.1, "breakdown_field_v_m": 60.0e6, "dissipation_factor": 0.0002, "density_kg_m3": 2200.0},
    ],
    "material_thermo_shield_screening": [
        {"id": "aluminum_6061_t6", "label": "Aluminum 6061-T6", "density_kg_m3": 2700.0, "thermal_expansion_1_k": 23.6e-6},
        {"id": "titanium_grade_5", "label": "Titanium Grade 5", "density_kg_m3": 4430.0, "thermal_expansion_1_k": 8.6e-6},
        {"id": "invar_36", "label": "Invar 36", "density_kg_m3": 8050.0, "thermal_expansion_1_k": 1.2e-6},
    ],
    "material_structural_panel_screening": [
        {"id": "aluminum_7075_t6", "label": "Aluminum 7075-T6", "density_kg_m3": 2810.0, "youngs_modulus_pa": 71.7e9, "yield_strength_pa": 503.0e6},
        {"id": "steel_4130_normalized", "label": "Steel 4130 normalized", "density_kg_m3": 7850.0, "youngs_modulus_pa": 205.0e9, "yield_strength_pa": 435.0e6},
        {"id": "carbon_fiber_quasi_iso", "label": "Carbon fiber quasi-isotropic", "density_kg_m3": 1600.0, "youngs_modulus_pa": 70.0e9, "yield_strength_pa": 600.0e6},
    ],
    "material_composite_thermo_electric_panel": [
        {"id": "copper_polyimide_aluminum", "label": "Copper / Polyimide / Aluminum", "breakdown_field_v_m": 300.0e6, "areal_mass_kg_m2": 2.85, "interface_risk_score": 0.63},
        {"id": "aluminum_alumina_aluminum", "label": "Aluminum / Alumina / Aluminum", "breakdown_field_v_m": 130.0e6, "areal_mass_kg_m2": 3.7, "interface_risk_score": 0.42},
        {"id": "copper_ptfe_glass_epoxy", "label": "Copper / PTFE / Glass epoxy", "breakdown_field_v_m": 60.0e6, "areal_mass_kg_m2": 2.25, "interface_risk_score": 0.91},
    ],
}
