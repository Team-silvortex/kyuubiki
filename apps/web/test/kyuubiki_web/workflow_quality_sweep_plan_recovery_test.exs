defmodule KyuubikiWeb.WorkflowQualitySweepPlanRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-sweep-plan-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  test "direct plan errors block task publication and fresh repairs preserve metadata" do
    for id <-
          ~w(enabled_stop wrong_strategy wrong_focus_state rejected_enabled declared_count_stale report_failed),
        policy <- ~w(fail skip) do
      graph = graph(policy)
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
        assert {:error, {:workflow_node_error, "materialize", reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["materialize"]
        assert rejected["completed_nodes"] == ["input"]

        for key <- ~w(materialize.result expand.result batch.result export.json),
            do: refute(Map.has_key?(rejected["artifacts"], key))
      end

      refute_received {^ref, :export}
      repaired = Enum.find(@fixtures, &(&1["id"] == "repair"))["payload"]
      assert {:ok, fresh} = Runner.run(graph, %{"input" => repaired}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      batch = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert batch["task_count"] == 2

      for task <- batch["tasks"],
          field <- ~w(coupled_readiness optimization_hint repair_strategy repair_focus),
          do: assert(task["metadata"][field] == repaired[field])
    end
  end

  test "generated plans reject single field corruption and preserve valid repair paths" do
    for state <- ~w(ready warn block) do
      report = %{"coupled_readiness_state" => state}

      assert {:ok, decision} =
               Runtime.run_transform_operator(
                 "transform.prepare_quality_next_round_request",
                 %{
                   "ranking" => [
                     %{
                       "candidate_id" => "a",
                       "score" => 1,
                       "ready" => true,
                       "coupled_readiness" => report
                     }
                   ]
                 },
                 %{"target_score" => 0.1, "search_space" => %{"temperature_delta" => [120, 160]}}
               )

      assert {:ok, plan} =
               Runtime.run_transform_operator(
                 "transform.build_quality_parameter_sweep_plan",
                 decision,
                 %{}
               )

      for {field, value} <- [
            {"sweep_action", "stop"},
            {"source_rejected_candidates", [%{"candidate_id" => "bad"}]},
            {"source_ranking_complete", false},
            {"converged", false},
            {"repair_strategy", "unknown"},
            {"repair_focus", %{"domain" => "mismatched"}},
            {"case_count_estimate", 3}
          ] do
        changed = plan |> Map.put(field, value) |> Jason.encode!() |> Jason.decode!()

        assert {:error, _} =
                 Runtime.run_transform_operator(
                   "transform.materialize_quality_sweep_expansion",
                   changed,
                   %{}
                 )
      end

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
        assert item["metadata"]["coupled_readiness"] == report
        assert item["metadata"]["repair_strategy"] == plan["repair_strategy"]
      end
    end
  end

  test "disabled and billion sample deferred plans never expand cases" do
    for id <- ~w(stop disabled_repair disabled_incomplete deferred_limit_raise) do
      fixture = Enum.find(@fixtures, &(&1["id"] == id))

      assert {:ok, materialized} =
               Runtime.run_transform_operator(
                 "transform.materialize_quality_sweep_expansion",
                 fixture["payload"],
                 fixture["config"]
               )

      assert {:ok, cases} =
               Runtime.run_transform_operator(
                 "transform.expand_parameter_sweep",
                 materialized,
                 %{}
               )

      assert cases["case_count"] == 0
      assert cases["cases"] == []
    end
  end

  defp graph(policy) do
    graph = Fixture.graph(policy)
    nodes = Enum.reject(graph["nodes"], &(&1["id"] == "plan"))

    edges =
      graph["edges"]
      |> Enum.reject(&(&1["to"]["node"] == "plan"))
      |> Enum.map(fn edge ->
        if edge["to"]["node"] == "materialize",
          do: Map.put(edge, "from", %{"node" => "input", "port" => "result"}),
          else: edge
      end)

    %{graph | "nodes" => nodes, "edges" => edges}
  end
end
