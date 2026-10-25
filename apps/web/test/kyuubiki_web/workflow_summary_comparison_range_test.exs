defmodule KyuubikiWeb.WorkflowSummaryComparisonRangeTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture

  for sample <- Fixture.comparison_cases() do
    @sample sample

    test "summary comparison range: #{sample["id"]}" do
      case @sample do
        %{"error" => field} ->
          assert {:error, reason} = Fixture.direct(@sample, @sample["payload"])
          assert reason =~ "transform.compare_summary_pair"
          assert reason =~ field

        _ ->
          assert {:ok, actual} = Fixture.direct(@sample, @sample["payload"])
          assert Map.take(actual, Map.keys(@sample["expected"])) == @sample["expected"]
          for key <- @sample["absent"], do: refute(Map.has_key?(actual, key))
          assert {:ok, result} = Engine.run_workflow_graph(Fixture.request(@sample, %{}))
          assert result["artifacts"]["result.payload"] == actual
      end
    end
  end

  test "comparison overflow isolates the failed node without partial metrics or lineage" do
    for sample <- Fixture.comparison_cases(), Map.has_key?(sample, "error") do
      assert {:error, {:workflow_node_error, "assess", _}} =
               Engine.run_workflow_graph(Fixture.request(sample, %{}))

      assert {:ok, failed} = Engine.run_workflow_graph(Fixture.request(sample, %{}, "left", true))
      assert failed["failed_nodes"] == ["assess"]
      assert failed["skipped_nodes"] == ["result"]
      refute Map.has_key?(failed["artifacts"], "assess.payload")
      refute Map.has_key?(failed["artifacts"], "result.payload")
      refute Enum.any?(failed["artifact_lineage"], &(&1["node_id"] == "assess"))
      assert failed["artifacts"]["raw.payload"] == sample["payload"]["left"]
    end
  end

  test "zero baseline option combinations never evaluate undefined relative metrics" do
    sample = hd(Fixture.comparison_cases())

    for originals <- [false, true],
        delta <- [false, true],
        ratio <- [false, true],
        percent <- [false, true],
        count <- [false, true] do
      sample =
        Map.put(sample, "config", %{
          "include_originals" => originals,
          "include_delta" => delta,
          "include_ratio" => ratio,
          "include_percent_change" => percent,
          "include_shared_field_count" => count
        })

      if originals or delta do
        assert {:ok, result} = Fixture.direct(sample, sample["payload"])
        refute Map.has_key?(result, "ratio_metric")
        refute Map.has_key?(result, "percent_change_metric")
        assert Map.has_key?(result, "delta_metric") == delta
        assert Map.has_key?(result, "left_metric") == originals
        assert Map.has_key?(result, "summary_shared_numeric_field_count") == count
      else
        assert {:error, _} = Fixture.direct(sample, sample["payload"])
      end
    end
  end

  test "relative comparison is invariant under bounded binary rescaling" do
    sample = hd(Fixture.comparison_cases())

    for exponent <- -1000..1000//100, sign <- [-1.0, 1.0] do
      scale = sign * :math.pow(2.0, exponent)
      payload = %{"left" => %{"metric" => 1.25 * scale}, "right" => %{"metric" => 1.5 * scale}}
      assert {:ok, result} = Fixture.direct(sample, payload)
      assert result["delta_metric"] == 0.25 * scale
      assert result["ratio_metric"] == 1.2
      assert result["percent_change_metric"] == 20.0
    end
  end
end
