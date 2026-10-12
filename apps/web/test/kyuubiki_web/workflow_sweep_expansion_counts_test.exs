defmodule KyuubikiWeb.WorkflowSweepExpansionCountsTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-sweep-expansion-counts.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  for fixture <- @fixtures do
    test "expansion count: #{fixture["id"]}" do
      fixture = unquote(Macro.escape(fixture))
      payload = fixture["payload"] |> Jason.encode!() |> Jason.decode!()

      result =
        Runtime.run_transform_operator(
          "transform.expand_parameter_sweep",
          payload,
          fixture["config"]
        )

      if fixture["error"] do
        assert {:error, reason} = result
        assert is_binary(reason)
      else
        assert {:ok, expanded} = result
        for {key, expected} <- fixture["expected"], do: assert(expanded[key] == expected)
      end
    end
  end

  test "125 count mirror placements accept integral floats but reject stale and null values" do
    source = Enum.find(@fixtures, &(&1["id"] == "wrapped_valid"))["payload"]
    choices = [:absent, 2, 2.0, nil, 3]

    for a <- choices, b <- choices, c <- choices do
      payload =
        source
        |> put_count([], a)
        |> put_count(["payload"], b)
        |> put_count(["payload", "case_metadata", "sweep_budget"], c)
        |> Jason.encode!()
        |> Jason.decode!()

      result = Runtime.run_transform_operator("transform.expand_parameter_sweep", payload, %{})

      if Enum.all?([a, b, c], &(&1 == :absent || &1 == 2)) do
        assert {:ok, expanded} = result
        assert expanded["case_count"] == 2
      else
        assert {:error, reason} = result
        assert is_binary(reason)
      end
    end
  end

  test "corrupt counts stop task exports and corrected fresh runs recover" do
    good = Enum.find(@fixtures, &(&1["id"] == "wrapped_valid"))["payload"]

    for id <-
          ~w(raw_axis_stale wrapped_total_stale blocked_deferred_with_values blocked_metadata_budget_stale stopped_total_stale metadata_limit_too_small raw_budget_block diagnostic_count_conflict),
        policy <- ~w(fail skip) do
      graph = Fixture.expansion_graph(policy)
      owner = self()
      ref = make_ref()

      options = [
        execute_solve: fn _, _, _ -> flunk("count admission must not run a solver") end,
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
      assert {:ok, fresh} = Runner.run(graph, %{"input" => good}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      batch = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert batch["task_count"] == 2
      for task <- batch["tasks"], do: assert(task["metadata"] == good["payload"]["case_metadata"])
    end
  end

  test "generated deferred and concrete counts stay consistent without allocating samples" do
    for limit <- [1, 4] do
      assert {:ok, plan} =
               Runtime.run_transform_operator(
                 "transform.build_quality_parameter_sweep_plan",
                 %{
                   "request_payload" => %{"search_space" => %{"temperature_delta" => [120, 160]}}
                 },
                 %{"max_cases" => limit}
               )

      assert {:ok, expansion} =
               Runtime.run_transform_operator(
                 "transform.materialize_quality_sweep_expansion",
                 plan,
                 %{"max_cases" => limit}
               )

      mutations = [
        put_in(expansion, ["case_count_estimate"], 3),
        put_in(expansion, ["sweep_budget", "case_count_estimate"], 3),
        put_in(expansion, ["payload", "case_metadata", "sweep_budget", "case_count_estimate"], 3),
        update_in(expansion, ["payload", "axes"], fn [axis] ->
          [Map.put(axis, "value_count", 3)]
        end)
      ]

      for changed <- mutations do
        assert {:error, _} =
                 Runtime.run_transform_operator("transform.expand_parameter_sweep", changed, %{
                   "max_cases" => 10
                 })
      end

      assert {:ok, expanded} =
               Runtime.run_transform_operator("transform.expand_parameter_sweep", expansion, %{
                 "max_cases" => 10
               })

      assert expanded["case_count"] == if(limit == 1, do: 0, else: 2)
    end

    body = Enum.find(@fixtures, &(&1["id"] == "deferred_valid"))["payload"]["payload"]

    plan = %{
      "base" => body["base"],
      "axes" => body["axes"],
      "sweep_budget" => %{"case_budget_exceeded" => true},
      "case_count_estimate" => 1_000_000_000
    }

    for values <- [nil, [], [120, 160]] do
      ambiguous = update_in(plan, ["axes"], fn [axis] -> [Map.put(axis, "values", values)] end)

      assert {:error, _} =
               Runtime.run_transform_operator(
                 "transform.materialize_quality_sweep_expansion",
                 ambiguous,
                 %{}
               )
    end

    assert {:ok, expansion} =
             Runtime.run_transform_operator(
               "transform.materialize_quality_sweep_expansion",
               plan,
               %{"max_cases" => 2_000_000_000}
             )

    refute Map.has_key?(hd(expansion["payload"]["axes"]), "values")

    assert {:ok, expanded} =
             Runtime.run_transform_operator("transform.expand_parameter_sweep", expansion, %{
               "max_cases" => 2_000_000_000
             })

    assert expanded["case_count"] == 0
  end

  defp put_count(value, [], :absent), do: Map.delete(value, "case_count_estimate")
  defp put_count(value, [], count), do: Map.put(value, "case_count_estimate", count)
  defp put_count(value, path, count), do: update_in(value, path, &put_count(&1, [], count))
end
