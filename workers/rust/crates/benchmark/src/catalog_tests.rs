use super::{
    BenchmarkCatalogSpec, BenchmarkFamily, BenchmarkMatrixSpec, CaseTemplateSpec,
    benchmark_case_ids, benchmark_cases_for_ids, default_catalog_spec, resolve_matrix_templates,
    select_matrix_spec,
};
use crate::config::BenchmarkProfile;

#[test]
fn checked_in_catalog_matches_the_rust_fallback() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/catalog.default.json");
    let content = std::fs::read_to_string(path).expect("checked-in benchmark catalog");
    let checked_in =
        serde_json::from_str::<BenchmarkCatalogSpec>(&content).expect("valid benchmark catalog");

    assert_eq!(checked_in, default_catalog_spec());
}

#[test]
fn matrix_template_resolution_preserves_declared_order() {
    let spec = catalog_spec(vec![
        template("a", BenchmarkFamily::AxialBar),
        template("b", BenchmarkFamily::HeatBar1d),
        template("c", BenchmarkFamily::Frame2d),
    ]);
    let matrix = BenchmarkMatrixSpec {
        name: "ordered".to_string(),
        template_stems: vec!["c".to_string(), "a".to_string()],
        owned_templates: vec![],
    };

    let stems = resolve_matrix_templates(&spec, &matrix)
        .into_iter()
        .map(|template| template.stem.as_str())
        .collect::<Vec<_>>();

    assert_eq!(stems, vec!["c", "a"]);
}

#[test]
#[should_panic(expected = "benchmark matrix 'broken' references missing template 'missing'")]
fn matrix_template_resolution_rejects_missing_stems() {
    let spec = catalog_spec(vec![template("a", BenchmarkFamily::AxialBar)]);
    let matrix = BenchmarkMatrixSpec {
        name: "broken".to_string(),
        template_stems: vec!["missing".to_string()],
        owned_templates: vec![],
    };

    let _ = resolve_matrix_templates(&spec, &matrix);
}

#[test]
#[should_panic(expected = "benchmark matrix 'missing' is not defined")]
fn matrix_selection_rejects_unknown_name_instead_of_running_core() {
    let mut spec = catalog_spec(vec![template("a", BenchmarkFamily::AxialBar)]);
    spec.matrices.push(BenchmarkMatrixSpec {
        name: "core".to_string(),
        template_stems: vec!["a".to_string()],
        owned_templates: vec![],
    });

    let _ = select_matrix_spec(&spec, "missing");
}

#[test]
fn exact_case_generation_does_not_materialize_the_rest_of_the_matrix() {
    let ids = benchmark_case_ids(BenchmarkProfile::Medium, "thermal-structural");
    let selected = vec!["frame-2d-medium".to_string()];
    let cases = benchmark_cases_for_ids(BenchmarkProfile::Medium, "thermal-structural", &selected);

    assert!(ids.len() > cases.len());
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].id, "frame-2d-medium");
}

#[test]
fn dynamic_response_matrix_keeps_experimental_cases_out_of_the_release_gate() {
    let spec = default_catalog_spec();
    let dynamic = select_matrix_spec(&spec, "dynamic-response");
    let qualified = select_matrix_spec(&spec, "physics-coverage");

    assert_eq!(dynamic.template_stems.len(), 3);
    assert!(
        dynamic
            .template_stems
            .iter()
            .all(|stem| !qualified.template_stems.contains(stem))
    );
}

fn catalog_spec(templates: Vec<CaseTemplateSpec>) -> BenchmarkCatalogSpec {
    BenchmarkCatalogSpec {
        templates,
        matrices: vec![],
        profiles: vec![],
    }
}

fn template(stem: &str, family: BenchmarkFamily) -> CaseTemplateSpec {
    CaseTemplateSpec {
        stem: stem.to_string(),
        family,
    }
}
