defmodule KyuubikiWeb.Orchestra.OperatorTaskIRTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "lowers a material transform operator into a stable engine task envelope" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.evaluate_material_thermal_shock",
               %{
                 "temperature_delta" => 160.0,
                 "thermal_expansion" => 1.2e-5,
                 "youngs_modulus" => 70.0e9,
                 "yield_strength" => 320.0e6
               },
               %{"constraint_factor" => 0.7},
               task_id: "task-material-shock-1",
               orchestration_context: %{"orch_id" => "orch-main"},
               dataset_contract: %{"id" => "kyuubiki.dataset.material_thermal_shock/v1"}
             )

    assert task["schema_version"] == "kyuubiki.operator-task-ir/v1"
    assert task["task_id"] == "task-material-shock-1"
    assert task["operator"]["id"] == "transform.evaluate_material_thermal_shock"
    assert task["operator"]["kind"] == "transform"
    assert task["config"] == %{"constraint_factor" => 0.7}
    assert task["orchestration_context"] == %{"orch_id" => "orch-main"}
    assert task["runtime_hints"]["agent_fetchable"] == true
    assert "workflow_transform_runtime" in task["runtime_hints"]["required_capabilities"]
    assert is_binary(task["integrity"]["descriptor_digest"])
  end

  test "lowers a workflow node while preserving node identity and ports" do
    node = %{
      "id" => "estimate_fatigue",
      "kind" => "transform",
      "operator_id" => "transform.estimate_material_fatigue_life",
      "config" => %{"fatigue_strength" => 120.0},
      "inputs" => [%{"id" => "candidates", "artifact_type" => "report/summary_collection"}],
      "outputs" => [%{"id" => "fatigue", "artifact_type" => "report/summary"}]
    }

    assert {:ok, task} =
             OperatorTaskIR.from_node(node, %{"candidates" => %{"aluminum" => %{}}})

    assert task["task_id"] ==
             "operator-task:estimate_fatigue:transform.estimate_material_fatigue_life"

    assert task["node"]["id"] == "estimate_fatigue"
    assert task["node"]["inputs"] == node["inputs"]
    assert task["node"]["outputs"] == node["outputs"]
    assert task["config"] == %{"fatigue_strength" => 120.0}
  end

  test "exposes the future agent RPC method and params shape" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    assert OperatorTaskIR.agent_rpc_method() == "run_operator_task_ir"
    assert OperatorTaskIR.agent_rpc_params(task) == %{"task_ir" => task}
  end

  test "exposes agent routing opts from runtime hints" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{},
               task_id: "operator-task:rank-materials",
               placement_tags: ["materials", "ranking"],
               orchestration_context: %{"orch_id" => "orch-main"}
             )

    assert OperatorTaskIR.agent_routing_opts(task) == [
             job_id: "operator-task:rank-materials",
             orchestration: %{"orch_id" => "orch-main"},
             placement_tags: ["materials", "ranking"],
             required_capabilities: ["workflow_transform_runtime"]
           ]
  end
end
