use super::mismatch;
use crate::{HeadlessExecutionBatchStep, HeadlessWorkflowStep};
use serde_json::Value;
use std::sync::OnceLock;

type Profiles = Vec<HeadlessWorkflowStep>;
type ProfileBuilder = fn() -> Profiles;

pub(super) fn candidate_profiles(study: &str) -> Result<&'static [HeadlessWorkflowStep], String> {
    static HEAT: OnceLock<Profiles> = OnceLock::new();
    static DIELECTRIC: OnceLock<Profiles> = OnceLock::new();
    static THERMO: OnceLock<Profiles> = OnceLock::new();
    static STRUCTURAL: OnceLock<Profiles> = OnceLock::new();
    static COMPOSITE: OnceLock<Profiles> = OnceLock::new();
    let (cache, build): (&OnceLock<Profiles>, ProfileBuilder) = match study {
        "material_heat_spreader_screening" => (&HEAT, crate::build_heat_spreader_screening_steps),
        "material_dielectric_screening" => (&DIELECTRIC, crate::build_dielectric_screening_steps),
        "material_thermo_shield_screening" => (&THERMO, crate::build_thermo_shield_screening_steps),
        "material_structural_panel_screening" => {
            (&STRUCTURAL, crate::build_structural_panel_screening_steps)
        }
        "material_composite_thermo_electric_panel" => {
            (&COMPOSITE, crate::build_composite_panel_steps)
        }
        _ => return Err(mismatch("study has no fixed report input profile")),
    };
    // Only the finite built-in candidate set is cached, never caller models or results.
    Ok(cache.get_or_init(|| {
        build()
            .into_iter()
            .filter(|step| step.payload.get("research").is_some())
            .collect()
    }))
}

pub(super) fn validate_fixed_inputs(
    study: &str,
    step: &HeadlessExecutionBatchStep,
) -> Result<(), String> {
    let candidate = step
        .payload
        .pointer("/research/candidate_id")
        .and_then(Value::as_str);
    let profile = candidate_profiles(study)?
        .iter()
        .find(|profile| {
            profile
                .payload
                .pointer("/research/candidate_id")
                .and_then(Value::as_str)
                == candidate
        })
        .ok_or_else(|| {
            mismatch(format!(
                "step {} has no fixed candidate profile",
                step.index
            ))
        })?;
    let expected = profile
        .payload
        .as_object()
        .ok_or_else(|| mismatch("invalid built-in candidate profile"))?;
    let actual = step
        .payload
        .as_object()
        .ok_or_else(|| mismatch("candidate payload must be an object"))?;
    for (key, value) in expected {
        let path = child_path("", key);
        let changed = if key == "research" {
            research_difference(value, actual.get(key))
        } else {
            difference(value, actual.get(key), &path)
        };
        if let Some(path) = changed {
            return Err(profile_error(step, candidate, &path));
        }
    }
    for key in actual.keys() {
        if !expected.contains_key(key)
            && !context_key(key)
            && !matches!(key.as_str(), "annotations" | "notes")
        {
            return Err(profile_error(step, candidate, &child_path("", key)));
        }
    }
    Ok(())
}

fn profile_error(step: &HeadlessExecutionBatchStep, candidate: Option<&str>, path: &str) -> String {
    mismatch(format!(
        "step {} candidate {} differs from the fixed report input profile at {path}; use a custom research specification or supported materialized-candidate report for changed physical inputs, not original fixed candidate constants",
        step.index,
        candidate.unwrap_or("unknown")
    ))
}

fn context_key(key: &str) -> bool {
    matches!(
        key,
        "project_id"
            | "projectId"
            | "model_version_id"
            | "modelVersionId"
            | "simulation_case_id"
            | "simulationCaseId"
    )
}

