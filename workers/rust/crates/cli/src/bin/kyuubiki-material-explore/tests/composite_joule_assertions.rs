use serde_json::Value;

pub(super) fn assert_candidate(row: &Value, id: &str) {
    let projection = &row["joule_heating_projection"];
    assert_eq!(
        projection["status"].as_str(),
        Some("pass"),
        "{id}: Joule status"
    );
    assert_eq!(
        projection["schema_version"].as_str(),
        Some("kyuubiki.composite-current-to-heat-projection/v1"),
        "{id}: Joule schema"
    );
    assert_eq!(
        projection["model"].as_str(),
        Some("solved_steady_current_density_sigma_e_squared"),
        "{id}: solved current model"
    );
    assert!(
        projection["max_current_density_a_m2"]
            .as_f64()
            .is_some_and(|density| density > 0.0),
        "{id}: solved current density"
    );
    assert!(
        projection["total_joule_loss_w"]
            .as_f64()
            .is_some_and(|power| power > 0.0),
        "{id}: Joule power"
    );
    assert!(
        projection["energy_balance_relative_error"]
            .as_f64()
            .is_some_and(|error| error <= 1.0e-12),
        "{id}: Joule energy balance"
    );
    assert_eq!(
        projection["regions"].as_array().map(Vec::len),
        Some(1),
        "{id}: Joule region coverage"
    );
}

pub(super) fn assert_quality_gate(report: &Value) {
    assert!(
        report["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.iter().any(|gate| {
                gate["id"].as_str() == Some("gate.joule_heating.energy_balance")
                    && gate["status"].as_str() == Some("pass")
            })),
        "Joule quality gate must pass"
    );
}
