use crate::{
    SolveAdvectionDiffusionBar1dRequest as Request, SolveAdvectionDiffusionBar1dResult as Result,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.advection-diffusion-scheme.json"
    ))
    .unwrap()
}

#[test]
fn transport_scheme_shared_fixture_accepts_only_explicit_supported_schemes() {
    let cases = fixture();
    assert_eq!(cases["accepted"].as_array().unwrap().len(), 3);
    assert_eq!(cases["rejected"].as_array().unwrap().len(), 7);
    for case in cases["accepted"].as_array().unwrap() {
        let mut model = cases["model"].clone();
        if let Some(scheme) = case.get("scheme") {
            model["scheme"] = scheme.clone();
        }
        let input: Request = serde_json::from_value(model).unwrap();
        assert_eq!(
            serde_json::to_value(input.scheme).unwrap(),
            case["expected"]
        );
        let normalized = serde_json::to_value(&input).unwrap();
        if case["expected"] == "galerkin" {
            assert!(normalized.get("scheme").is_none());
        } else {
            assert_eq!(normalized["scheme"], "upwind");
        }
        assert_eq!(
            serde_json::from_value::<Request>(normalized).unwrap(),
            input
        );
    }
    for case in cases["rejected"].as_array().unwrap() {
        let mut model = cases["model"].clone();
        model["scheme"] = case["scheme"].clone();
        assert!(
            serde_json::from_value::<Request>(model).is_err(),
            "{}",
            case["id"]
        );
    }
}

fn result() -> Value {
    json!({
        "input":fixture()["model"],"nodes":[],
        "elements":[{"index":0,"id":"e0","node_i":0,"node_j":1,"length":0.5,
                     "average_concentration":0.125,"concentration_gradient":0.5,
                     "diffusive_flux":-0.5,"advective_flux":2.5,"total_flux":2.0,"peclet_number":5.0}],
        "max_concentration":1.0,"max_total_flux":11.0,"max_peclet_number":5.0
    })
}

#[test]
fn legacy_transport_result_decodes_without_new_optional_diagnostics() {
    let value: Result = serde_json::from_value(result()).unwrap();
    assert!(value.max_numerical_flux.is_none());
    assert!(value.elements[0].stabilization.is_none());
    let output = serde_json::to_value(value).unwrap();
    assert!(output.get("max_numerical_flux").is_none());
    assert!(output["elements"][0].get("stabilization").is_none());
    assert!(output["input"].get("scheme").is_none());
}

#[test]
fn upwind_result_preserves_separate_physical_and_numerical_diagnostics() {
    let mut input = result();
    input["input"]["scheme"] = json!("upwind");
    input["max_numerical_flux"] = json!(3.5);
    input["elements"][0]["stabilization"] = json!({
        "artificial_diffusivity":5.0,"stabilization_flux":-2.5,"numerical_flux":-0.5
    });
    let value: Result = serde_json::from_value(input).unwrap();
    assert_eq!(value.elements[0].total_flux, 2.0);
    assert_eq!(
        value.elements[0]
            .stabilization
            .as_ref()
            .unwrap()
            .numerical_flux,
        -0.5
    );
    let output = serde_json::to_value(&value).unwrap();
    assert_eq!(output["input"]["scheme"], "upwind");
    assert_eq!(serde_json::from_value::<Result>(output).unwrap(), value);
}
