defmodule KyuubikiWeb.Api.WorkflowLargeGraphApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  require Logger
  alias KyuubikiWeb.TestSupport.WorkflowLargeGraphBenchmark

  test "runs a large workflow graph at 96 intermediate nodes through the orchestration API" do
    run_large_graph_case(96)
  end

  test "runs a large workflow graph at 192 intermediate nodes through the orchestration API" do
    run_large_graph_case(192)
  end

  test "runs a large workflow graph at 256 intermediate nodes through the orchestration API" do
    run_large_graph_case(256)
  end

  test "runs a large workflow graph at 384 intermediate nodes through the orchestration API" do
    run_large_graph_case(384)
  end

  test "runs a large workflow graph at 512 intermediate nodes through the orchestration API" do
    run_large_graph_case(512)
  end

  test "supports compact workflow graph responses for large runs" do
    result =
      WorkflowLargeGraphBenchmark.run_case(
        @opts,
        512,
        KyuubikiWeb.WorkflowGraphResponse.compact_options()
      )

    payload = result["payload"]

    assert length(payload["completed_nodes"]) == 517
    assert get_in(payload, ["performance", "completed_node_count"]) == 517
    refute Map.has_key?(payload, "artifacts")
    refute Map.has_key?(payload, "node_runs")
    refute Map.has_key?(payload, "artifact_lineage")
    refute Map.has_key?(payload, "dataset_lineage")
    refute Map.has_key?(payload, "branch_decisions")
  end

  defp run_large_graph_case(pass_through_count) do
    result = WorkflowLargeGraphBenchmark.run_case(@opts, pass_through_count)
    payload = result["payload"]

    Logger.info(
      "workflow_large_graphs[web]: pass_through_count=#{pass_through_count} completed_nodes=#{length(payload["completed_nodes"])} elapsed_ms=#{result["elapsed_ms"]}"
    )

    assert length(payload["completed_nodes"]) == pass_through_count + 5
    assert payload["skipped_nodes"] == []
    assert hd(payload["completed_nodes"]) == "heat_model"
    assert List.last(payload["completed_nodes"]) == "thermo_output"

    tail_key =
      "pass_#{String.pad_leading(Integer.to_string(pass_through_count - 1), 3, "0")}.result"

    assert get_in(payload, ["artifacts", tail_key, "max_temperature"]) == 100.0
    assert get_in(payload, ["artifacts", "thermo_output.result", "max_stress"]) > 0.0
    assert length(payload["node_runs"]) == pass_through_count + 5
    assert get_in(payload, ["performance", "completed_node_count"]) == pass_through_count + 5
    assert get_in(payload, ["performance", "artifact_count"]) >= pass_through_count + 5
    assert get_in(payload, ["performance", "total_elapsed_ms"]) >= 0.0
    assert get_in(payload, ["performance", "node_execution_elapsed_ms"]) >= 0.0
    assert get_in(payload, ["performance", "scheduler_overhead_ms"]) >= 0.0
    assert get_in(payload, ["performance", "node_kind_breakdown", "solve", "count"]) == 2

    assert get_in(payload, ["performance", "node_kind_breakdown", "transform", "count"]) ==
             pass_through_count + 1

    assert get_in(payload, ["performance", "node_kind_breakdown", "output", "count"]) == 1
    assert length(get_in(payload, ["performance", "slowest_nodes"])) > 0

    assert Enum.any?(payload["node_runs"], fn entry ->
             entry["node_id"] == "solve_heat" and is_number(entry["duration_ms"])
           end)

    assert Enum.all?(get_in(payload, ["performance", "slowest_nodes"]), fn entry ->
             is_binary(entry["node_id"]) and is_number(entry["duration_ms"])
           end)
  end
end
