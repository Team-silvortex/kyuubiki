defmodule KyuubikiWeb.WorkflowQualitySweepRequestRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-sweep-request-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  test "direct request conflicts stop task publication and fresh explicit repairs recover" do
    for id <-
          ~w(stop_block request_ready_masks_block hint_conflict continue_block hidden_failed_convergence hint_warning_count_conflict),
        policy <- ~w(fail skip) do
      graph = Fixture.graph(policy)
      owner = self()
      ref = make_ref()

      options = [
        execute_solve: fn _, _, _ -> flunk("admission test must not run a solver") end,
        execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
        execute_transform: &Runtime.run_transform_operator/3,
        execute_export: fn operator, payload, config ->
          send(owner, {ref, :export})
          Runtime.run_export_operator(operator, payload, config)
        end
      ]

      bad = Enum.find(@fixtures, &(&1["id"] == id))["payload"]
      result = Runner.run(graph, %{"input" => bad}, options)

      if policy == "fail" do
        assert {:error, {:workflow_node_error, "plan", reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["plan"]
        assert rejected["completed_nodes"] == ["input"]

        for key <- ~w(plan.result materialize.result expand.result batch.result export.json),
            do: refute(Map.has_key?(rejected["artifacts"], key))
      end

      refute_received {^ref, :export}
      corrected = Enum.find(@fixtures, &(&1["id"] == "repair"))["payload"]
      assert {:ok, fresh} = Runner.run(graph, %{"input" => corrected}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      batch = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert batch["task_count"] == 2

      for task <- batch["tasks"] do
        assert task["metadata"]["repair_strategy"] == "repair_coupled_readiness_sweep"
        assert task["metadata"]["coupled_readiness"]["coupled_readiness_ready"] == false
      end
    end
  end
end
