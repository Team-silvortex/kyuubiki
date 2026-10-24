defmodule KyuubikiWeb.WorkflowExecutionEvidenceTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowExecutionEvidence, as: Fixture

  test "condition traces name only actual publications, including false zero and null" do
    for sample <- Fixture.cases(),
        reverse <- [false, true],
        backup_first <- [false, true],
        options <- [%{}, %{"include_artifacts" => false}] do
      request = Fixture.request(sample["value"], reverse, backup_first)

      assert {:ok, result} =
               Engine.run_workflow_graph(Map.put(request, "response_options", options))

      assert result["failed_nodes"] == []

      assert Fixture.trace(result, "branch")["produced_artifacts"] == [
               "branch.#{sample["chosen_output"]}"
             ]

      assert Fixture.trace(result, "choose")["consumed_artifacts"] == [
               if(backup_first, do: "b.payload", else: "a.payload")
             ]

      assert_publications(result)

      if Map.get(options, "include_artifacts", true) do
        assert Map.fetch!(result["artifacts"], "branch.#{sample["chosen_output"]}") ==
                 sample["value"]

        refute Map.has_key?(result["artifacts"], "branch.unused")
      else
        refute Map.has_key?(result, "artifacts")
      end
    end
  end

  test "failed fallback traces keep the selected source without claiming unused alternatives" do
    for aliased <- [false, true],
        selected <- if(aliased, do: ["a"], else: ["a", "b"]),
        reverse <- [false, true],
        nested <- [false, true] do
      request = Fixture.failure(selected, aliased, reverse)

      request =
        if nested,
          do:
            Fixture.node(
              request,
              "choose",
              &Map.put(&1, "config", %{"recovery" => %{"on_error" => "skip"}})
            ),
          else: request

      assert {:ok, result} = Engine.run_workflow_graph(request)
      assert result["failed_nodes"] == ["choose"]
      assert Enum.sort(result["skipped_nodes"]) == ["branch", "no", "yes"]
      assert Fixture.trace(result, "choose")["consumed_artifacts"] == ["#{selected}.payload"]
      assert Fixture.trace(result, "choose")["error_message"] =~ "length security budget"
      assert result["branch_decisions"] == []
      assert_publications(result)

      assert result["artifacts"]["raw_#{selected}.payload"] ==
               request["input_artifacts"][selected]
    end
  end

  test "failed named inputs preserve both bindings even when the source is aliased" do
    for aliased <- [false, true] do
      request =
        Fixture.request(nil)
        |> Fixture.alias_backup(aliased)
        |> Fixture.node(
          "choose",
          &Map.put(&1, "operator_id", "transform.benchmark_structural_pair")
        )

      assert {:ok, result} = Engine.run_workflow_graph(request)
      assert result["failed_nodes"] == ["choose"]

      assert Fixture.trace(result, "choose")["consumed_artifacts"] == [
               "a.payload",
               if(aliased, do: "a.payload", else: "b.payload")
             ]

      assert_publications(result)
    end
  end

  defp assert_publications(result) do
    for trace <- result["node_runs"] do
      assert Enum.sort(trace["produced_artifacts"]) == Fixture.published(result, trace["node_id"])
      if trace["status"] != "completed", do: assert(trace["produced_artifacts"] == [])
    end
  end
end