fn research_difference(expected: &Value, actual: Option<&Value>) -> Option<String> {
    let (Some(expected), Some(actual)) = (expected.as_object(), actual.and_then(Value::as_object))
    else {
        return Some("/research".into());
    };
    for (key, value) in expected {
        if key != "study"
            && !display_key(key)
            && let Some(path) = difference(value, actual.get(key), &child_path("/research", key))
        {
            return Some(path);
        }
    }
    for key in actual.keys() {
        if !expected.contains_key(key) && !display_key(key) {
            return Some(child_path("/research", key));
        }
    }
    None
}

fn difference(expected: &Value, actual: Option<&Value>, path: &str) -> Option<String> {
    let Some(actual) = actual else {
        return Some(path.into());
    };
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                if let Some(path) = difference(value, actual.get(key), &child_path(path, key)) {
                    return Some(path);
                }
            }
            actual
                .keys()
                .find(|key| !expected.contains_key(*key))
                .map(|key| child_path(path, key))
        }
        (Value::Array(expected), Value::Array(actual)) if expected.len() == actual.len() => {
            expected
                .iter()
                .zip(actual)
                .enumerate()
                .find_map(|(index, (expected, actual))| {
                    difference(
                        expected,
                        Some(actual),
                        &child_path(path, &index.to_string()),
                    )
                })
        }
        (Value::Number(expected), Value::Number(actual)) => {
            let same = if expected.is_f64() || actual.is_f64() {
                match (expected.as_f64(), actual.as_f64()) {
                    (Some(expected), Some(actual))
                        if expected.is_finite() && actual.is_finite() =>
                    {
                        // Permit JSON float roundoff, not study or solver tolerances.
                        expected == actual
                            || (expected - actual).abs()
                                <= 8.0 * f64::EPSILON * expected.abs().max(actual.abs())
                    }
                    _ => false,
                }
            } else {
                expected == actual
            };
            (!same).then(|| path.into())
        }
        _ => (expected != actual).then(|| path.into()),
    }
}

fn display_key(key: &str) -> bool {
    matches!(
        key,
        "candidate_label" | "note" | "objective" | "annotations" | "notes"
    )
}

fn child_path(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numeric_profile_comparison_is_roundoff_only_not_a_physical_tolerance() {
        for scale in [1.0e-11, 1.0, 1.0e11] {
            assert!(
                difference(
                    &json!(scale),
                    Some(&json!(scale * (1.0 + f64::EPSILON))),
                    "/value"
                )
                .is_none()
            );
            for changed in [scale * (1.0 + 1.0e-10), 0.0, -scale] {
                assert_eq!(
                    difference(&json!(scale), Some(&json!(changed)), "/value").as_deref(),
                    Some("/value")
                );
            }
        }
        assert!(difference(&json!(1.0), Some(&json!(1)), "/value").is_none());
        assert!(difference(&json!(u64::MAX), Some(&json!(u64::MAX - 1)), "/value").is_some());
        assert!(difference(&json!(0.0), Some(&json!(f64::from_bits(1))), "/value").is_some());
    }

    #[test]
    fn profile_difference_returns_escaped_paths_and_rejects_missing_research_objects() {
        let expected = json!({"a/b~c": [1, 2]});
        let actual = json!({"a/b~c": [1, 3]});
        assert_eq!(
            difference(&expected, Some(&actual), "").as_deref(),
            Some("/a~1b~0c/1")
        );
        for actual in [None, Some(&Value::Null), Some(&json!([]))] {
            assert_eq!(
                research_difference(&json!({}), actual).as_deref(),
                Some("/research")
            );
        }
    }

    #[test]
    fn profile_cache_contains_only_the_finite_built_in_candidate_set() {
        let mut count = 0;
        let mut bytes = 0;
        for study in crate::material_study_descriptors() {
            let profiles = candidate_profiles(study.id).unwrap();
            assert_eq!(profiles.len(), 3);
            assert!(std::ptr::eq(
                profiles,
                candidate_profiles(study.id).unwrap()
            ));
            count += profiles.len();
            bytes += serde_json::to_vec(profiles).unwrap().len();
        }
        assert_eq!(count, 15);
        assert!(bytes < 128 * 1024, "built-in profile JSON: {bytes}");
        assert!(candidate_profiles("caller-controlled-study").is_err());
    }
}
