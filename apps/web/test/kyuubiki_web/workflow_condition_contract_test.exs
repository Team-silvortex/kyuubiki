defmodule KyuubikiWeb.WorkflowConditionContractTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowConditionContract, as: Fixture
  alias KyuubikiWeb.{WorkflowGraphRunner, WorkflowOperatorRuntime}

  for sample <- Fixture.cases() do
    @sample sample
    test "shared condition decision: #{sample["id"]}" do
      for reverse <- [false, true] do
        request = Fixture.request(@sample, "fail", reverse)

        if path = @sample["error"] do
          assert {:error, {:workflow_node_error, "gate", reason}} =
                   Engine.run_workflow_graph(request)

          assert inspect(reason) =~ path
        else
          assert {:ok, result} = Engine.run_workflow_graph(request)
          assert result["failed_nodes"] == []
          assert result["artifacts"]["raw.payload"] == @sample["payload"]

          assert [%{"node_id" => "gate", "predicate_result" => decision, "chosen_output" => port}] =
                   result["branch_decisions"]

          assert decision == @sample["decision"]
          assert port == if(decision, do: "if_true", else: "if_false")
          assert Map.has_key?(result["artifacts"], "accepted.payload") == decision
          assert Map.has_key?(result["artifacts"], "rejected.payload") == not decision
          assert Map.has_key?(result["artifacts"], "gate.#{port}")

          if decision do
            assert result["artifacts"]["accepted.payload"] == @sample["payload"]
          end
        end
      end
    end
  end

  test "invalid conditions isolate without publishing decisions or invoking blocked consumers" do
    for sample <- Fixture.cases(),
        sample["error"],
        is_map(sample["config"]),
        reverse <- [false, true] do
      request = Fixture.request(sample, "skip", reverse)

      assert {:ok, result} =
               WorkflowGraphRunner.run(request["graph"], request["input_artifacts"],
                 execute_solve: fn _, _, _ -> flunk("no solver expected") end,
                 execute_transform: fn _, _, _ -> flunk("blocked consumer must not run") end,
                 execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
                 execute_export: &WorkflowOperatorRuntime.run_export_operator/3
               )

      assert result["failed_nodes"] == ["gate"]
      assert Enum.sort(result["skipped_nodes"]) == ["accepted", "consumer", "rejected"]
      assert result["branch_decisions"] == []
      assert Map.keys(result["artifacts"]) |> Enum.sort() == ["input.payload", "raw.payload"]
      assert hd(result["node_failures"])["error_message"] =~ sample["error"]
      refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "gate"))
    end

    assert {:ok, clean} = Engine.run_workflow_graph(Fixture.request(hd(Fixture.cases())))
    assert clean["failed_nodes"] == []
    assert Map.has_key?(clean["artifacts"], "accepted.payload")
  end

  test "array index parsing is bounded without rejecting long leading-zero paths" do
    zeros = String.duplicate("0", 100_000)

    for {path, decision} <- [
          {String.duplicate("9", 100_000), false},
          {zeros, true},
          {"+" <> zeros, true},
          {"-" <> zeros, false},
          {zeros <> "+0", false},
          {"++0", false},
          {"+", false},
          {"18446744073709551615", false}
        ] do
      assert {:ok, ^decision} =
               KyuubikiWeb.WorkflowCondition.evaluate([true], %{
                 "predicate" => %{"path" => path, "operator" => "truthy"}
               })
    end
  end
end
