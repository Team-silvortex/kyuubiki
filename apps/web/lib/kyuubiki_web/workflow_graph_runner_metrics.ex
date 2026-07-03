defmodule KyuubikiWeb.WorkflowGraphRunnerMetrics do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.WorkflowCatalogSupport

  def finalize_result(state, graph, artifact_lineage, result_options \\ %{}) do
    artifact_lineage = Enum.reverse(artifact_lineage)
    node_runs = Enum.reverse(state.node_runs)
    total_elapsed_ms = elapsed_ms(state.started_at_us)
    node_execution_elapsed_ms = total_node_duration_ms(node_runs)

    %{
      "completed_nodes" => Enum.reverse(state.ordered_completed),
      "skipped_nodes" => Enum.reverse(state.ordered_skipped),
      "performance" => %{
        "total_elapsed_ms" => total_elapsed_ms,
        "node_execution_elapsed_ms" => node_execution_elapsed_ms,
        "scheduler_overhead_ms" => max(total_elapsed_ms - node_execution_elapsed_ms, 0.0),
        "loop_passes" => state.loop_passes,
        "artifact_count" => map_size(state.artifacts),
        "completed_node_count" => MapSet.size(state.completed),
        "skipped_node_count" => MapSet.size(state.skipped),
        "node_kind_breakdown" => node_kind_breakdown(node_runs),
        "slowest_nodes" => slowest_nodes(node_runs)
      }
    }
    |> maybe_put("branch_decisions", Enum.reverse(state.branch_decisions), result_options)
    |> maybe_put("node_runs", node_runs, result_options)
    |> maybe_put("artifact_lineage", artifact_lineage, result_options)
    |> maybe_put_lazy("dataset_lineage", fn -> dataset_lineage(graph, artifact_lineage) end, result_options)
    |> maybe_put("artifacts", state.artifacts, result_options)
  end

  defp maybe_put(payload, key, value, result_options) do
    if Map.get(result_options, "include_#{key}", true), do: Map.put(payload, key, value), else: payload
  end

  defp maybe_put_lazy(payload, key, value_fun, result_options) do
    if Map.get(result_options, "include_#{key}", true),
      do: Map.put(payload, key, value_fun.()),
      else: payload
  end

  defp dataset_lineage(graph, artifact_lineage),
    do: WorkflowCatalogSupport.derive_dataset_lineage(graph, artifact_lineage)

  def mark_completed(
        state,
        node_id,
        node,
        incoming,
        duration_ms,
        result_options,
        accepts_partial_inputs?,
        consumed_artifacts_for_node,
        incoming_artifact_keys,
        artifact_key
      ) do
    run = completed_run(state, node_id, node, incoming, duration_ms, result_options, %{
      accepts_partial_inputs?: accepts_partial_inputs?,
      consumed_artifacts_for_node: consumed_artifacts_for_node,
      incoming_artifact_keys: incoming_artifact_keys,
      artifact_key: artifact_key
    })

    %{
      state
      | completed: MapSet.put(state.completed, node_id),
        ordered_completed: [node_id | state.ordered_completed],
        node_runs: [run | state.node_runs]
    }
  end

  def mark_skipped(state, node_id, node, incoming, result_options, incoming_artifact_keys) do
    run =
      if detailed_node_runs?(result_options) do
        %{
          "node_id" => node_id,
          "kind" => Map.get(node, "kind"),
          "operator_id" => Map.get(node, "operator_id"),
          "status" => "skipped",
          "consumed_artifacts" => incoming_artifact_keys.(incoming, state.artifacts),
          "produced_artifacts" => [],
          "duration_ms" => 0.0
        }
        |> maybe_put_task_ir_ref(node)
      else
        minimal_run(node_id, node, "skipped", 0.0)
      end

    %{
      state
      | skipped: MapSet.put(state.skipped, node_id),
        ordered_skipped: [node_id | state.ordered_skipped],
        node_runs: [run | state.node_runs]
    }
  end

  defp completed_run(state, node_id, node, incoming, duration_ms, result_options, resolvers) do
    if detailed_node_runs?(result_options) do
      %{
        "node_id" => node_id,
        "kind" => Map.get(node, "kind"),
        "operator_id" => Map.get(node, "operator_id"),
        "status" => "completed",
        "consumed_artifacts" =>
          consumed_artifacts_for_run(
            node,
            incoming,
            state.artifacts,
            resolvers.accepts_partial_inputs?,
            resolvers.consumed_artifacts_for_node,
            resolvers.incoming_artifact_keys
          ),
        "produced_artifacts" => produced_artifacts_for_run(node, incoming, resolvers.artifact_key),
        "duration_ms" => duration_ms
      }
      |> maybe_put_task_ir_ref(node)
    else
      minimal_run(node_id, node, "completed", duration_ms)
    end
  end

  defp minimal_run(node_id, node, status, duration_ms) do
    %{
      "node_id" => node_id,
      "kind" => Map.get(node, "kind"),
      "operator_id" => Map.get(node, "operator_id"),
      "status" => status,
      "duration_ms" => duration_ms
    }
  end

  defp detailed_node_runs?(result_options),
    do: Map.get(result_options, "include_node_runs", true)

  defp consumed_artifacts_for_run(
         node,
         incoming,
         artifacts,
         accepts_partial_inputs?,
         consumed_artifacts_for_node,
         incoming_artifact_keys
       ) do
    if accepts_partial_inputs?.(node) do
      consumed_artifacts_for_node.(node, incoming, artifacts)
    else
      incoming_artifact_keys.(incoming, artifacts)
    end
  end

  defp produced_artifacts_for_run(%{"kind" => "output", "id" => node_id}, incoming, artifact_key) do
    incoming
    |> Enum.map(&artifact_key.(node_id, get_in(&1, ["to", "port"])))
    |> Enum.sort()
  end

  defp produced_artifacts_for_run(%{"id" => node_id} = node, _incoming, artifact_key) do
    node
    |> Map.get("outputs", [])
    |> Enum.map(&artifact_key.(node_id, Map.get(&1, "id")))
    |> Enum.sort()
  end

  defp maybe_put_task_ir_ref(run, %{"operator_id" => operator_id} = node)
       when is_binary(operator_id) do
    case OperatorTaskIR.ref_from_node(node) do
      {:ok, task_ref} -> Map.put(run, "task_ir_ref", task_ref)
      _ -> run
    end
  end

  defp maybe_put_task_ir_ref(run, _node), do: run

  defp elapsed_ms(started_at_us) do
    (System.monotonic_time(:microsecond) - started_at_us) / 1000.0
  end

  defp total_node_duration_ms(node_runs) do
    Enum.reduce(node_runs, 0.0, fn run, total ->
      total + numeric_duration(Map.get(run, "duration_ms"))
    end)
  end

  defp node_kind_breakdown(node_runs) do
    node_runs
    |> Enum.group_by(&Map.get(&1, "kind", "unknown"))
    |> Enum.sort_by(fn {kind, _runs} -> kind end)
    |> Enum.into(%{}, fn {kind, runs} ->
      completed_count = Enum.count(runs, &(Map.get(&1, "status") == "completed"))
      skipped_count = Enum.count(runs, &(Map.get(&1, "status") == "skipped"))

      {kind,
       %{
         "count" => length(runs),
         "completed_count" => completed_count,
         "skipped_count" => skipped_count,
         "elapsed_ms" => total_node_duration_ms(runs)
       }}
    end)
  end

  defp slowest_nodes(node_runs) do
    node_runs
    |> Enum.filter(&(Map.get(&1, "status") == "completed"))
    |> Enum.sort_by(&numeric_duration(Map.get(&1, "duration_ms")), :desc)
    |> Enum.take(5)
    |> Enum.map(fn run ->
      %{
        "node_id" => Map.get(run, "node_id"),
        "kind" => Map.get(run, "kind"),
        "operator_id" => Map.get(run, "operator_id"),
        "duration_ms" => numeric_duration(Map.get(run, "duration_ms"))
      }
    end)
  end

  defp numeric_duration(value) when is_integer(value), do: value * 1.0
  defp numeric_duration(value) when is_float(value), do: value
  defp numeric_duration(_value), do: 0.0
end
