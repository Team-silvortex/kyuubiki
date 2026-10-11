defmodule KyuubikiWeb.WorkflowGraphRunner do
  @moduledoc false
  alias KyuubikiWeb.WorkflowCondition
  alias KyuubikiWeb.WorkflowGraphInputs
  alias KyuubikiWeb.WorkflowGraphRecovery
  alias KyuubikiWeb.WorkflowGraphPreflight
  alias KyuubikiWeb.WorkflowGraphProvenance
  alias KyuubikiWeb.WorkflowGraphRunnerMetrics
  alias KyuubikiWeb.WorkflowGraphScheduler
  alias KyuubikiWeb.WorkflowJsonBudget

  def run(graph, input_artifacts, opts \\ [])

  def run(graph, input_artifacts, opts)
      when is_map(graph) and is_map(input_artifacts) and is_list(opts) do
    with :ok <- WorkflowGraphPreflight.validate(graph, input_artifacts),
         nodes when is_list(nodes) <- Map.get(graph, "nodes"),
         edges when is_list(edges) <- Map.get(graph, "edges", []),
         execute_solve when is_function(execute_solve, 3) or is_function(execute_solve, 4) <-
           Keyword.get(opts, :execute_solve),
         execute_transform
         when is_function(execute_transform, 3) or is_function(execute_transform, 4) or
                is_function(execute_transform, 5) <-
           Keyword.get(opts, :execute_transform),
         execute_extract
         when is_function(execute_extract, 3) or is_function(execute_extract, 4) or
                is_function(execute_extract, 5) <-
           Keyword.get(opts, :execute_extract),
         execute_export when is_function(execute_export, 3) <- Keyword.get(opts, :execute_export) do
      run_ordered_workflow_graph(
        WorkflowGraphScheduler.indexes(nodes, edges),
        input_artifacts,
        WorkflowGraphScheduler.initial_state(),
        opts
      )
    else
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph}
    end
  end

  def run(_graph, _input_artifacts, _opts), do: {:error, :invalid_workflow_graph}

  defp run_ordered_workflow_graph(indexes, input_artifacts, state, opts) do
    state = %{state | loop_passes: state.loop_passes + 1}

    {next_state, progressed?} =
      Enum.reduce(indexes.nodes, {state, false}, fn node, {acc, progressed?} ->
        case process_workflow_node(Map.get(node, "id"), indexes, input_artifacts, acc, opts) do
          {:ok, updated, true} -> {updated, true}
          {:ok, updated, false} -> {updated, progressed?}
        end
      end)

    cond do
      WorkflowGraphScheduler.complete?(indexes, next_state) ->
        finalize_workflow_result(indexes, next_state, opts)

      progressed? ->
        do_run_workflow_graph(
          indexes,
          WorkflowGraphScheduler.pending_node_ids(indexes, next_state),
          input_artifacts,
          next_state,
          opts
        )

      true ->
        stalled_workflow(indexes, next_state)
    end
  catch
    {:workflow_node_error, node_id, reason} ->
      {:error, {:workflow_node_error, node_id, reason}}

    {:workflow_cancelled, node_id} ->
      {:error, {:workflow_cancelled, node_id}}
  end

  defp do_run_workflow_graph(indexes, [], _input_artifacts, state, opts) do
    if WorkflowGraphScheduler.complete?(indexes, state),
      do: finalize_workflow_result(indexes, state, opts),
      else: stalled_workflow(indexes, state)
  end

  defp do_run_workflow_graph(
         indexes,
         [node_id | rest],
         input_artifacts,
         state,
         opts
       ) do
    state = %{state | loop_passes: state.loop_passes + 1}

    case process_workflow_node(node_id, indexes, input_artifacts, state, opts) do
      {:ok, updated, true} ->
        do_run_workflow_graph(
          indexes,
          WorkflowGraphScheduler.enqueue_downstream(rest, indexes, node_id),
          input_artifacts,
          updated,
          opts
        )

      {:ok, updated, false} ->
        do_run_workflow_graph(indexes, rest, input_artifacts, updated, opts)
    end
  catch
    {:workflow_node_error, node_id, reason} ->
      {:error, {:workflow_node_error, node_id, reason}}

    {:workflow_cancelled, node_id} ->
      {:error, {:workflow_cancelled, node_id}}
  end

  defp process_workflow_node(node_id, indexes, input_artifacts, state, opts) do
    node = Map.fetch!(indexes.nodes_by_id, node_id)
    incoming = Map.get(indexes.incoming_edges_by_node, node_id, [])
    kind = Map.get(node, "kind")

    cond do
      WorkflowGraphScheduler.resolved?(state, node_id) ->
        {:ok, state, false}

      kind == "input" or workflow_node_ready?(node, incoming, state) ->
        started_at_us = System.monotonic_time(:microsecond)

        binding =
          WorkflowGraphProvenance.input_digest(
            state,
            consumed_artifacts_for_node(node, incoming, state.artifacts)
          )

        opts = Keyword.put(opts, :input_binding_sha256, binding)

        case execute_workflow_node(node, incoming, input_artifacts, state, opts) do
          {:ok, next_state} ->
            updated =
              mark_completed(
                next_state,
                node_id,
                node,
                incoming,
                (System.monotonic_time(:microsecond) - started_at_us) / 1000.0,
                opts
              )

            maybe_emit_progress(updated, node_id, "completed", indexes.node_count, opts)
            {:ok, updated, true}

          {:error, {:native_workflow_recovery_blocked, _} = reason} ->
            throw({:workflow_node_error, node_id, reason})

          {:error, {:workflow_solve_checkpoint_blocked, _} = reason} ->
            throw({:workflow_node_error, node_id, reason})

          {:error, {:workflow_branch_checkpoint_blocked, _} = reason} ->
            throw({:workflow_node_error, node_id, reason})

          {:error, reason} ->
            if WorkflowGraphRecovery.skip_on_error?(Map.get(node, "config")) do
              updated =
                WorkflowGraphRunnerMetrics.mark_failed(
                  state,
                  node,
                  incoming_artifact_keys(incoming, state.artifacts),
                  reason,
                  (System.monotonic_time(:microsecond) - started_at_us) / 1000.0,
                  Keyword.get(opts, :result_options, %{})
                )

              maybe_emit_progress(updated, node_id, "failed", indexes.node_count, opts)
              {:ok, updated, true}
            else
              throw({:workflow_node_error, node_id, reason})
            end
        end

      unresolved_missing_inputs?(incoming, state) ->
        updated = mark_skipped(state, node_id, node, incoming, opts)
        maybe_emit_progress(updated, node_id, "skipped", indexes.node_count, opts)
        {:ok, updated, true}

      true ->
        {:ok, state, false}
    end
  end

  defp finalize_workflow_result(indexes, state, opts) do
    graph = %{
      "nodes" => indexes.nodes,
      "edges" => indexes.edges,
      "dataset_contract" => Keyword.get(opts, :dataset_contract)
    }

    {:ok,
     WorkflowGraphRunnerMetrics.finalize_result(
       state,
       graph,
       state.artifact_lineage,
       Keyword.get(opts, :result_options, %{})
     )}
  end

  defp stalled_workflow(indexes, state),
    do: {:error, {:workflow_stalled, WorkflowGraphScheduler.pending_node_ids(indexes, state)}}

  defp workflow_node_ready?(node, incoming, state) do
    if WorkflowGraphInputs.partial_inputs?(node) do
      # Match the Rust topological runner: do not choose a fallback while a preferred input is pending.
      Enum.all?(incoming, &WorkflowGraphScheduler.resolved?(state, get_in(&1, ["from", "node"]))) and
        Enum.any?(incoming, &Map.has_key?(state.artifacts, edge_from_key(&1)))
    else
      Enum.all?(incoming, &Map.has_key?(state.artifacts, edge_from_key(&1)))
    end
  end

  defp unresolved_missing_inputs?(incoming, state) do
    Enum.any?(incoming, fn edge ->
      key = edge_from_key(edge)
      not Map.has_key?(state.artifacts, key)
    end) and
      Enum.all?(incoming, fn edge ->
        key = edge_from_key(edge)
        from_node = get_in(edge, ["from", "node"])

        Map.has_key?(state.artifacts, key) or WorkflowGraphScheduler.resolved?(state, from_node)
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
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts) do
      callback = Keyword.fetch!(opts, :execute_solve)

      result =
        if is_function(callback, 4),
          do: callback.(operator_id, payload, node, Keyword.fetch!(opts, :input_binding_sha256)),
          else: callback.(operator_id, payload, node)

      publish_operator_result(
        state,
        node,
        result,
        incoming_artifact_keys(incoming, state.artifacts)
      )
    end
  end

  defp execute_workflow_node(%{"kind" => "transform"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_transform_payload(node, incoming, state.artifacts) do
      result =
        execute_with_node(
          Keyword.fetch!(opts, :execute_transform),
          operator_id,
          payload,
          node,
          opts
        )

      consumed = consumed_artifacts_for_node(node, incoming, state.artifacts)
      publish_operator_result(state, node, result, consumed)
    end
  end

  defp execute_workflow_node(%{"kind" => "extract"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts) do
      result =
        execute_with_node(
          Keyword.fetch!(opts, :execute_extract),
          operator_id,
          payload,
          node,
          opts
        )

      publish_operator_result(
        state,
        node,
        result,
        incoming_artifact_keys(incoming, state.artifacts)
      )
    end
  end

  defp execute_workflow_node(%{"kind" => "export"} = node, incoming, _inputs, state, opts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts) do
      result =
        Keyword.fetch!(opts, :execute_export).(operator_id, payload, Map.get(node, "config"))

      publish_operator_result(
        state,
        node,
        result,
        incoming_artifact_keys(incoming, state.artifacts)
      )
    end
  end

  defp execute_workflow_node(%{"kind" => "condition"} = node, incoming, _inputs, state, opts) do
    with {:ok, payload} <- resolve_single_input_payload(node, incoming, state.artifacts),
         {:ok, decision} <- execute_condition(payload, node, opts),
         {:ok, selected_output} <- WorkflowCondition.validate_decision(node, decision) do
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
       |> append_branch_decision(node, selected_output, decision["predicate_result"])}
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

  defp execute_condition(payload, node, opts) do
    case Keyword.get(opts, :execute_condition, &WorkflowCondition.decide/2) do
      callback when is_function(callback, 2) or is_function(callback, 3) ->
        result =
          if is_function(callback, 3),
            do: callback.(payload, node, Keyword.fetch!(opts, :input_binding_sha256)),
            else: callback.(payload, node)

        case result do
          {:ok, _decision} = result -> result
          {:error, _reason} = error -> error
          _ -> {:error, :invalid_condition_result}
        end

      _ ->
        {:error, :invalid_condition_executor}
    end
  end

  defp execute_with_node(callback, id, payload, node, opts) do
    cond do
      is_function(callback, 5) ->
        callback.(
          id,
          payload,
          Map.get(node, "config"),
          node,
          Keyword.fetch!(opts, :input_binding_sha256)
        )

      is_function(callback, 4) ->
        callback.(id, payload, Map.get(node, "config"), node)

      true ->
        callback.(id, payload, Map.get(node, "config"))
    end
  end

  defp fetch_operator_id(%{"operator_id" => operator_id})
       when is_binary(operator_id) and operator_id != "",
       do: {:ok, operator_id}

  defp fetch_operator_id(_node), do: {:error, :missing_operator_id}

  defp resolve_single_input_payload(%{"id" => node_id}, incoming, artifacts) do
    case incoming do
      [first] ->
        key = edge_from_key(first)

        case Map.fetch(artifacts, key) do
          {:ok, value} -> {:ok, value}
          :error -> {:error, {:missing_upstream_artifact, key}}
        end

      [] ->
        {:error, {:missing_workflow_input, node_id}}

      _multiple ->
        {:error, {:ambiguous_workflow_input, node_id}}
    end
  end

  defp resolve_transform_payload(node, incoming, artifacts) do
    if WorkflowGraphInputs.partial_inputs?(node) do
      case Enum.find(incoming, &Map.has_key?(artifacts, edge_from_key(&1))) do
        nil -> {:error, {:missing_workflow_input, Map.get(node, "id")}}
        edge -> {:ok, Map.fetch!(artifacts, edge_from_key(edge))}
      end
    else
      if WorkflowGraphInputs.named_inputs?(node) and not input_envelope?(node, incoming) do
        resolve_named_transform_payload(node, incoming, artifacts)
      else
        resolve_single_input_payload(node, incoming, artifacts)
      end
    end
  end

  # A single explicit envelope port preserves direct invocation; a missing named
  # input must never be mistaken for an already assembled multi-input payload.
  defp input_envelope?(%{"inputs" => [%{"id" => port}]} = node, [edge])
       when port in ["input", "payload"],
       do: WorkflowGraphInputs.input_envelope?(node) and get_in(edge, ["to", "port"]) == port

  defp input_envelope?(_node, _incoming), do: false

  defp resolve_named_transform_payload(%{"id" => node_id} = node, incoming, artifacts) do
    payload =
      Enum.reduce_while(incoming, %{}, fn edge, acc ->
        key = edge_from_key(edge)

        case Map.fetch(artifacts, key) do
          {:ok, value} ->
            {:cont,
             Map.put(
               acc,
               get_in(edge, ["to", "port"]),
               named_transform_input(value, edge, node)
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

  # Keep single-field legacy port envelopes, but never discard sibling data or
  # status fields merely because an artifact contains a same-named JSON key.
  defp named_transform_input(value, edge, node) do
    if WorkflowGraphInputs.input_envelope?(node) and not (is_map(value) and map_size(value) == 1),
      do: value,
      else: unwrap_named_transform_input(value, edge)
  end

  defp unwrap_named_transform_input(value, edge) when is_map(value) do
    case get_in(edge, ["from", "port"]) do
      port_id when is_binary(port_id) -> Map.get(value, port_id, value)
      _ -> value
    end
  end

  defp unwrap_named_transform_input(value, _edge), do: value

  defp consumed_artifacts_for_node(node, incoming, artifacts) do
    if WorkflowGraphInputs.partial_inputs?(node) do
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

  defp publish_operator_result(state, node, {:ok, value}, source_artifacts) do
    # Validate once before publishing any port or lineage, even for zero-output nodes.
    with :ok <- WorkflowJsonBudget.validate_output(value, "workflow node #{node["id"]} output"),
         do: {:ok, publish_node_outputs(state, node, value, source_artifacts)}
  end

  defp publish_operator_result(_state, _node, {:error, _} = error, _sources), do: error

  defp publish_operator_result(_state, _node, _invalid, _sources),
    do: {:error, :invalid_operator_result}

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
    |> WorkflowGraphProvenance.publish(artifact_key, node_id, port_id, source_artifacts)
  end

  defp append_branch_decision(state, node, selected_output, predicate_result) do
    entry = %{
      "node_id" => Map.get(node, "id"),
      "chosen_output" => Map.get(selected_output, "id"),
      "predicate_result" => predicate_result
    }

    %{state | branch_decisions: [entry | state.branch_decisions]}
  end

  defp mark_completed(state, node_id, node, incoming, duration_ms, opts),
    do:
      WorkflowGraphRunnerMetrics.mark_completed(
        state,
        node_id,
        node,
        incoming,
        duration_ms,
        Keyword.get(opts, :result_options, %{}),
        &WorkflowGraphInputs.partial_inputs?/1,
        &consumed_artifacts_for_node/3,
        &incoming_artifact_keys/2,
        &artifact_key/2
      )

  defp mark_skipped(state, node_id, node, incoming, opts),
    do:
      WorkflowGraphRunnerMetrics.mark_skipped(
        state,
        node_id,
        node,
        incoming,
        Keyword.get(opts, :result_options, %{}),
        &incoming_artifact_keys/2
      )

  defp maybe_emit_progress(state, node_id, status, node_count, opts) do
    case Keyword.get(opts, :progress_callback) do
      callback when is_function(callback, 1) ->
        callback.(%{
          "node_id" => node_id,
          "status" => status,
          "completed_nodes" => MapSet.size(state.completed),
          "skipped_nodes" => MapSet.size(state.skipped),
          "failed_nodes" => MapSet.size(state.failed),
          "resolved_nodes" =>
            MapSet.size(state.completed) + MapSet.size(state.skipped) + MapSet.size(state.failed),
          "total_nodes" => node_count
        })

      _ ->
        :ok
    end
  end

  defp edge_from_key(edge),
    do: artifact_key(get_in(edge, ["from", "node"]), get_in(edge, ["from", "port"]))

  defp artifact_key(node_id, port_id), do: "#{node_id}.#{port_id}"
end
