defmodule KyuubikiWeb.Api.WorkflowCatalogThermalJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.WorkflowDiagnosticAgent, as: DiagnosticAgent

  test "submits a heat quad guard catalog workflow as an asynchronous job" do
    agent =
      DiagnosticAgent.start!([
        DiagnosticAgent.solver("solve_heat_plane_quad_2d", [guard_heat_result()]),
        DiagnosticAgent.diagnostic(
          "extract_thermal_diagnostics",
          "extract.thermal_result_diagnostics"
        )
      ])

    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.heat-plane-quad-guard-json",
        WorkflowApi.heat_to_thermo_quad_input_artifacts()
      )

    assert result_payload["job"]["status"] == "completed"
    [_, diagnostic] = DiagnosticAgent.assert_complete(agent)
    assert_diagnostic_input(diagnostic, result_payload, "solve_heat.result")
    assert result_payload["result"]["workflow_id"] == "workflow.heat-plane-quad-guard-json"

    assert result_payload["result"]["dataset_contract"]["id"] ==
             "kyuubiki.dataset.heat_plane_quad_guard/v1"

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
    agent =
      DiagnosticAgent.start!([
        DiagnosticAgent.solver("solve_heat_plane_quad_2d", [benchmark_heat_result()]),
        DiagnosticAgent.diagnostic(
          "extract_thermal_diagnostics",
          "extract.thermal_result_diagnostics"
        ),
        DiagnosticAgent.solver("solve_thermal_plane_quad_2d", [benchmark_thermo_result()]),
        DiagnosticAgent.diagnostic(
          "extract_thermo_diagnostics",
          "extract.thermo_result_diagnostics"
        )
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
    [_, thermal, _, thermo] = DiagnosticAgent.assert_complete(agent)
    assert_diagnostic_input(thermal, result_payload, "solve_heat.result")
    assert_diagnostic_input(thermo, result_payload, "solve_thermo.result")
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

  for field <- ~w(task_id task_digest operator_id program_id) do
    test "rejects a diagnostic receipt with stale #{field} and a fresh run recovers" do
      assert_rejected_then_recovered(
        &Map.put(&1, unquote(field), "stale-diagnostic-identity"),
        "native_workflow_recovery_dispatch_unconfirmed"
      )
    end
  end

  test "a delivery acknowledgement cannot publish diagnostics or export a guard report" do
    assert_rejected_then_recovered(
      fn receipt ->
        receipt
        |> Map.put("operator_task_ir_status", "verified_pending_execution")
        |> Map.put("execution_readiness", %{"status" => "blocked", "ready_to_dispatch" => false})
      end,
      "native_workflow_operator_not_executed"
    )
  end

  test "an executed but incomplete diagnostic cannot downgrade a blocking guard to a warning" do
    assert_rejected_then_recovered(
      fn receipt ->
        Map.update!(
          receipt,
          "result",
          &Map.drop(&1, ~w(thermal_peak_flux_magnitude thermal_flux_peak_magnitude))
        )
      end,
      "payload.thermal_peak_flux_magnitude",
      ["heat_model", "solve_heat", "extract_thermal_diagnostics"]
    )
  end

  defp assert_rejected_then_recovered(
         mutate_receipt,
         reason,
         completed_nodes \\ ["heat_model", "solve_heat"]
       ) do
    agent =
      DiagnosticAgent.start!([
        DiagnosticAgent.solver("solve_heat_plane_quad_2d", [guard_heat_result()]),
        DiagnosticAgent.diagnostic(
          "extract_thermal_diagnostics",
          "extract.thermal_result_diagnostics",
          mutate_receipt
        ),
        DiagnosticAgent.solver("solve_heat_plane_quad_2d", [guard_heat_result()]),
        DiagnosticAgent.diagnostic(
          "extract_thermal_diagnostics",
          "extract.thermal_result_diagnostics"
        )
      ])

    WorkflowApi.configure_fake_agent_pool(WorkflowApi.await_fake_agent_port())
    failed = submit_guard()
    assert failed["job"]["status"] == "failed"
    assert failed["job"]["message"] =~ reason
    refute get_in(failed, ["result", "artifacts", "json_output.json"])
    assert {:ok, retained} = AnalysisResultStore.get(failed["job"]["job_id"])
    uncertain? = reason == "native_workflow_recovery_dispatch_unconfirmed"
    expected_state = if uncertain?, do: "recovery_blocked", else: "failed"
    assert retained["_workflow_recovery"]["state"] == expected_state
    refute Map.has_key?(retained["artifacts"], "extract_thermal_diagnostics.summary")
    refute Map.has_key?(retained["artifacts"], "json_output.json")
    assert Enum.map(retained["progress_events"], & &1["node_id"]) == completed_nodes

    if uncertain? do
      assert retained["_workflow_recovery"]["native_dispatch_evidence"]
      assert [%{consecutive_failures: 1}] = AgentPool.endpoints()
      # Explicitly re-register the repaired fixture before a new job, not a replay.
      AgentPool.reload()
    end

    recovered = submit_guard()
    assert recovered["job"]["status"] == "completed"
    refute recovered["job"]["job_id"] == failed["job"]["job_id"]
    assert recovered["result"]["failed_nodes"] == []
    exported = recovered["result"]["artifacts"]["json_output.json"]
    assert Jason.decode!(exported["content"])["guard_status"] == "block"
    [_, rejected_request, _, recovered_request] = DiagnosticAgent.assert_complete(agent)
    rejected_task = rejected_request["params"]["task_ir"]
    recovered_task = recovered_request["params"]["task_ir"]
    refute rejected_task["task_id"] == recovered_task["task_id"]
    assert rejected_task["input_artifact"] == recovered_task["input_artifact"]
    assert_diagnostic_input(recovered_request, recovered, "solve_heat.result")
  end

  defp submit_guard do
    WorkflowApi.submit_catalog_workflow_job(
      @opts,
      "workflow.heat-plane-quad-guard-json",
      WorkflowApi.heat_to_thermo_quad_input_artifacts()
    )
  end

  defp assert_diagnostic_input(request, payload, artifact_key) do
    task = request["params"]["task_ir"]
    assert task["config"] == %{}
    assert task["input_artifact"] == payload["result"]["artifacts"][artifact_key]
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
          %{
            "id" => "h0",
            "temperature_delta" => 70.0,
            "displacement_x" => 0.0,
            "displacement_y" => 0.0
          },
          %{
            "id" => "h1",
            "temperature_delta" => 45.0,
            "displacement_x" => 0.001,
            "displacement_y" => 0.0
          },
          %{
            "id" => "h2",
            "temperature_delta" => 20.0,
            "displacement_x" => 0.0015,
            "displacement_y" => 0.0
          },
          %{
            "id" => "h3",
            "temperature_delta" => 20.0,
            "displacement_x" => 0.0012,
            "displacement_y" => 0.0
          }
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
