defmodule KyuubikiWeb.Orchestra.EngineTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.Engine

  test "exposes workflow and operator catalogs from one compact engine facade" do
    assert %{"workflows" => workflows} = Engine.list_workflow_catalog(%{"q" => "material"})
    workflow_ids = MapSet.new(workflows, & &1["id"])

    assert "workflow.material-exploration-snapshot-json" in workflow_ids

    assert {:ok, %{"operator" => %{"id" => "transform.build_material_exploration_snapshot"}}} =
             Engine.fetch_operator_catalog_entry("transform.build_material_exploration_snapshot")
  end

  test "runs a catalog workflow graph through the engine facade" do
    assert {:ok, graph} = Engine.workflow_graph_by_id("workflow.material-next-round-request-json")

    assert {:ok, result} =
             Engine.run_workflow_graph(%{
               "graph" => graph,
               "input_artifacts" => %{
                 "results_input" => %{
                   "results" => [
                     %{
                       "experiment_id" => "coupon-1",
                       "candidate_id" => "aluminum",
                       "priority" => 1,
                       "expected_score" => 0.58,
                       "observed_score" => 0.62,
                       "passed" => true
                     }
                   ]
                 }
               }
             })

    assert %{"format" => "json", "content" => content} = result["artifacts"]["json_output.json"]

    summary = Jason.decode!(content)

    assert result["workflow_id"] == "workflow.material-next-round-request-json"
    assert summary["material_next_round_action"] == "run_more_experiments"
    assert summary["material_next_round_seed_candidate_id"] == "aluminum"
  end

  test "records operator task IR references in node run lineage" do
    assert {:ok, graph} = Engine.workflow_graph_by_id("workflow.material-fatigue-life-json")

    assert {:ok, result} =
             Engine.run_workflow_graph(%{
               "graph" => graph,
               "input_artifacts" => %{
                 "candidates_input" => %{
                   "candidates" => %{
                     "aluminum" => %{
                       "stress_amplitude" => 95.0,
                       "fatigue_strength" => 120.0
                     }
                   }
                 }
               }
             })

    fatigue_run =
      Enum.find(
        result["node_runs"],
        &(&1["operator_id"] == "transform.estimate_material_fatigue_life")
      )

    assert fatigue_run["task_ir_ref"]["schema_version"] == "kyuubiki.operator-task-ir/v1"
    assert fatigue_run["task_ir_ref"]["agent_rpc_method"] == "run_operator_task_ir"
    assert fatigue_run["task_ir_ref"]["operator_kind"] == "transform"
    assert is_binary(fatigue_run["task_ir_ref"]["descriptor_digest"])
    assert "workflow_transform_runtime" in fatigue_run["task_ir_ref"]["required_capabilities"]
  end
end
