defmodule KyuubikiWeb.Api.WorkflowCatalogTriangleGuardJobApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits a guarded electrostatic triangle to heat to thermo workflow and blocks the coupled branch" do
    {:ok, _pid} = WorkflowApi.start_guarded_triangle_sessions(:blocked)
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
        WorkflowApi.electrostatic_plane_triangle_input_artifacts()
      )

    WorkflowApi.assert_guard_blocked(
      result_payload,
      "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
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

  test "submits a guarded electrostatic triangle to heat to thermo workflow and continues the coupled branch" do
    {:ok, _pid} = WorkflowApi.start_guarded_triangle_sessions(:continued)
    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        @opts,
        "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
        WorkflowApi.electrostatic_plane_triangle_input_artifacts()
      )

    WorkflowApi.assert_guard_continued(
      result_payload,
      "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
      [75.0, 55.0, 35.0],
      %{
        "max_displacement" => 0.0025,
        "max_stress" => 22_500_000.0,
        "max_temperature_delta" => 75.0
      }
    )
  end
end
