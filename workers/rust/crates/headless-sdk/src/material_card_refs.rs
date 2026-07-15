use crate::material_research::MaterialCardReference;

pub(crate) fn built_in_material_card_ref(
    candidate_id: &str,
    confidence: &str,
    parameter_scope: &str,
    source: &str,
) -> MaterialCardReference {
    MaterialCardReference {
        material_card_id: format!("kyuubiki.material_card.{candidate_id}.v1"),
        schema_version: "kyuubiki.material-card/v1".to_string(),
        candidate_id: candidate_id.to_string(),
        confidence: confidence.to_string(),
        unit_system: "si".to_string(),
        parameter_scope: parameter_scope.to_string(),
        source: source.to_string(),
    }
}
