defmodule KyuubikiWeb.Orchestra.OperatorExecutionProgramTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorExecutionProgram

  test "builds solver RPC execution programs" do
    program =
      OperatorExecutionProgram.build(
        %{
          "id" => "solve.heat_plane_quad_2d",
          "family" => "heat_plane_quad_2d",
          "kind" => "solver",
          "operator_category_id" => "physics_solver"
        },
        %{
          "package_ref" => "orchestra://operator-package/solve.heat_plane_quad_2d",
          "package_version" => "library-managed"
        },
        %{}
      )

    assert program["schema_version"] == OperatorExecutionProgram.schema_version()
    assert program["runtime_protocol"] == "kyuubiki.solver-rpc/v1"
    assert program["abi"]["kind"] == "solver_rpc"

    assert program["entrypoint"] == %{
             "kind" => "solver_method",
             "name" => "solve_heat_plane_quad_2d"
           }

    assert program["bindings"]["input_artifact"] == "task.input_artifact"
  end

  test "builds operator-task execution programs with workflow node binding" do
    node = %{
      "id" => "rank_candidates",
      "inputs" => [%{"id" => "candidates", "artifact_type" => "report/summary_collection"}],
      "outputs" => [%{"id" => "ranking", "artifact_type" => "report/summary"}]
    }

    program =
      OperatorExecutionProgram.build(
        %{
          "id" => "transform.rank_material_candidates",
          "family" => "material_candidate_rank",
          "kind" => "transform",
          "operator_category_id" => "optimization_selection"
        },
        %{
          "package_ref" => "orchestra://operator-package/transform.rank_material_candidates",
          "integrity" => %{"sha256" => "abc"}
        },
        node
      )

    assert program["runtime_protocol"] == "kyuubiki.operator-execution/v1"
    assert program["abi"]["kind"] == "operator_task"
    assert program["entrypoint"]["kind"] == "operator_id"
    assert program["entrypoint"]["name"] == "transform.rank_material_candidates"
    assert program["package_integrity"] == %{"sha256" => "abc"}
    assert program["node_binding"]["node_id"] == "rank_candidates"
    assert program["node_binding"]["input_ports"] == node["inputs"]
  end
end
