defmodule KyuubikiWeb.WorkflowGraphRunner do
  @moduledoc false

  alias KyuubikiWeb.WorkflowGraphRunnerMetrics
  def run(graph, input_artifacts, opts \\ [])

  def run(graph, input_artifacts, opts)
      when is_map(graph) and is_map(input_artifacts) and is_list(opts) do
    with nodes when is_list(nodes) <- Map.get(graph, "nodes"),
         edges when is_list(edges) <- Map.get(graph, "edges", []),
         execute_solve when is_function(execute_solve, 3) <- Keyword.get(opts, :execute_solve),
         execute_transform when is_function(execute_transform, 3) <-
           Keyword.get(opts, :execute_transform),
         execute_extract when is_function(execute_extract, 3) <-
           Keyword.get(opts, :execute_extract),
         execute_export when is_function(execute_export, 3) <- Keyword.get(opts, :execute_export) do
      state = %{
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

      do_run_workflow_graph(
        nodes,
        edges,
        incoming_edges_by_node(edges),
        length(nodes),
        input_artifacts,
        state,
        opts
      )
    else
      _ -> {:error, :invalid_workflow_graph}
    end
  end

  def run(_graph, _input_artifacts, _opts), do: {:error, :invalid_workflow_graph}

  defp do_run_workflow_graph(
         nodes,
         edges,
         incoming_edges_by_node,
         node_count,
         input_artifacts,
         state,
         opts
       ) do
    state = %{state | loop_passes: state.loop_passes + 1}

    {next_state, progressed?} =
      Enum.reduce(nodes, {state, false}, fn node, {acc, moved?} ->
        node_id = Map.get(node, "id")

        cond do
          MapSet.member?(acc.completed, node_id) or MapSet.member?(acc.skipped, node_id) ->
            {acc, moved?}

          true ->
            incoming = Map.get(incoming_edges_by_node, node_id, [])
            kind = Map.get(node, "kind")
            ready? = kind == "input" or workflow_node_ready?(node, incoming, acc.artifacts)

            cond do
              ready? ->
                started_at_us = System.monotonic_time(:microsecond)

                case execute_workflow_node(node, incoming, input_artifacts, acc, opts) do
                  {:ok, next_acc} ->
                    updated =
                      mark_completed(
                        next_acc,
                        node_id,
                        node,
                        incoming,
                        elapsed_ms(started_at_us)
                      )

                    maybe_emit_progress(updated, node_count, opts)
                    {updated, true}

                  {:error, reason} ->
                    throw({:workflow_node_error, node_id, reason})
                end

              unresolved_missing_inputs?(incoming, acc.artifacts, acc.completed, acc.skipped) ->
                {mark_skipped(acc, node_id, node, incoming), true}

              true ->
                {acc, moved?}
            end
        end
      end)

    cond do
      MapSet.size(next_state.completed) + MapSet.size(next_state.skipped) == node_count ->
        graph = %{
          "nodes" => nodes,
          "edges" => edges,
          "dataset_contract" => Keyword.get(opts, :dataset_contract)
        }

        artifact_lineage = next_state.artifact_lineage

        {:ok, WorkflowGraphRunnerMetrics.finalize_result(next_state, graph, artifact_lineage)}

      progressed? ->
        do_run_workflow_graph(
          nodes,
          edges,
          incoming_edges_by_node,
          node_count,
          input_artifacts,
          next_state,
          opts
        )

      true ->
        pending =
          nodes
          |> Enum.map(&Map.get(&1, "id"))
          |> Enum.reject(
            &(MapSet.member?(next_state.completed, &1) or MapSet.member?(next_state.skipped, &1))
          )

        {:error, {:workflow_stalled, pending}}
    end
  catch
    {:workflow_node_error, node_id, reason} ->
      {:error, {:workflow_node_error, node_id, reason}}

    {:workflow_cancelled, node_id} ->
      {:error, {:workflow_cancelled, node_id}}
  end

  defp workflow_node_ready?(node, incoming, artifacts) do
    if transform_operator_accepts_partial_inputs?(node) do
      Enum.any?(incoming, &Map.has_key?(artifacts, edge_from_key(&1)))
    else
      Enum.all?(incoming, &Map.has_key?(artifacts, edge_from_key(&1)))
    end
  end

  defp transform_operator_accepts_partial_inputs?(%{
         "kind" => "transform",
         "operator_id" => "transform.first_available"
       }),
       do: true

  defp transform_operator_accepts_partial_inputs?(_node), do: false
  defp transform_operator_requires_port_map?(%{
         "kind" => "transform",
         "operator_id" => operator_id
       })
       when operator_id in [
              "transform.merge_summary_pair",
              "transform.compare_summary_pair",
              "transform.aggregate_summary_collection",
              "transform.select_best_summary",
              "transform.compose_quality_objective",
              "transform.compose_diagnostics_bundle",
              "transform.compose_diagnostics_report_payload",
              "transform.resolve_focus_bridge_execution",
              "transform.benchmark_coupled_heat_pair",
              "transform.validate_electrostatic_heat_bridge",
              "transform.validate_heat_thermo_bridge"
            ],
       do: true

  defp transform_operator_requires_port_map?(_node), do: false
  defp unresolved_missing_inputs?(incoming, artifacts, completed, skipped) do
    Enum.any?(incoming, fn edge ->
      key = edge_from_key(edge)
      not Map.has_key?(artifacts, key)
    end) and
      Enum.all?(incoming, fn edge ->
        key = edge_from_key(edge)
        from_node = get_in(edge, ["from", "node"])

        Map.has_key?(artifacts, key) or MapSet.member?(completed, from_node) or
          MapSet.member?(skipped, from_node)
      end)
  end

  defp execute_workflow_node(
         %{"kind" => "input", "id" => node_id} = node,
         _incoming,
         input_artifacts,
         state,
         _opts
       ) do
    case Map.fetch(input_artifacts, node_id) do
      {:ok, value} ->
        {:ok, publish_node_outputs(state, node, value, [])}

      :error ->
        {:error, :missing_input_artifact}
    end
  end

  defp execute_workflow_node(%{"kind" => "solve"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts),
         {:ok, result} <- Keyword.fetch!(opts, :execute_solve).(operator_id, payload, node) do
      {:ok,
       publish_node_outputs(
         state,
         node,
         result,
         incoming_artifact_keys(incoming, state.artifacts)
       )}
    end
  end

  defp execute_workflow_node(%{"kind" => "transform"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_transform_payload(node, incoming, state.artifacts),
         {:ok, result} <-
           Keyword.fetch!(opts, :execute_transform).(
             operator_id,
             payload,
             Map.get(node, "config")
           ) do
      consumed = consumed_artifacts_for_node(node, incoming, state.artifacts)
      {:ok, publish_node_outputs(state, node, result, consumed)}
    end
  end

  defp execute_workflow_node(%{"kind" => "extract"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts),
         {:ok, result} <-
           Keyword.fetch!(opts, :execute_extract).(operator_id, payload, Map.get(node, "config")) do
      {:ok,
       publish_node_outputs(
         state,
         node,
         result,
         incoming_artifact_keys(incoming, state.artifacts)
       )}
    end
  end

  defp execute_workflow_node(%{"kind" => "export"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts),
         {:ok, result} <-
           Keyword.fetch!(opts, :execute_export).(operator_id, payload, Map.get(node, "config")) do
      {:ok,
       publish_node_outputs(
         state,
         node,
         result,
         incoming_artifact_keys(incoming, state.artifacts)
       )}
    end
  end

  defp execute_workflow_node(%{"kind" => "condition"} = node, incoming, _inputs, state, _opts) do
    with {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts),
         {:ok, predicate_result} <- evaluate_condition_operator(payload, Map.get(node, "config")),
         {:ok, selected_output} <- select_condition_output(node, predicate_result) do
      artifact_key = artifact_key(Map.get(node, "id"), Map.get(selected_output, "id"))
      source_artifacts = incoming_artifact_keys(incoming, state.artifacts)

      {:ok,
       state
       |> put_artifact(artifact_key, payload)
       |> append_artifact_lineage(
         artifact_key,
         Map.get(node, "id"),
         Map.get(selected_output, "id"),
         source_artifacts
       )
       |> append_branch_decision(node, selected_output, predicate_result)}
    end
  end

  defp execute_workflow_node(%{"kind" => "output"} = node, incoming, _inputs, state, _opts) do
    updated =
      Enum.reduce(incoming, state, fn edge, acc ->
        source_key = edge_from_key(edge)
        target_key = artifact_key(Map.get(node, "id"), get_in(edge, ["to", "port"]))
        value = Map.fetch!(acc.artifacts, source_key)

        acc
        |> put_artifact(target_key, value)
        |> append_artifact_lineage(
          target_key,
          Map.get(node, "id"),
          get_in(edge, ["to", "port"]),
          [source_key]
        )
      end)

    {:ok, updated}
  end

  defp execute_workflow_node(%{"kind" => kind}, _incoming, _inputs, _state, _opts),
    do: {:error, {:unsupported_workflow_node_kind, kind}}

  defp fetch_operator_id(%{"operator_id" => operator_id})
       when is_binary(operator_id) and operator_id != "",
       do: {:ok, operator_id}

  defp fetch_operator_id(_node), do: {:error, :missing_operator_id}

  defp resolve_single_input_payload(%{"id" => node_id}, incoming, artifacts) do
    case incoming do
      [first | _] ->
        key = edge_from_key(first)

        case Map.fetch(artifacts, key) do
          {:ok, value} -> {:ok, value}
          :error -> {:error, {:missing_upstream_artifact, key}}
        end

      [] ->
        {:error, {:missing_workflow_input, node_id}}
    end
  end

  defp resolve_transform_payload(node, incoming, artifacts) do
    if transform_operator_accepts_partial_inputs?(node) do
      case Enum.find(incoming, &Map.has_key?(artifacts, edge_from_key(&1))) do
        nil -> {:error, {:missing_workflow_input, Map.get(node, "id")}}
        edge -> {:ok, Map.fetch!(artifacts, edge_from_key(edge))}
      end
    else
      if transform_operator_requires_port_map?(node) do
        resolve_named_transform_payload(node, incoming, artifacts)
      else
        resolve_single_input_payload(node, incoming, artifacts)
      end
    end
  end

  defp resolve_named_transform_payload(%{"id" => node_id}, incoming, artifacts) do
    payload =
      Enum.reduce_while(incoming, %{}, fn edge, acc ->
        key = edge_from_key(edge)

        case Map.fetch(artifacts, key) do
          {:ok, value} ->
            {:cont,
             Map.put(
               acc,
               get_in(edge, ["to", "port"]),
               unwrap_named_transform_input(value, edge)
             )}

          :error ->
            {:halt, {:error, {:missing_upstream_artifact, key}}}
        end
      end)

    case payload do
      {:error, _reason} = error ->
        error

      payload when map_size(payload) == 0 ->
        {:error, {:missing_workflow_input, node_id}}

      payload ->
        {:ok, payload}
    end
  end

  defp unwrap_named_transform_input(value, edge) when is_map(value) do
    case get_in(edge, ["from", "port"]) do
      port_id when is_binary(port_id) -> Map.get(value, port_id, value)
      _ -> value
    end
  end

  defp unwrap_named_transform_input(value, _edge), do: value

  defp consumed_artifacts_for_node(node, incoming, artifacts) do
    if transform_operator_accepts_partial_inputs?(node) do
      incoming
      |> Enum.find(&Map.has_key?(artifacts, edge_from_key(&1)))
      |> case do
        nil -> []
        edge -> [edge_from_key(edge)]
      end
    else
      incoming_artifact_keys(incoming, artifacts)
    end
  end

  defp incoming_artifact_keys(incoming, artifacts) do
    incoming
    |> Enum.map(&edge_from_key/1)
    |> Enum.filter(&Map.has_key?(artifacts, &1))
  end

  defp select_condition_output(node, predicate_result) do
    outputs = Map.get(node, "outputs", [])

    selected =
      Enum.find(outputs, fn output ->
        output_id = Map.get(output, "id")

        (predicate_result and output_id in ["if_true", "true"]) or
          (!predicate_result and output_id in ["if_false", "false"])
      end) ||
        if predicate_result do
          List.first(outputs)
        else
          Enum.at(outputs, 1) || List.first(outputs)
        end

    if selected do
      {:ok, selected}
    else
      {:error, :missing_condition_outputs}
    end
  end

  defp evaluate_condition_operator(payload, config) when is_map(config) do
    predicate = Map.get(config, "predicate", %{})
    operator = Map.get(predicate, "operator", "gt")
    target = predicate |> Map.get("path") |> resolve_condition_target(payload)

    case operator do
      "truthy" ->
        {:ok, truthy?(target)}

      "falsy" ->
        {:ok, not truthy?(target)}

      "eq" ->
        {:ok, target == Map.get(predicate, "value")}

      "neq" ->
        {:ok, target != Map.get(predicate, "value")}

      numeric when numeric in ["gt", "gte", "lt", "lte"] ->
        compare_numeric_condition(numeric, target, Map.get(predicate, "value"))

      "contains" ->
        contains_condition(target, Map.get(predicate, "value"))

      _ ->
        {:error, {:unsupported_condition_operator, operator}}
    end
  end

  defp evaluate_condition_operator(payload, _config), do: {:ok, truthy?(payload)}

  defp resolve_condition_target(nil, payload), do: payload

  defp resolve_condition_target(path, payload) when is_binary(path) do
    path
    |> String.split(".", trim: true)
    |> Enum.reduce(payload, fn segment, current ->
      cond do
        is_map(current) -> Map.get(current, segment)
        is_list(current) -> current |> Enum.at(parse_array_index(segment))
        true -> nil
      end
    end)
  end

  defp resolve_condition_target(_path, payload), do: payload

  defp parse_array_index(segment) do
    case Integer.parse(segment) do
      {index, ""} -> index
      _ -> -1
    end
  end

  defp compare_numeric_condition(operator, left, right)
       when is_number(left) and is_number(right) do
    result =
      case operator do
        "gt" -> left > right
        "gte" -> left >= right
        "lt" -> left < right
        "lte" -> left <= right
      end

    {:ok, result}
  end

  defp compare_numeric_condition(operator, _left, _right),
    do: {:error, {:invalid_condition_numeric_operand, operator}}

  defp contains_condition(target, value) when is_binary(target) and is_binary(value),
    do: {:ok, String.contains?(target, value)}

  defp contains_condition(target, value) when is_list(target),
    do: {:ok, Enum.member?(target, value)}

  defp contains_condition(_target, _value), do: {:error, :invalid_condition_contains_operand}

  defp truthy?(nil), do: false
  defp truthy?(false), do: false
  defp truthy?(""), do: false
  defp truthy?(value) when is_number(value), do: value != 0
  defp truthy?(value) when is_list(value), do: value != []
  defp truthy?(value) when is_map(value), do: map_size(value) > 0
  defp truthy?(_value), do: true

  defp publish_node_outputs(state, node, value, source_artifacts) do
    Enum.reduce(Map.get(node, "outputs", []), state, fn output, acc ->
      port_id = Map.get(output, "id")
      key = artifact_key(Map.get(node, "id"), port_id)

      acc
      |> put_artifact(key, value)
      |> append_artifact_lineage(key, Map.get(node, "id"), port_id, source_artifacts)
    end)
  end

  defp put_artifact(state, key, value),
    do: %{state | artifacts: Map.put(state.artifacts, key, value)}

  defp append_artifact_lineage(state, artifact_key, node_id, port_id, source_artifacts) do
    entry = %{
      "artifact_key" => artifact_key,
      "node_id" => node_id,
      "port_id" => port_id,
      "source_artifacts" => source_artifacts
    }

    %{state | artifact_lineage: [entry | state.artifact_lineage]}
  end

  defp append_branch_decision(state, node, selected_output, predicate_result) do
    entry = %{
      "node_id" => Map.get(node, "id"),
      "chosen_output" => Map.get(selected_output, "id"),
      "predicate_result" => predicate_result
    }

    %{state | branch_decisions: [entry | state.branch_decisions]}
  end

  defp mark_completed(state, node_id, node, incoming, duration_ms),
    do:
      WorkflowGraphRunnerMetrics.mark_completed(
        state,
        node_id,
        node,
        incoming,
        duration_ms,
        &transform_operator_accepts_partial_inputs?/1,
        &consumed_artifacts_for_node/3,
        &incoming_artifact_keys/2,
        &artifact_key/2
      )

  defp mark_skipped(state, node_id, node, incoming),
    do:
      WorkflowGraphRunnerMetrics.mark_skipped(
        state,
        node_id,
        node,
        incoming,
        &incoming_artifact_keys/2
      )

  defp maybe_emit_progress(state, node_count, opts) do
    case Keyword.get(opts, :progress_callback) do
      callback when is_function(callback, 1) ->
        callback.(%{
          "node_id" => hd(state.ordered_completed),
          "completed_nodes" => length(state.ordered_completed),
          "total_nodes" => node_count
        })

      _ ->
        :ok
    end
  end

  defp edge_from_key(edge),
    do: artifact_key(get_in(edge, ["from", "node"]), get_in(edge, ["from", "port"]))

  defp artifact_key(node_id, port_id), do: "#{node_id}.#{port_id}"

  defp incoming_edges_by_node(edges) do
    Enum.reduce(edges, %{}, fn edge, acc ->
      Map.update(acc, get_in(edge, ["to", "node"]), [edge], &[edge | &1])
    end)
  end

  defp elapsed_ms(started_at_us) do
    (System.monotonic_time(:microsecond) - started_at_us) / 1000.0
  end
end
