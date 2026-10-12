defmodule KyuubikiWeb.WorkflowQualitySweepExpansionRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-sweep-expansion-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  test "expansion errors block task publication and fresh repairs keep quality evidence" do
    for id <-
          ~w(wrapped_missing_repair raw_missing_review wrapped_wrong_focus payload_null_masks_config wrapped_blocked_budget failed_body),
        policy <- ~w(fail skip) do
      graph = Fixture.expansion_graph(policy)
      owner = self()
      ref = make_ref()

      options = [
        execute_solve: fn _, _, _ -> flunk("admission must not run a solver") end,
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
        assert {:error, {:workflow_node_error, "expand", reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["expand"]
        assert rejected["completed_nodes"] == ["input"]

        for key <- ~w(expand.result batch.result export.json),
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
          do: assert(task["metadata"] == repaired["payload"]["case_metadata"])
    end
  end

  test "125 expansion metadata alias placements cannot mask quality evidence" do
    source = Enum.find(@fixtures, &(&1["id"] == "repair"))["payload"]

    choices = [
      :absent,
      nil,
      source["payload"]["case_metadata"],
      %{
        "coupled_readiness" => %{"coupled_readiness_state" => "warn"},
        "optimization_hint" => %{"action" => "review_coupled_readiness"}
      },
      "invalid"
    ]

    for a <- choices, b <- choices, c <- choices do
      body = source["payload"] |> Map.delete("case_metadata") |> put_metadata(a)
      payload = %{source | "payload" => body, "config" => put_metadata(source["config"], b)}
      config = put_metadata(%{}, c)
      aliases = Enum.reject([a, b, c], &(&1 == :absent))

      accepted =
        Enum.all?(aliases, &(is_nil(&1) || is_map(&1))) && length(Enum.uniq(aliases)) <= 1

      exchanged = payload |> Jason.encode!() |> Jason.decode!()

      result =
        Runtime.run_transform_operator("transform.expand_parameter_sweep", exchanged, config)

      if accepted do
        assert {:ok, expanded} = result
        metadata = if aliases == [], do: %{}, else: hd(aliases)
        for item <- expanded["cases"], do: assert(item["metadata"] == metadata)
      else
        assert {:error, reason} = result
        assert is_binary(reason)
      end
    end
  end

  test "generated expansions reject corrupted evidence and keep valid repair controls" do
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

      assert {:ok, expansion} =
               Runtime.run_transform_operator(
                 "transform.materialize_quality_sweep_expansion",
                 plan,
                 %{}
               )

      for {field, value} <- [
            {"converged", false},
            {"repair_strategy", "invalid"},
            {"repair_focus", %{"domain" => "mismatched"}},
            {"source_ranking_complete", false},
            {"sweep_enabled", false},
            {"sweep_budget", %{"case_budget_exceeded" => true}}
          ] do
        changed = put_in(expansion, ["payload", "case_metadata", field], value)

        assert {:error, _} =
                 Runtime.run_transform_operator("transform.expand_parameter_sweep", changed, %{})
      end

      assert {:ok, expanded} =
               Runtime.run_transform_operator("transform.expand_parameter_sweep", expansion, %{})

      assert expanded["case_count"] == 2

      for item <- expanded["cases"],
          do: assert(item["metadata"] == expansion["payload"]["case_metadata"])
    end
  end

  defp put_metadata(value, :absent), do: value
  defp put_metadata(value, metadata), do: Map.put(value, "case_metadata", metadata)
end
