defmodule KyuubikiWeb.WorkflowExecutionEvidenceTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.WorkflowGraphRunner
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

  test "skipped joins do not consume available inputs or invoke their operators" do
    for value <- [false, 0, nil],
        mode <- [:inactive, :failed, :missing],
        reverse <- [false, true] do
      request = Fixture.skipped(value, mode, reverse)

      opts = [
        execute_solve: fn _, _, _ -> flunk("unexpected solve") end,
        execute_extract: fn _, _, _ -> flunk("unexpected extract") end,
        execute_export: fn _, _, _ -> flunk("unexpected export") end,
        execute_transform: fn
          "transform.first_available", payload, _ -> {:ok, payload}
          _, _, _ -> flunk("skipped strict join must not invoke the operator")
        end
      ]

      assert {:ok, result} =
               WorkflowGraphRunner.run(request["graph"], request["input_artifacts"], opts)

      for trace <- result["node_runs"], trace["status"] == "skipped" do
        assert trace["consumed_artifacts"] == [], trace["node_id"]
        assert trace["produced_artifacts"] == []
        assert Fixture.published(result, trace["node_id"]) == []
      end

      for id <- ["strict", "blocked", "strict_result"] do
        assert Fixture.trace(result, id)["status"] == "skipped"
      end

      expected_failures =
        case mode do
          :inactive -> []
          :failed -> ["gate"]
          :missing -> ["flag"]
        end

      assert result["failed_nodes"] == expected_failures

      assert Fixture.trace(result, "fallback")["consumed_artifacts"] == ["a.payload"]
      assert Map.fetch!(result["artifacts"], "result.payload") === value
      assert Map.fetch!(result["artifacts"], "raw.payload") === value
      assert_publications(result)
    end
  end

  test "executed failures retain their inputs while subsequent skipped nodes consume nothing" do
    for value <- [false, 0, nil], reverse <- [false, true] do
      assert {:ok, result} = Engine.run_workflow_graph(Fixture.skipped(value, :active, reverse))
      assert result["failed_nodes"] == ["strict"]

      assert Fixture.trace(result, "strict")["consumed_artifacts"] == [
               "a.payload",
               "gate.if_true"
             ]

      assert Fixture.trace(result, "strict_result")["consumed_artifacts"] == []
      assert Fixture.trace(result, "blocked")["status"] == "completed"

      assert Fixture.trace(result, "blocked")["consumed_artifacts"] == [
               "a.payload",
               "gate.if_true"
             ]

      assert Fixture.trace(result, "fallback")["consumed_artifacts"] == ["gate.if_true"]
      assert result["artifacts"]["result.payload"] == true
      assert Map.fetch!(result["artifacts"], "raw.payload") === value
      assert_publications(result)
    end
  end
end
