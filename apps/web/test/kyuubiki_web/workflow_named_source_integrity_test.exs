defmodule KyuubikiWeb.WorkflowNamedSourceIntegrityTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowNamedInputContract, as: Fixture
  alias KyuubikiWeb.WorkflowOperatorRuntime

  for sample <- Fixture.source_integrity_cases() do
    @sample sample
    test "named sources retain sibling fields through #{sample["operator_id"]}" do
      payload = Map.new(Enum.zip(@sample["ports"], @sample["values"]))

      assert {:ok, direct} =
               WorkflowOperatorRuntime.run_transform_operator(
                 @sample["operator_id"],
                 payload,
                 @sample["config"]
               )

      assert Map.take(direct, Map.keys(@sample["expected"])) == @sample["expected"]

      for reverse <- [false, true] do
        request = @sample |> Fixture.request() |> Fixture.reorder(reverse)
        assert {:ok, result} = Engine.run_workflow_graph(request)
        assert result["artifacts"]["result.summary"] == direct
        assert result["artifacts"]["raw.payload"] == hd(@sample["values"])
        assert result["failed_nodes"] == []
      end
    end
  end

  test "diagnostic rejection markers cannot be stripped by source-port unpacking" do
    sample = List.last(Fixture.source_integrity_cases())

    for marker <- Fixture.invalid_source_markers(), reverse <- [false, true] do
      request =
        sample
        |> Fixture.request()
        |> Fixture.reorder(reverse)
        |> update_in(["input_artifacts", "left"], &Map.merge(&1, marker))

      assert {:error, {:workflow_node_error, "combine", _}} = Engine.run_workflow_graph(request)

      recoverable =
        update_in(request, ["graph", "nodes"], fn nodes ->
          Enum.map(nodes, fn node ->
            if node["id"] == "combine",
              do: put_in(node, ["config", "on_error"], "skip"),
              else: node
          end)
        end)

      assert {:ok, result} = Engine.run_workflow_graph(recoverable)
      assert result["failed_nodes"] == ["combine"]
      assert result["skipped_nodes"] == ["result"]
      refute Map.has_key?(result["artifacts"], "result.summary")
      refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "combine"))
      assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]
      assert {:ok, clean} = Engine.run_workflow_graph(Fixture.request(sample))
      assert clean["artifacts"]["result.summary"]["bundle_total_node_count"] == 5
    end
  end

  test "single-field legacy source wrappers still work for every shared named operator" do
    for sample <- Fixture.source_integrity_cases(), reverse <- [false, true] do
      request = sample |> Fixture.request() |> Fixture.reorder(reverse)

      request =
        update_in(request, ["input_artifacts"], fn inputs ->
          Map.new(inputs, fn {id, value} -> {id, %{"payload" => value}} end)
        end)

      assert {:ok, result} = Engine.run_workflow_graph(request)
      summary = result["artifacts"]["result.summary"]
      assert Map.take(summary, Map.keys(sample["expected"])) == sample["expected"]
      assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]
    end
  end
end
