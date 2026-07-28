use crate::build_composite_panel_report;
use serde_json::json;

#[test]
fn composite_report_exposes_regions_and_reliability() {
    let report = build_composite_panel_report(&[
        json!({
            "electrostatic": { "max_electric_field": 42.0e6 },
            "heat": { "max_temperature": 90.0 },
            "thermal": { "max_stress": 120.0e6 }
        }),
        json!({
            "electrostatic": { "max_electric_field": 38.0e6 },
            "heat": { "max_temperature": 105.0 },
            "thermal": { "max_stress": 150.0e6 }
        }),
        json!({
            "electrostatic": { "max_electric_field": 48.0e6 },
            "heat": { "max_temperature": 82.0 },
            "thermal": { "max_stress": 95.0e6 }
        }),
    ])
    .expect("composite report");

    assert_eq!(report.schema_version, "kyuubiki.composite-panel-report/v1");
    assert_eq!(report.material_regions.len(), 3);
    assert_eq!(report.material_card_refs.len(), 3);
    assert!(
        report
            .material_card_refs
            .iter()
            .any(|reference| reference.material_card_id
                == "kyuubiki.material_card.copper_polyimide_aluminum.v1"
                && reference.schema_version == "kyuubiki.material-card/v1")
    );
    assert_eq!(report.reliability.posture, "prototype_screening_only");
    assert!(report.reliability.quality_gates.len() >= 5);
    assert!(
        report
            .candidates
            .iter()
            .all(|row| { row.interface_risk_score.is_some() && row.weakest_interface.is_some() })
    );
    assert!(
        report
            .reliability
            .quality_gates
            .iter()
            .any(|gate| { gate.id == "gate.interface_risk.prototype" })
    );
    assert!(
        report
            .candidates
            .iter()
            .all(|row| row.electrostatic_cross_validation.status == "fail"),
        "the synthetic test payloads should visibly fail the analytic cross-check"
    );
    assert!(
        report
            .candidates
            .iter()
            .all(|row| row.electrostatic_mesh_convergence.status == "missing"),
        "synthetic payloads without real refinement runs must not claim mesh convergence"
    );
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.electrostatic_closed_form.relative_error" && gate.status == "violate"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.electrostatic_mesh_convergence.finest_pair" && gate.status == "unknown"
    }));
    assert!(
        report
            .candidates
            .iter()
            .all(|row| row.heat_cross_validation.status == "fail"
                && row.heat_mesh_convergence.status == "missing")
    );
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.heat_closed_form.relative_error" && gate.status == "violate"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.heat_mesh_convergence.finest_pair" && gate.status == "unknown"
    }));
    assert!(
        report
            .candidates
            .iter()
            .all(|row| row.thermal_mesh_convergence.status == "missing")
    );
    assert!(report.candidates.iter().all(|row| {
        row.thermal_constraint_sensitivity.diagnosis == "insufficient_evidence"
            && row.thermal_constraint_sensitivity.qualification_effect
                == "diagnostic_only_does_not_override_primary_quality_gates"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.thermal_mesh_convergence.strain_energy" && gate.status == "unknown"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.thermal_mesh_gci.displacement" && gate.status == "unknown"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.thermal_mesh_gci.strain_energy" && gate.status == "unknown"
    }));
    assert!(report.reliability.quality_gates.iter().any(|gate| {
        gate.id == "gate.thermal_solver.relative_residual" && gate.status == "unknown"
    }));
    assert!(
        report
            .candidates
            .iter()
            .all(|row| row.thermal_stress_recovery.status == "missing")
    );
    assert!(report.candidates.iter().all(|row| {
        row.thermal_interface_grading_assessment.diagnosis == "insufficient_evidence"
            && row
                .thermal_interface_grading_assessment
                .qualification_effect
                == "diagnostic_only_does_not_override_uniform_mesh_gates"
    }));
    assert!(
        report.reliability.quality_gates.iter().any(|gate| {
            gate.id == "gate.thermal_stress_recovery.p95" && gate.status == "unknown"
        })
    );
    assert!(report.winner_candidate_id.is_some());
}
