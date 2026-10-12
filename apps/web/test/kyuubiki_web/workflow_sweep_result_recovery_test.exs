defmodule KyuubikiWeb.WorkflowSweepResultRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.SweepResultRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for policy <- ~w(fail skip) do
    @policy policy
    test "#{policy}: damaged result evidence cannot export a winner and fresh runs recover" do
      ref = make_ref()

      for {name, payload, node, overrides} <- Fixture.corruptions() do
        result =
          Runner.run(Fixture.graph(@policy, overrides), %{"input" => payload}, options(ref))

        if @policy == "fail" do
          assert {:error, {:workflow_node_error, ^node, _}} = result, name
        else
          assert {:ok, run} = result
          assert run["failed_nodes"] == [node], name
          refute Map.has_key?(run["artifacts"], "score.result"), name
          refute Map.has_key?(run["artifacts"], "export.json"), name
        end

        refute_received {^ref, :export}

        assert {:ok, recovered} =
                 Runner.run(Fixture.graph(@policy), %{"input" => Fixture.valid()}, options(ref))

        assert_received {^ref, :export}
        report = Jason.decode!(recovered["artifacts"]["export.json"]["content"])
        assert report["best"]["case_id"] == "a"
        assert report["best"]["mass"] == 1
        assert report["scored_count"] == 2
      end
    end
  end

  test "actual executed batches return through shuffled join summary score and JSON export" do
    {input, config} = Fixture.actual_batch()

    assert {:ok, run} =
             Runner.run(Fixture.graph("fail", config), %{"input" => input}, options(make_ref()))

    report = Jason.decode!(run["artifacts"]["export.json"]["content"])
    Fixture.assert_actual(report)
  end

  defp options(ref) do
    owner = self()

    [
      execute_transform: &Runtime.run_transform_operator/3,
      execute_solve: fn _, _, _ -> flunk("result collection must not re-execute solvers") end,
      execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
      execute_export: fn op, payload, config ->
        send(owner, {ref, :export})
        Runtime.run_export_operator(op, payload, config)
      end
    ]
  end
end
