defmodule KyuubikiWeb.WorkflowSummaryReductionRangeTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.TestSupport.WorkflowSummaryAdmission, as: Fixture
  alias KyuubikiWeb.Orchestra.Engine

  for sample <- Fixture.reduction_cases() do
    @sample sample
    test "summary reduction range: #{sample["id"]}" do
      case @sample do
        %{"error" => field} ->
          assert {:error, reason} = Fixture.direct(@sample, @sample["payload"])
          assert reason =~ @sample["operator_id"]
          assert reason =~ field

        _ ->
          assert {:ok, result} = Fixture.direct(@sample, @sample["payload"])
          assert Map.take(result, Map.keys(@sample["expected"])) == @sample["expected"]
      end
    end
  end

  test "summary reductions isolate failures without partial results and recover on a fresh request" do
    cases = Fixture.reduction_cases()

    for sample <- cases do
      request = Fixture.request(sample, %{})

      if Map.has_key?(sample, "error") do
        assert {:error, {:workflow_node_error, "assess", _}} = Engine.run_workflow_graph(request)

        assert {:ok, failed} =
                 Engine.run_workflow_graph(Fixture.request(sample, %{}, "left", true))

        assert failed["failed_nodes"] == ["assess"]
        assert failed["skipped_nodes"] == ["result"]
        refute Map.has_key?(failed["artifacts"], "assess.payload")
        refute Map.has_key?(failed["artifacts"], "result.payload")
        refute Enum.any?(failed["artifact_lineage"], &(&1["node_id"] == "assess"))
        assert failed["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]

        clean =
          Enum.find(
            cases,
            &(&1["operator_id"] == sample["operator_id"] and not Map.has_key?(&1, "error"))
          )

        assert {:ok, fresh} = Engine.run_workflow_graph(Fixture.request(clean, %{}))

        assert Map.take(fresh["artifacts"]["result.payload"], Map.keys(clean["expected"])) ==
                 clean["expected"]
      else
        assert {:ok, direct} = Fixture.direct(sample, sample["payload"])
        assert {:ok, graph} = Engine.run_workflow_graph(request)
        assert graph["artifacts"]["result.payload"] == direct
      end
    end
  end

  test "normalization and selection reject malformed numeric options instead of using defaults" do
    cases = Fixture.reduction_cases()
    normalize = Enum.find(cases, &(&1["id"] == "normalize-regular-clamp"))
    select = Enum.find(cases, &(&1["id"] == "select-regular"))

    for bad <- [nil, true, "bad", [], %{}] do
      for key <- ["scale", "offset", "clamp_min", "clamp_max"] do
        sample = put_in(normalize, ["config", "rules"], [%{"source" => "metric", key => bad}])
        assert {:error, reason} = Fixture.direct(sample, sample["payload"])
        assert reason =~ "config.rules[0].#{key}"
      end

      sample = put_in(select, ["config", "criteria"], [%{"field" => "metric", "weight" => bad}])
      assert {:error, reason} = Fixture.direct(sample, sample["payload"])
      assert reason =~ "config.criteria[0].weight"
    end
  end

  test "aggregate cancellation tails survive bounded permutations and binary scales" do
    sample = hd(Fixture.reduction_cases())

    for exponent <- [-900, 0, 900],
        order <- [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] do
      scale = :math.pow(2.0, exponent)
      values = [:math.pow(2.0, 60) * scale, scale, -:math.pow(2.0, 60) * scale]

      payload =
        Enum.zip(["left", "middle", "right"], order)
        |> Map.new(fn {id, index} -> {id, %{"metric" => Enum.at(values, index)}} end)

      assert {:ok, result} = Fixture.direct(sample, payload)
      assert result["aggregate_metric_mean"] == scale / 3.0
    end

    payload = Map.new(["left", "middle", "right"], &{&1, %{"metric" => 1.7976931348623157e308}})
    assert {:ok, result} = Fixture.direct(sample, payload)
    assert result["aggregate_metric_mean"] == 1.7976931348623157e308
  end
end
