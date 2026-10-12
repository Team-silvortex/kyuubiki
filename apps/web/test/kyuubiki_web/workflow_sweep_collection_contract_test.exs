defmodule KyuubikiWeb.WorkflowSweepCollectionContractTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-sweep-collection-contract.json",
              __DIR__
            )
  for fixture <- @fixtures |> File.read!() |> Jason.decode!() do
    @fixture fixture
    test "shared sweep collection contract: #{fixture["id"]}" do
      result =
        Runtime.run_transform_operator(
          @fixture["operator"],
          @fixture["payload"],
          @fixture["config"]
        )

      if @fixture["error"] do
        assert {:error, _} = result
      else
        assert {:ok, actual} = result
        assert_fields(actual, @fixture["expected"])
      end
    end
  end

  test "fresh collection removes the prior custom summary without mutating input" do
    for received <- [false, true] do
      payload = %{
        "joined_summary_field" => "old_quality",
        "cases" => [
          %{
            "id" => "a",
            "old_quality" => %{"mass" => 0},
            "summary" => %{"mass" => 0},
            "result" => %{"mass" => 0},
            "result_error" => "old",
            "metadata" => %{"round" => 1}
          }
        ],
        "results" =>
          if(received, do: [%{"case_id" => "a", "summary" => %{"mass" => 5}}], else: [])
      }

      assert {:ok, joined} =
               Runtime.run_transform_operator(
                 "transform.join_parameter_sweep_results",
                 payload,
                 %{"output_field" => "fresh"}
               )

      entry = hd(joined["cases"])

      for field <- ~w(old_quality summary result result_error),
          do: refute(Map.has_key?(entry, field))

      assert entry["metadata"] == %{"round" => 1}
      assert hd(payload["cases"])["old_quality"] == %{"mass" => 0}
      result = Runtime.run_transform_operator("transform.summarize_parameter_sweep", joined, %{})

      if received do
        assert {:ok, table} = result
        assert hd(table["rows"])["mass"] == 5
      else
        assert {:error, _} = result
      end
    end
  end

  test "metadata cannot enter collection scoring and fresh graphs recover without solver replay" do
    alias KyuubikiWeb.TestSupport.SweepResultRecovery, as: Fixture
    alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
    owner = self()
    ref = make_ref()

    options = [
      execute_transform: &Runtime.run_transform_operator/3,
      execute_solve: fn _, _, _ -> flunk("collection must not run solvers") end,
      execute_extract: fn _, _, _ -> flunk("collection must not extract new data") end,
      execute_export: fn operator, payload, config ->
        send(owner, {ref, :export})
        Runtime.run_export_operator(operator, payload, config)
      end
    ]

    input =
      Map.update!(Fixture.valid(), "results", fn results ->
        Enum.map(results, &Map.put(&1, "metadata", %{"mass" => 0}))
      end)

    for policy <- ~w(fail skip) do
      graph = Fixture.graph(policy, %{"join" => %{"summary_field" => "metadata"}})
      result = Runner.run(graph, %{"input" => input}, options)

      if policy == "fail" do
        assert {:error, {:workflow_node_error, "join", reason}} = result
        assert reason =~ "reserved"
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["join"]
        refute Map.has_key?(rejected["artifacts"], "score.result")
        refute Map.has_key?(rejected["artifacts"], "export.json")
      end

      refute_received {^ref, :export}
      assert {:ok, recovered} = Runner.run(Fixture.graph(policy), %{"input" => input}, options)
      assert_received {^ref, :export}
      report = Jason.decode!(recovered["artifacts"]["export.json"]["content"])
      assert report["best"]["case_id"] == "a"
      assert report["best"]["mass"] == 1
    end
  end

  defp assert_fields(actual, expected) when is_map(expected) do
    for {key, value} <- expected do
      assert Map.has_key?(actual, key)
      assert_fields(actual[key], value)
    end
  end

  defp assert_fields(actual, expected) when is_list(expected) do
    assert is_list(actual)
    assert length(actual) == length(expected)
    for {value, target} <- Enum.zip(actual, expected), do: assert_fields(value, target)
  end

  defp assert_fields(actual, expected), do: assert(actual == expected)
end
