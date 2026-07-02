defmodule KyuubikiWeb.WorkflowGraphScheduler do
  @moduledoc false

  def initial_state do
    %{
      completed: MapSet.new(),
      skipped: MapSet.new(),
      ordered_completed: [],
      ordered_skipped: [],
      branch_decisions: [],
      node_runs: [],
      artifact_lineage: [],
      artifacts: %{},
      loop_passes: 0,
      started_at_us: System.monotonic_time(:microsecond)
    }
  end

  def indexes(nodes, edges) do
    %{
      nodes: nodes,
      edges: edges,
      node_count: length(nodes),
      nodes_by_id: Map.new(nodes, &{Map.get(&1, "id"), &1}),
      node_order: node_order(nodes),
      incoming_edges_by_node: group_edges(edges, ["to", "node"]),
      outgoing_edges_by_node: group_edges(edges, ["from", "node"])
    }
  end

  def initial_ready_node_ids(nodes) do
    nodes
    |> Enum.filter(&(Map.get(&1, "kind") == "input"))
    |> Enum.map(&Map.get(&1, "id"))
  end

  def enqueue_downstream(queue, indexes, node_id) do
    downstream =
      indexes.outgoing_edges_by_node
      |> Map.get(node_id, [])
      |> Enum.map(&get_in(&1, ["to", "node"]))
      |> Enum.uniq()
      |> Enum.sort_by(&Map.get(indexes.node_order, &1, indexes.node_count))

    queue ++ downstream
  end

  def resolved?(state, node_id) do
    MapSet.member?(state.completed, node_id) or MapSet.member?(state.skipped, node_id)
  end

  def pending_node_ids(indexes, state) do
    indexes.nodes
    |> Enum.map(&Map.get(&1, "id"))
    |> Enum.reject(&resolved?(state, &1))
  end

  def complete?(indexes, state) do
    MapSet.size(state.completed) + MapSet.size(state.skipped) == indexes.node_count
  end

  defp node_order(nodes) do
    nodes
    |> Enum.with_index()
    |> Map.new(fn {node, index} -> {Map.get(node, "id"), index} end)
  end

  defp group_edges(edges, path) do
    Enum.reduce(edges, %{}, fn edge, acc ->
      Map.update(acc, get_in(edge, path), [edge], &[edge | &1])
    end)
  end
end
