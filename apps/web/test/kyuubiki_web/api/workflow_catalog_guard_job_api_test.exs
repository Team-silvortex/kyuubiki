defmodule KyuubikiWeb.Api.WorkflowCatalogGuardJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits an electrostatic quad catalog workflow as an asynchronous job" do
    {:ok, _pid} = WorkflowApi.start_electrostatic_quad_summary_session()
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-plane-quad-2d",
        WorkflowApi.electrostatic_plane_quad_input_artifacts()
      )

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["workflow_id"] == "workflow.electrostatic-plane-quad-2d"
    assert length(result_payload["result"]["completed_nodes"]) == 5

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    assert exported["format"] == "json"

    summary = Jason.decode!(exported["content"])
    assert summary["max_potential"] == 10.0
    assert summary["max_electric_field"] == 10.0
    assert summary["max_flux_density"] == 20.0
  end

  test "submits a guarded electrostatic to heat to thermo workflow and blocks the coupled branch" do
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:blocked)
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-preheat-guard-heat-thermo-json",
        WorkflowApi.electrostatic_plane_quad_input_artifacts()
      )

    WorkflowApi.assert_guard_blocked(
      result_payload,
      "workflow.electrostatic-preheat-guard-heat-thermo-json",
      [
        "bridge_field_to_heat",
        "solve_heat",
        "bridge_temperature",
        "solve_thermo",
        "extract_thermo_summary"
      ],
      10.0
    )
  end

  test "submits a guarded electrostatic to heat to thermo workflow and continues the coupled branch" do
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:continued)
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-preheat-guard-heat-thermo-json",
        WorkflowApi.electrostatic_plane_quad_input_artifacts()
      )

    WorkflowApi.assert_guard_continued(
      result_payload,
      "workflow.electrostatic-preheat-guard-heat-thermo-json",
      [70.0, 45.0, 20.0, 20.0],
      %{
        "max_displacement" => 0.0015,
        "max_stress" => 18_000_000.0,
        "max_temperature_delta" => 70.0
      }
    )
  end
end
