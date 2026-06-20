defmodule KyuubikiWeb.WorkflowGraphResponse do
  @moduledoc false

  @default_options %{
    "include_artifact_lineage" => true,
    "include_artifacts" => true,
    "include_branch_decisions" => true,
    "include_dataset_lineage" => true,
    "include_node_runs" => true
  }
  @large_workflow_compact_threshold 1024

  def compact_options do
    %{
      "include_artifact_lineage" => false,
      "include_artifacts" => false,
      "include_branch_decisions" => false,
      "include_dataset_lineage" => false,
      "include_node_runs" => false
    }
  end

  def normalize_options(options) when is_map(options) do
    Enum.reduce(@default_options, @default_options, fn {key, default}, acc ->
      Map.put(acc, key, Map.get(options, key, default) == true)
    end)
  end

  def normalize_options(_options), do: @default_options

  def resolve_options(graph, options) when is_map(graph) and is_map(options),
    do: normalize_options(options)

  def resolve_options(graph, _options) when is_map(graph) do
    if large_workflow?(graph), do: compact_options(), else: @default_options
  end

  def shape(graph, result, options) when is_map(graph) and is_map(result) and is_map(options) do
    %{
      "workflow_id" => Map.get(graph, "id"),
      "dataset_contract" => Map.get(graph, "dataset_contract"),
      "completed_nodes" => Map.get(result, "completed_nodes", []),
      "skipped_nodes" => Map.get(result, "skipped_nodes", []),
      "performance" => Map.get(result, "performance", %{})
    }
    |> maybe_put("branch_decisions", options, result)
    |> maybe_put("node_runs", options, result)
    |> maybe_put("artifact_lineage", options, result)
    |> maybe_put("dataset_lineage", options, result)
    |> maybe_put("artifacts", options, result)
  end

  defp maybe_put(payload, "branch_decisions", options, result) do
    if Map.get(options, "include_branch_decisions") do
      Map.put(payload, "branch_decisions", Map.get(result, "branch_decisions", []))
    else
      payload
    end
  end

  defp maybe_put(payload, "node_runs", options, result) do
    if Map.get(options, "include_node_runs") do
      Map.put(payload, "node_runs", Map.get(result, "node_runs", []))
    else
      payload
    end
  end

  defp maybe_put(payload, "artifact_lineage", options, result) do
    if Map.get(options, "include_artifact_lineage") do
      Map.put(payload, "artifact_lineage", Map.get(result, "artifact_lineage", []))
    else
      payload
    end
  end

  defp maybe_put(payload, "dataset_lineage", options, result) do
    if Map.get(options, "include_dataset_lineage") do
      Map.put(payload, "dataset_lineage", Map.get(result, "dataset_lineage", []))
    else
      payload
    end
  end

  defp maybe_put(payload, "artifacts", options, result) do
    if Map.get(options, "include_artifacts") do
      Map.put(payload, "artifacts", Map.get(result, "artifacts", %{}))
    else
      payload
    end
  end

  defp large_workflow?(graph) do
    graph
    |> Map.get("nodes", [])
    |> length()
    |> Kernel.>=(@large_workflow_compact_threshold)
  end
end
