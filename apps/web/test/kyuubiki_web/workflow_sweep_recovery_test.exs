defmodule KyuubikiWeb.WorkflowSweepRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor

  for policy <- ~w(fail skip) do
    @policy policy
    test "#{policy}: invalid planning or batch configuration cannot export, corrected chain executes" do
      ref = make_ref()

      for {node, overrides} <- [
            {"plan", %{"plan" => %{"samples_per_axis" => 2.5}}},
            {"materialize",
             %{
               "plan" => %{"base" => %{"temperature_delta" => 0}},
               "materialize" => %{"max_cases" => nil}
             }},
            {"expand", %{"expand" => %{"max_cases" => 1.5}}},
            {"batch", %{"batch" => %{"required_capabilities" => [nil]}}}
          ] do
        rejected =
          Runner.run(
            Fixture.graph(@policy, overrides),
            %{"input" => Fixture.request()},
            options(ref)
          )

        assert_rejected(rejected, node, @policy)
        refute_received {^ref, :export}
      end

      assert {:ok, run} =
               Runner.run(Fixture.graph(@policy), %{"input" => Fixture.request()}, options(ref))

      assert run["completed_nodes"] == ~w(input plan materialize expand batch export output)
      assert_received {^ref, :export}
      batch = Jason.decode!(run["artifacts"]["export.json"]["content"])
      Fixture.assert_batch(batch)
      assert {:ok, prepared} = Executor.prepare_batch(batch)
      assert prepared["verified_count"] == 2
      assert {:ok, execution} = Executor.execute_batch(batch)
      Fixture.assert_execution(execution)
    end

    test "#{policy}: blocked scans remain diagnostic only and cannot issue task batches" do
      ref = make_ref()
      incomplete = Map.put(Fixture.request(), "source_ranking_complete", false)

      for {payload, overrides} <- [
            {incomplete, %{}},
            {Fixture.request(),
             %{
               "plan" => %{"max_cases" => 1},
               "materialize" => %{"max_cases" => 4},
               "expand" => %{"max_cases" => 4}
             }}
          ] do
        rejected =
          Runner.run(Fixture.graph(@policy, overrides), %{"input" => payload}, options(ref))

        assert_rejected(rejected, "batch", @policy)
        refute_received {^ref, :export}
      end
    end
  end

  defp assert_rejected(result, node, "fail"),
    do: assert({:error, {:workflow_node_error, ^node, _}} = result)

  defp assert_rejected(result, node, "skip") do
    assert {:ok, run} = result
    assert node in run["failed_nodes"]
    refute Map.has_key?(run["artifacts"], "export.json")
    refute Map.has_key?(run["artifacts"], "batch.result")
  end

  defp options(ref) do
    owner = self()

    [
      execute_transform: &Runtime.run_transform_operator/3,
      execute_solve: fn _, _, _ -> flunk("generation must not dispatch a solver") end,
      execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
      execute_export: fn op, payload, config ->
        send(owner, {ref, :export})
        Runtime.run_export_operator(op, payload, config)
      end
    ]
  end
end
