defmodule KyuubikiWeb.Api.WorkflowCatalogThermalJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits a heat quad guard catalog workflow as an asynchronous job" do
    {:ok, _pid} = WorkflowApi.start_fake_agent_sessions([[guard_heat_result()]])
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.heat-plane-quad-guard-json",
        WorkflowApi.heat_to_thermo_quad_input_artifacts()
      )

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.heat-plane-quad-guard-json"
    assert result_payload["result"]["dataset_contract"]["id"] == "kyuubiki.dataset.heat_plane_quad_guard/v1"
    assert length(result_payload["result"]["completed_nodes"]) == 6

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"

    summary = Jason.decode!(exported["content"])
    assert summary["guard_status"] == "block"
    assert summary["guard_passed"] == false
    assert summary["guard_trigger_count"] == 2

    assert Enum.any?(summary["guard_triggers"], fn trigger ->
             trigger["field"] == "thermal_temperature_max" and trigger["severity"] == "warn"
           end)

    assert Enum.any?(summary["guard_triggers"], fn trigger ->
             trigger["field"] == "thermal_peak_flux_magnitude" and trigger["severity"] == "block"
           end)
  end

  test "submits a heat thermo benchmark catalog workflow as an asynchronous job" do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [benchmark_heat_result()],
        [benchmark_thermo_result()]
      ])

    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.heat-thermo-quad-benchmark-json",
        WorkflowApi.heat_to_thermo_quad_input_artifacts()
      )

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.heat-thermo-quad-benchmark-json"
    assert result_payload["result"]["dataset_contract"]["id"] ==
             "kyuubiki.dataset.heat_thermo_quad_benchmark/v1"

    assert length(result_payload["result"]["completed_nodes"]) == 9

    assert Enum.any?(result_payload["result"]["dataset_lineage"], fn entry ->
             entry["dataset_value"] == "benchmark_summary" and
               MapSet.new(entry["source_datasets"]) ==
                 MapSet.new(["thermal_diagnostics", "thermo_diagnostics"])
           end)

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"

    summary = Jason.decode!(exported["content"])
    assert summary["thermal_score"] == 1.5
    assert summary["thermo_score"] == 4.5
    assert summary["benchmark_winner"] == "thermo"
    assert summary["benchmark_margin"] == 3.0
    assert summary["benchmark_criteria_count"] == 3
  end

  defp guard_heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 140.0,
        "max_heat_flux" => 3000.0,
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 140.0, "heat_load" => 450.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 90.0, "heat_load" => 450.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 40.0, "heat_load" => 450.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 30.0, "heat_load" => 450.0}
        ],
        "elements" => [
          %{
            "id" => "hq0",
            "temperature_gradient_x" => -30.0,
            "temperature_gradient_y" => -60.0,
            "heat_flux_x" => -1800.0,
            "heat_flux_y" => 2400.0
          }
        ],
        "input" => %{
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
        }
      }
    }
  end

  defp benchmark_heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 70.0,
        "max_heat_flux" => 3000.0,
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 70.0, "heat_load" => 300.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 45.0, "heat_load" => 300.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0}
        ],
        "elements" => [
          %{
            "id" => "hq0",
            "temperature_gradient_x" => -25.0,
            "temperature_gradient_y" => -50.0,
            "heat_flux_x" => -1800.0,
            "heat_flux_y" => 2400.0
          }
        ],
        "input" => %{
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
        }
      }
    }
  end

  defp benchmark_thermo_result do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0015,
        "max_stress" => 1200.0,
        "max_temperature_delta" => 70.0,
        "nodes" => [
          %{"id" => "h0", "temperature_delta" => 70.0, "displacement_x" => 0.0, "displacement_y" => 0.0},
          %{"id" => "h1", "temperature_delta" => 45.0, "displacement_x" => 0.001, "displacement_y" => 0.0},
          %{"id" => "h2", "temperature_delta" => 20.0, "displacement_x" => 0.0015, "displacement_y" => 0.0},
          %{"id" => "h3", "temperature_delta" => 20.0, "displacement_x" => 0.0012, "displacement_y" => 0.0}
        ],
        "elements" => [
          %{
            "id" => "tq0",
            "von_mises_stress" => 1200.0,
            "stress_x" => -1200.0,
            "stress_y" => -1200.0
          }
        ]
      }
    }
  end
end
