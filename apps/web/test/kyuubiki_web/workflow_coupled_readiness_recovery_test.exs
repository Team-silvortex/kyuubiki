defmodule KyuubikiWeb.WorkflowCoupledReadinessRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualityRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-coupled-readiness-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  test "malformed direct sweep readiness returns a contract error, not a runtime exception" do
    for value <- ["ready", [], false] do
      assert {:error, reason} =
               Runtime.run_transform_operator(
                 "transform.build_quality_parameter_sweep_plan",
                 %{
                   "action" => "continue",
                   "request_payload" => %{
                     "search_space" => %{"temperature" => [1, 2]},
                     "coupled_readiness" => value
                   }
                 },
                 %{}
               )

      assert reason =~ "coupled_readiness"
    end
  end

  test "conflicting coupled reports cannot export decisions and corrected fresh runs recover" do
    for id <- ~w(row_objective_conflict row_payload_conflict hidden_nonconverged tail_malformed),
        policy <- ~w(fail skip) do
      graph = Fixture.graph("round", policy)
      ref = make_ref()
      owner = self()

      options = [
        execute_solve: fn _, _, _ -> flunk("controlled evidence must not run a solver") end,
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
        assert {:error, {:workflow_node_error, "round", reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["round"]
        assert rejected["completed_nodes"] == ["input"]
        refute Map.has_key?(rejected["artifacts"], "round.result")
        refute Map.has_key?(rejected["artifacts"], "export.json")
      end

      refute_received {^ref, :export}
      corrected = Enum.find(@fixtures, &(&1["id"] == "row_ready"))["payload"]
      assert {:ok, fresh} = Runner.run(graph, %{"input" => corrected}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      report = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert report["action"] == "stop"
      assert report["selected_candidate_ready"] == true
    end
  end

  test "partial coupled evidence retains repair or review intent through sweep transforms" do
    for {id, action, strategy} <- [
          {"null_row_preserves_inner_block", "replan", "repair_coupled_readiness_sweep"},
          {"row_false_only", "replan", "repair_coupled_readiness_sweep"},
          {"row_required_missing_only", "replan", "repair_coupled_readiness_sweep"},
          {"row_warning_domains_only", "continue", "review_coupled_readiness_sweep"}
        ] do
      payload = Enum.find(@fixtures, &(&1["id"] == id))["payload"]

      assert {:ok, decision} =
               Runtime.run_transform_operator(
                 "transform.prepare_quality_next_round_request",
                 payload,
                 %{"search_space" => %{"temperature" => [1, 2]}}
               )

      assert decision["action"] == action
      assert decision["selected_candidate_ready"] == (action != "replan")

      assert {:ok, plan} =
               Runtime.run_transform_operator(
                 "transform.build_quality_parameter_sweep_plan",
                 decision,
                 %{}
               )

      assert plan["repair_strategy"] == strategy
      assert plan["coupled_readiness"] == decision["selected_coupled_readiness"]

      assert {:ok, materialized} =
               Runtime.run_transform_operator(
                 "transform.materialize_quality_sweep_expansion",
                 plan,
                 %{}
               )

      assert {:ok, cases} =
               Runtime.run_transform_operator(
                 "transform.expand_parameter_sweep",
                 materialized,
                 %{}
               )

      assert cases["case_count"] == 2

      for item <- cases["cases"] do
        assert item["metadata"]["repair_strategy"] == strategy
        assert item["metadata"]["coupled_readiness"] == decision["selected_coupled_readiness"]
      end
    end
  end
end
