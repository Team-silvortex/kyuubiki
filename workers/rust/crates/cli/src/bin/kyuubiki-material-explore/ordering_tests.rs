use super::candidate_index;
use serde_json::json;

#[test]
fn candidate_result_slots_follow_input_manifest_not_rank_order() {
    let exploration = json!({
        "study": "material_dielectric_screening",
        "candidate_input_manifest": {
            "entries": [
                {"candidate_id": "polyimide_film"},
                {"candidate_id": "alumina_96"},
                {"candidate_id": "ptfe"}
            ]
        },
        "report": {
            "candidates": [
                {"candidate_id": "polyimide_film", "rank": 1},
                {"candidate_id": "ptfe", "rank": 2},
                {"candidate_id": "alumina_96", "rank": 3}
            ]
        }
    });

    assert_eq!(candidate_index(&exploration, "polyimide_film"), Some(0));
    assert_eq!(candidate_index(&exploration, "alumina_96"), Some(1));
    assert_eq!(candidate_index(&exploration, "ptfe"), Some(2));
}

#[test]
fn legacy_runs_fall_back_to_builtin_study_order() {
    let exploration = json!({"study": "material_dielectric_screening"});

    assert_eq!(candidate_index(&exploration, "alumina_96"), Some(1));
    assert_eq!(candidate_index(&exploration, "ptfe"), Some(2));
}
