use super::{
    BenchmarkCatalogSpec, BenchmarkFamily, BenchmarkMatrixSpec, CaseTemplateSpec,
    resolve_matrix_templates,
};

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
