defmodule KyuubikiWeb.WorkflowDiagnosticsReportPayloadRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes diagnostics report payload transform operator" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.compose_diagnostics_report_payload")
    assert MapSet.member?(operators, "transform.select_focus_payload")
    assert MapSet.member?(operators, "transform.compose_focus_chain_input")
    assert MapSet.member?(operators, "transform.compose_focus_bridge_request")
    assert MapSet.member?(operators, "transform.resolve_focus_bridge_execution")
    assert MapSet.member?(operators, "transform.execute_focus_bridge_execution")
  end

  test "composes bundle and guard into export-ready payload" do
    payload = %{
      "bundle" => %{
        "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
        "bundle_sources" => ["electrostatic", "thermal"],
        "bundle_items" => [%{"source" => "electrostatic"}],
        "bundle_payloads" => %{
          "thermal" => %{"thermal_temperature_max" => 125.0},
          "thermo" => %{"thermo_peak_stress" => 220.0, "thermo_peak_thermal_strain" => 4.8e-4}
        }
      },
      "guard" => %{
        "guard_status" => "block",
        "guard_recommendation" => "hold_and_review",
        "guard_triggers" => [%{"field" => "thermal_temperature_max"}]
      }
    }

    assert {:ok, report_payload} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_report_payload",
               payload,
               %{}
             )

    assert report_payload["report_contract"] == "kyuubiki.workflow_report_payload/v1"
    assert report_payload["report_kind"] == "diagnostics_bundle_report_payload"
    assert report_payload["report_sources"] == ["electrostatic", "thermal"]
    assert report_payload["report_guard_status"] == "block"
    assert report_payload["guard_payload"]["guard_recommendation"] == "hold_and_review"
    assert report_payload["report_focus_metrics"]["thermal.temperature_max"] == 125.0
    assert report_payload["report_focus_metrics"]["thermo.stress_peak"] == 220.0

    assert report_payload["report_focus_payloads"]["thermo.stress_peak"]["focus_contract"] ==
             "kyuubiki.workflow_focus_payload/v1"

    assert report_payload["report_focus_payloads"]["thermo.stress_peak"]["metric_id"] ==
             "thermo.stress_peak"

    assert Enum.any?(report_payload["report_highlights"], fn item ->
             item["id"] == "thermal.temperature_max" and item["attention"] == true
           end)

    assert report_payload["bundle_contract"] == "kyuubiki.workflow_diagnostics_bundle/v1"
  end

  test "selects one focus payload by metric id" do
    payload = %{
      "report_focus_payloads" => %{
        "thermo.stress_peak" => %{
          "focus_contract" => "kyuubiki.workflow_focus_payload/v1",
          "metric_id" => "thermo.stress_peak",
          "source" => "thermo",
          "value" => 220.0,
          "value_field" => "thermo_peak_stress",
          "context" => %{"peak_stress_x" => 14.0}
        }
      }
    }

    assert {:ok, focus_payload} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.select_focus_payload",
               payload,
               %{"metric_id" => "thermo.stress_peak"}
             )

    assert focus_payload["focus_contract"] == "kyuubiki.workflow_focus_payload/v1"
    assert focus_payload["metric_id"] == "thermo.stress_peak"
    assert focus_payload["value"] == 220.0
    assert focus_payload["context"]["peak_stress_x"] == 14.0
  end

  test "composes one focus chain input from report payload" do
    payload = %{
      "report_focus_payloads" => %{
        "thermo.stress_peak" => %{
          "focus_contract" => "kyuubiki.workflow_focus_payload/v1",
          "metric_id" => "thermo.stress_peak",
          "source" => "thermo",
          "value" => 220.0,
          "value_field" => "thermo_peak_stress",
          "context" => %{"peak_stress_x" => 14.0}
        }
      }
    }

    assert {:ok, chain_input} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_focus_chain_input",
               payload,
               %{
                 "metric_id" => "thermo.stress_peak",
                 "target_operator" => "bridge.temperature_field_to_thermo_quad_2d",
                 "bindings" => %{"seed_model_ref" => "thermo_seed"},
                 "annotations" => %{"label" => "stress handoff"}
               }
             )

    assert chain_input["chain_contract"] == "kyuubiki.workflow_focus_chain_input/v1"
    assert chain_input["metric_id"] == "thermo.stress_peak"
    assert chain_input["value"] == 220.0
    assert chain_input["target_operator"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert chain_input["bindings"]["seed_model_ref"] == "thermo_seed"
    assert chain_input["annotations"]["label"] == "stress handoff"
    assert chain_input["focus_payload"]["focus_contract"] == "kyuubiki.workflow_focus_payload/v1"
  end

  test "composes one focus bridge request from chain input" do
    payload = %{
      "chain_contract" => "kyuubiki.workflow_focus_chain_input/v1",
      "metric_id" => "thermo.stress_peak",
      "source" => "thermo",
      "value" => 220.0,
      "focus_payload" => %{
        "focus_contract" => "kyuubiki.workflow_focus_payload/v1",
        "metric_id" => "thermo.stress_peak"
      },
      "bindings" => %{"seed_model_ref" => "thermo_seed"},
      "annotations" => %{"label" => "stress handoff"},
      "target_operator" => "bridge.temperature_field_to_thermo_quad_2d"
    }

    assert {:ok, bridge_request} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_focus_bridge_request",
               payload,
               %{
                 "seed_model" => %{"nodes" => [], "elements" => []},
                 "contract" => %{"target" => %{"field" => "temperature_delta"}},
                 "bridge_payload_source" => "solve_heat.result"
               }
             )

    assert bridge_request["request_contract"] == "kyuubiki.workflow_focus_bridge_request/v1"
    assert bridge_request["bridge_operator"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert bridge_request["metric_id"] == "thermo.stress_peak"
    assert bridge_request["focus_value"] == 220.0
    assert bridge_request["bridge_payload_source"] == "solve_heat.result"
    assert bridge_request["bridge_config"]["seed_model"]["nodes"] == []

    assert bridge_request["focus_chain_input"]["chain_contract"] ==
             "kyuubiki.workflow_focus_chain_input/v1"
  end

  test "resolves one focus bridge execution from bridge request" do
    payload = %{
      "request_contract" => "kyuubiki.workflow_focus_bridge_request/v1",
      "bridge_operator" => "bridge.temperature_field_to_thermo_quad_2d",
      "metric_id" => "thermo.stress_peak",
      "focus_value" => 220.0,
      "bridge_config" => %{
        "seed_model" => %{"nodes" => [], "elements" => []},
        "contract" => %{"target" => %{"field" => "temperature_delta"}}
      },
      "bindings" => %{"seed_model_ref" => "thermo_seed"},
      "annotations" => %{"label" => "stress handoff"}
    }

    assert {:ok, execution} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.resolve_focus_bridge_execution",
               payload,
               %{
                 "bridge_payload" => %{"nodes" => [], "elements" => []},
                 "bridge_payload_source" => "solve_heat.result"
               }
             )

    assert execution["execution_contract"] == "kyuubiki.workflow_focus_bridge_execution/v1"
    assert execution["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert execution["metric_id"] == "thermo.stress_peak"
    assert execution["focus_value"] == 220.0
    assert execution["bridge_payload_source"] == "solve_heat.result"
    assert execution["bridge_config"]["seed_model"]["nodes"] == []
  end

  test "resolves one focus bridge execution from named workflow inputs" do
    payload = %{
      "request" => %{
        "request_contract" => "kyuubiki.workflow_focus_bridge_request/v1",
        "bridge_operator" => "bridge.temperature_field_to_thermo_quad_2d",
        "metric_id" => "thermo.stress_peak",
        "focus_value" => 220.0,
        "bridge_config" => %{
          "seed_model" => %{"nodes" => [], "elements" => []},
          "contract" => %{"target" => %{"field" => "temperature_delta"}}
        }
      },
      "bridge_payload" => %{"nodes" => [%{"id" => "h0"}], "elements" => []}
    }

    assert {:ok, execution} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.resolve_focus_bridge_execution",
               payload,
               %{}
             )

    assert execution["execution_contract"] == "kyuubiki.workflow_focus_bridge_execution/v1"
    assert execution["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert execution["bridge_payload"]["nodes"] == [%{"id" => "h0"}]
  end

  test "executes one focus bridge execution into bridge result" do
    payload = %{
      "execution_contract" => "kyuubiki.workflow_focus_bridge_execution/v1",
      "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
      "metric_id" => "thermo.stress_peak",
      "focus_value" => 220.0,
      "bridge_payload" => %{
        "input" => %{
          "nodes" => [
            %{
              "id" => "h0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_temperature" => true,
              "temperature" => 100.0,
              "heat_load" => 6.0
            },
            %{
              "id" => "h1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_temperature" => true,
              "temperature" => 80.0,
              "heat_load" => 12.0
            },
            %{
              "id" => "h2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_temperature" => true,
              "temperature" => 60.0,
              "heat_load" => 18.0
            },
            %{
              "id" => "h3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_temperature" => true,
              "temperature" => 40.0,
              "heat_load" => 24.0
            }
          ],
          "elements" => [
            %{
              "id" => "hq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "conductivity" => 45.0
            }
          ]
        },
        "nodes" => [
          %{
            "index" => 0,
            "id" => "h0",
            "x" => 0.0,
            "y" => 0.0,
            "temperature" => 100.0,
            "heat_load" => 6.0
          },
          %{
            "index" => 1,
            "id" => "h1",
            "x" => 1.0,
            "y" => 0.0,
            "temperature" => 80.0,
            "heat_load" => 12.0
          },
          %{
            "index" => 2,
            "id" => "h2",
            "x" => 1.0,
            "y" => 1.0,
            "temperature" => 60.0,
            "heat_load" => 18.0
          },
          %{
            "index" => 3,
            "id" => "h3",
            "x" => 0.0,
            "y" => 1.0,
            "temperature" => 40.0,
            "heat_load" => 24.0
          }
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "hq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "area" => 1.0,
            "average_temperature" => 70.0,
            "temperature_gradient_x" => -15.0,
            "temperature_gradient_y" => -5.0,
            "heat_flux_x" => 30.0,
            "heat_flux_y" => 10.0,
            "heat_flux_magnitude" => 31.6227766017
          }
        ],
        "max_temperature" => 100.0,
        "max_heat_flux" => 31.6227766017
      },
      "bridge_config" => %{
        "seed_model" => %{
          "nodes" => [
            %{
              "id" => "t0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 0.0
            },
            %{
              "id" => "t1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 0.0
            },
            %{
              "id" => "t2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 0.0
            },
            %{
              "id" => "t3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "tq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "youngs_modulus" => 70.0e9,
              "poisson_ratio" => 0.33,
              "thermal_expansion" => 1.1e-5
            }
          ]
        }
      },
      "bridge_payload_source" => "solve_heat.result"
    }

    assert {:ok, bridge_result} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.execute_focus_bridge_execution",
               payload,
               %{}
             )

    assert bridge_result["result_contract"] == "kyuubiki.workflow_focus_bridge_result/v1"
    assert bridge_result["operator_id"] == "bridge.temperature_field_to_thermo_quad_2d"
    assert bridge_result["metric_id"] == "thermo.stress_peak"
    assert bridge_result["focus_value"] == 220.0
    assert bridge_result["bridge_payload_source"] == "solve_heat.result"
    assert bridge_result["bridge_result"]["nodes"] |> length() == 4
  end
end
