use super::*;

fn fixture(method: &str, job: &str) -> Value {
    let thermal = method.starts_with("solve_thermal_");
    let mut grid = heat_grid(job, if thermal { 12 } else { 40 });
    if method.contains("triangle") {
        let elements: Vec<_> = grid["elements"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|quad| {
                let mut first = quad.clone();
                first.as_object_mut().unwrap().remove("node_l");
                first["id"] = json!(format!("{}a", quad["id"].as_str().unwrap()));
                let mut second = first.clone();
                second["id"] = json!(format!("{}b", quad["id"].as_str().unwrap()));
                second["node_j"] = quad["node_k"].clone();
                second["node_k"] = quad["node_l"].clone();
                [first, second]
            })
            .collect();
        grid["elements"] = json!(elements);
    }
    if thermal {
        for node in grid["nodes"].as_array_mut().unwrap() {
            node.as_object_mut()
                .unwrap()
                .retain(|key, _| matches!(key.as_str(), "id" | "x" | "y"));
            node["fix_x"] = json!(node["x"] == 0.0);
            node["fix_y"] = json!(node["y"] == 0.0);
            node["load_x"] = json!(0.0);
            node["load_y"] = json!(0.0);
            node["temperature_delta"] = json!(20.0);
        }
        for element in grid["elements"].as_array_mut().unwrap() {
            element.as_object_mut().unwrap().remove("conductivity");
            element["youngs_modulus"] = json!(1e9);
            element["poisson_ratio"] = json!(0.25);
            element["thermal_expansion"] = json!(1e-5);
        }
    }
    grid
}

fn verify_field(method: &str, input: &Value, result: &Value) {
    assert_eq!(
        result["nodes"].as_array().unwrap().len(),
        input["nodes"].as_array().unwrap().len()
    );
    assert_eq!(
        result["elements"].as_array().unwrap().len(),
        input["elements"].as_array().unwrap().len()
    );
    if method.starts_with("solve_heat_") {
        for node in result["nodes"].as_array().unwrap() {
            let expected = 100.0 - 80.0 * node["x"].as_f64().unwrap();
            assert!((node["temperature"].as_f64().unwrap() - expected).abs() < 1e-6);
        }
        for element in result["elements"].as_array().unwrap() {
            assert!((element["heat_flux_x"].as_f64().unwrap() - 3600.0).abs() < 1e-3);
            assert!(element["heat_flux_y"].as_f64().unwrap().abs() < 1e-3);
        }
    } else {
        for node in result["nodes"].as_array().unwrap() {
            assert!(
                (node["ux"].as_f64().unwrap() - node["x"].as_f64().unwrap() * 2e-4).abs() < 1e-10
            );
            assert!(
                (node["uy"].as_f64().unwrap() - node["y"].as_f64().unwrap() * 2e-4).abs() < 1e-10
            );
        }
        assert!(result["max_stress"].as_f64().unwrap().abs() < 1e-4);
    }
}

fn exercise(method: &str, stage: &str, disconnect: bool) -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold(stage, method, "1")?;
    let job = "postprocess-held";
    let input = fixture(method, job);
    let steps = match stage {
        "result_node_summary" => input["nodes"].as_array().unwrap().len().min(1024),
        "result_element_summary" => input["elements"].as_array().unwrap().len().min(1024),
        _ => 64,
    };
    let baseline = agent.request("baseline", method, input.clone())?;
    let expected = successful_result(&baseline, "baseline");
    verify_field(method, &input, expected);
    record(&agent, "baseline", &baseline)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    fs::write(&agent.hold_path, job)?;
    let mut stream = pending(&agent, "postprocess-old", method, input.clone())?;
    let held = wait_for_numerical_steps(&agent, &["postprocess-old"], stage)?;
    assert_eq!(
        held["result"]["solver_control"]["active"][0]["checkpoint"]["completed_steps"],
        steps
    );
    if disconnect {
        stream.shutdown(Shutdown::Both)?;
        drop(stream);
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let outcome = agent.request("orphan-outcome", "describe_agent", json!({}))?;
        assert!(
            outcome["result"]["watchdog"]["recent_failures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["request_id"] == "postprocess-old"
                    && f["reason_code"] == "cancelled"
                    && f["message"]
                        .as_str()
                        .unwrap_or("")
                        .contains(&format!("{stage} after {steps} steps")))
        );
        record(&agent, "orphan-outcome", &outcome)?;
    } else {
        cancel(&agent, job)?;
        let response = final_frame(&mut stream)?;
        assert_cancelled(&response, stage);
        assert_eq!(
            response["error"]["details"]["solver_checkpoint"]["completed_steps"],
            steps
        );
        record(&agent, "cancelled", &response)?;
        wait_for_lifecycle(&agent, "accepting", 0)?;
    }
    assert!(agent.hold_path.exists());
    fs::remove_file(&agent.hold_path)?;
    let healthy = agent.request("healthy-next", method, input.clone())?;
    let actual = successful_result(&healthy, "healthy-next");
    verify_field(method, &input, actual);
    assert_eq!(actual, expected);
    record(&agent, "healthy-next", &healthy)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

fn cancel_stages(method: &str) -> Result<(), Box<dyn Error>> {
    for stage in [
        "result_free_dofs",
        "result_nodes",
        "result_elements",
        "result_node_summary",
        "result_element_summary",
        "result_totals",
        if method.starts_with("solve_heat_") {
            "result_prescribed"
        } else {
            "result_rhs_norm"
        },
    ] {
        exercise(method, stage, false)?;
    }
    Ok(())
}

fn disconnect_stages(method: &str) -> Result<(), Box<dyn Error>> {
    for stage in ["result_elements", "result_totals"] {
        exercise(method, stage, true)?;
    }
    Ok(())
}

#[test]
fn heat_quad_postprocessing_cancel_and_replay() -> Result<(), Box<dyn Error>> {
    cancel_stages(HEAT)
}
#[test]
fn heat_triangle_postprocessing_cancel_and_replay() -> Result<(), Box<dyn Error>> {
    cancel_stages("solve_heat_plane_triangle_2d")
}
#[test]
fn thermal_quad_postprocessing_cancel_and_replay() -> Result<(), Box<dyn Error>> {
    cancel_stages("solve_thermal_plane_quad_2d")
}
#[test]
fn thermal_triangle_postprocessing_cancel_and_replay() -> Result<(), Box<dyn Error>> {
    cancel_stages("solve_thermal_plane_triangle_2d")
}
#[test]
fn heat_quad_postprocessing_disconnect_and_replay() -> Result<(), Box<dyn Error>> {
    disconnect_stages(HEAT)
}
#[test]
fn heat_triangle_postprocessing_disconnect_and_replay() -> Result<(), Box<dyn Error>> {
    disconnect_stages("solve_heat_plane_triangle_2d")
}
#[test]
fn thermal_quad_postprocessing_disconnect_and_replay() -> Result<(), Box<dyn Error>> {
    disconnect_stages("solve_thermal_plane_quad_2d")
}
#[test]
fn thermal_triangle_postprocessing_disconnect_and_replay() -> Result<(), Box<dyn Error>> {
    disconnect_stages("solve_thermal_plane_triangle_2d")
}
