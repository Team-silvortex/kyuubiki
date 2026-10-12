defmodule KyuubikiWeb.WorkflowObjectiveRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.TestSupport.QualityRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for kind <- ~w(objective ranking round), policy <- ~w(fail skip) do
    @kind kind
    @policy policy
    test "#{kind}/#{policy}: admission failures cannot export and fresh runs recover" do
      ref = make_ref()
      graph = Fixture.graph(@kind, @policy)
      result = Runner.run(graph, %{"input" => Fixture.invalid(@kind)}, options(ref))

      if @policy == "fail" do
        assert {:error, {:workflow_node_error, @kind, reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, run} = result
        assert run["failed_nodes"] == [@kind]
        assert run["completed_nodes"] == ["input"]
        refute Map.has_key?(run["artifacts"], "export.json")
      end

      refute_received {^ref, :export}

      assert {:ok, recovered} =
               Runner.run(graph, %{"input" => Fixture.valid(@kind)}, options(ref))

      assert recovered["failed_nodes"] == []
      assert_received {^ref, :export}
      report = Jason.decode!(recovered["artifacts"]["export.json"]["content"])

      if @kind == "objective",
        do: assert(report["composite_quality_ready"]),
        else: assert(report["action"] == "stop")
    end
  end

  for policy <- ~w(fail skip) do
    @policy policy
    test "#{policy}: partial rankings export explicit replan, never a completed research decision" do
      ref = make_ref()
      graph = Fixture.graph("ranking", @policy)
      assert {:ok, partial} = Runner.run(graph, %{"input" => Fixture.partial()}, options(ref))
      report = Jason.decode!(partial["artifacts"]["export.json"]["content"])
      assert report["action"] == "replan"
      refute report["source_ranking_complete"]
      assert [%{"candidate_id" => "b"}] = report["source_rejected_candidates"]
      assert_received {^ref, :export}

      assert {:ok, complete} = Runner.run(graph, %{"input" => Fixture.repaired()}, options(ref))
      report = Jason.decode!(complete["artifacts"]["export.json"]["content"])
      assert report["action"] == "stop"
      assert report["source_ranking_complete"]
      assert report["source_rejected_candidates"] == []
      assert_received {^ref, :export}
    end
  end

  defp options(ref) do
    owner = self()

    [
      execute_solve: fn _, _, _ -> flunk("controlled summaries must not execute a solver") end,
      execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
      execute_transform: &Runtime.run_transform_operator/3,
      execute_export: fn operator, payload, config ->
        send(owner, {ref, :export})
        Runtime.run_export_operator(operator, payload, config)
      end
    ]
  end
end
