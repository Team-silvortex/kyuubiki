defmodule KyuubikiWeb.Orchestra.ControlPlaneSurfaceTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.ControlPlaneSurface

  test "describes solver execution routes without making orchestra a solver host" do
    surface = ControlPlaneSurface.surface()
    route_ids = Enum.map(surface.solver_execution_routes, & &1.id)

    assert surface.owner == "orchestra-control-plane"
    assert "workflow_graph_execution" in route_ids
    assert "operator_task_ir_execution" in route_ids
    assert Enum.all?(surface.solver_execution_routes, &Map.has_key?(&1, :delegated_runtime))
  end

  test "validation gates cover all declared executable contracts" do
    surface = ControlPlaneSurface.surface()

    assert ControlPlaneSurface.validates_surface?()

    assert Enum.map(surface.validation_gates, & &1.id) == [
             "workflow_graph_shape",
             "operator_task_digest",
             "operator_task_batch_entry_rpc_mirror"
           ]
  end

  test "benchmark lanes and headless parity contracts are explicit" do
    assert Enum.any?(
             ControlPlaneSurface.benchmark_commands(),
             &String.contains?(&1, "workflow_catalog_report_test.exs")
           )

    assert ControlPlaneSurface.headless_parity_contracts() == [
             "headless_sdk_control_plane",
             "run_operator_task_ir"
           ]
  end
end
