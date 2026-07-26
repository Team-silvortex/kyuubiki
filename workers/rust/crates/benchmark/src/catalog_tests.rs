use super::{
    BenchmarkCatalogSpec, BenchmarkFamily, BenchmarkMatrixSpec, CaseTemplateSpec,
    default_catalog_spec, resolve_matrix_templates, select_matrix_spec,
};

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
