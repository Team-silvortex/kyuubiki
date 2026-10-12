defmodule KyuubikiWeb.WorkflowQualityRankingRecoveryTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.QualityRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-ranking-integrity.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  test "incoherent rankings cannot export decisions and corrected fresh runs recover" do
    for id <-
          ~w(descending_score selected_objective_score_mismatch tail_objective_score_mismatch duplicate_rank),
        policy <- ~w(fail skip) do
      fixture = Enum.find(@fixtures, &(&1["id"] == id))
      graph = Fixture.graph("round", policy)
      ref = make_ref()
      owner = self()

      options = [
        execute_solve: fn _, _, _ -> flunk("controlled rankings must not run a solver") end,
        execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
        execute_transform: &Runtime.run_transform_operator/3,
        execute_export: fn operator, payload, config ->
          send(owner, {ref, :export})
          Runtime.run_export_operator(operator, payload, config)
        end
      ]

      result = Runner.run(graph, %{"input" => fixture["payload"]}, options)

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

      corrected = %{
        "ranking" => [
          %{
            "candidate_id" => "a",
            "rank" => 1,
            "score" => 1,
            "ready" => true,
            "objective" => %{"composite_quality_score" => 1}
          },
          %{
            "candidate_id" => "b",
            "rank" => 2,
            "score" => 2,
            "ready" => true,
            "objective" => %{"composite_quality_score" => 2}
          }
        ]
      }

      assert {:ok, fresh} = Runner.run(graph, %{"input" => corrected}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      report = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert report["action"] == "stop"
      assert report["selected_candidate_id"] == "a"
    end
  end
end
