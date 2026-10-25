defmodule KyuubikiWeb.WorkflowSummaryAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture

  for sample <- Fixture.cases() do
    @sample sample
    test "summary admission rejects invalid final status: #{sample["operator_id"]}" do
      for marker <- Fixture.invalid_markers(), location <- Fixture.locations(@sample) do
        payload = Fixture.marked(@sample["payload"], location, marker)
        assert {:error, reason} = Fixture.direct(@sample, payload)
        assert reason =~ @sample["operator_id"]
        assert reason =~ if(location, do: "payload.#{location}.", else: "payload.")
      end
    end

    test "summary admission preserves metrics and ignores trial history: #{sample["operator_id"]}" do
      for marker <- Fixture.valid_markers(), location <- Fixture.locations(@sample) do
        # Collection-level arbitrary object keys are candidate entries, not generic metadata.
        marker =
          if @sample["shape"] == "collection" and is_nil(location),
            do: Map.take(marker, ["converged", "stability_result"]),
            else: marker

        payload = Fixture.marked(@sample["payload"], location, marker)
        assert {:ok, result} = Fixture.direct(@sample, payload)
        assert Map.take(result, Map.keys(@sample["expected"])) == @sample["expected"]
      end
    end
  end

  test "rejected summaries cannot publish assessments but raw results survive explicit recovery" do
    for sample <- Fixture.cases(),
        marker <- Fixture.invalid_markers(),
        source <- if(sample["shape"] == "single", do: ["left"], else: ["left", "right"]) do
      request = Fixture.request(sample, marker, source)
      assert {:error, {:workflow_node_error, "assess", _}} = Engine.run_workflow_graph(request)
      recoverable = Fixture.request(sample, marker, source, true)
      assert {:ok, result} = Engine.run_workflow_graph(recoverable)
      assert result["failed_nodes"] == ["assess"]
      assert result["skipped_nodes"] == ["result"]
      refute Map.has_key?(result["artifacts"], "assess.payload")
      refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "assess"))
      assert result["artifacts"]["raw.payload"] == request["input_artifacts"][source]
      assert {:ok, clean} = Engine.run_workflow_graph(Fixture.request(sample, %{}))

      assert Map.take(clean["artifacts"]["result.payload"], Map.keys(sample["expected"])) ==
               sample["expected"]
    end
  end

  test "status metadata alone cannot form a summary collection" do
    for sample <- Fixture.cases(), sample["shape"] == "collection" do
      assert {:error, _} =
               Fixture.direct(sample, %{
                 "converged" => true,
                 "stability_result" => %{"converged" => true, "cost" => 0}
               })
    end
  end

  test "raw failed solver results remain exportable for diagnosis" do
    payload = %{"converged" => false, "cost" => 1, "achieved_load_factor" => 0.25}
    assert {:ok, exported} = KyuubikiWeb.WorkflowReportingRuntime.export_summary_json(payload)
    assert Jason.decode!(exported["content"]) == payload
  end
end
