defmodule KyuubikiSdk.WorkflowResults do
  @moduledoc "Workflow output manifest and result validation helpers."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.WorkflowContracts

  def build_output_manifest(graph) when is_map(graph) do
    with {:ok, validated} <- WorkflowContracts.validate_graph(graph) do
      node_map =
        validated
        |> Map.get("nodes", [])
        |> Enum.reduce(%{}, fn
          %{"id" => node_id} = node, acc when is_binary(node_id) -> Map.put(acc, node_id, node)
          _node, acc -> acc
        end)

      outputs =
        validated
        |> Map.get("output_nodes", [])
        |> Enum.flat_map(fn node_id ->
          case Map.get(node_map, node_id) do
            %{"inputs" => inputs} when is_list(inputs) ->
              Enum.flat_map(inputs, fn
                %{"id" => port_id, "artifact_type" => artifact_type} = port
                when is_binary(port_id) and is_binary(artifact_type) ->
                  [
                    %{
                      "key" => "#{node_id}.#{port_id}",
                      "node_id" => node_id,
                      "port_id" => port_id,
                      "artifact_type" => artifact_type,
                      "dataset_value" => Map.get(port, "dataset_value"),
                      "required" => Map.get(port, "required", true)
                    }
                  ]

                _port ->
                  []
              end)

            _ ->
              []
          end
        end)

      {:ok,
       %{
         "graph_id" => validated["id"],
         "graph_version" => validated["version"],
         "outputs" => outputs
       }}
    end
  end

  def build_output_manifest(_graph),
    do: {:error, Error.validation(["graph must be an object"])}

  def validate_result_against_graph(graph, payload) when is_map(payload) do
    with {:ok, manifest} <- build_output_manifest(graph),
         {:ok, artifacts} <- extract_artifacts(payload) do
      {errors, normalized} =
        Enum.reduce(manifest["outputs"], {[], %{}}, fn output, {errors, normalized} ->
          case find_artifact(artifacts, output) do
            nil ->
              if Map.get(output, "required", true) do
                {
                  errors ++
                    ["workflow result is missing required artifact for output #{inspect(output["key"])}"],
                  normalized
                }
              else
                {errors, normalized}
              end

            artifact ->
              next_errors = errors ++ artifact_errors(output, artifact)
              {next_errors, Map.put(normalized, output["key"], artifact)}
          end
        end)

      if errors == [] do
        {:ok, runtime} = normalize_runtime(payload)

        {:ok,
         %{
           "graph_id" => manifest["graph_id"],
           "graph_version" => manifest["graph_version"],
           "manifest" => manifest,
           "workflow_runtime" => runtime,
           "artifacts" => normalized
         }}
      else
        {:error, Error.validation(errors)}
      end
    end
  end

  def validate_result_against_graph(_graph, _payload),
    do: {:error, Error.validation(["workflow result payload must be an object"])}

  def normalize_runtime(payload) when is_map(payload) do
    runtime = extract_runtime_payload(payload)
    errors =
      []
      |> maybe_type_error(Map.get(runtime, "workflow_id"), "workflow runtime workflow_id must be a string", &is_binary/1)
      |> maybe_type_error(Map.get(runtime, "run_id"), "workflow runtime run_id must be a string", &is_binary/1)
      |> maybe_type_error(Map.get(runtime, "status"), "workflow runtime status must be a string", &is_binary/1)
      |> maybe_type_error(Map.get(runtime, "current_node"), "workflow runtime current_node must be a string", &is_binary/1)
      |> maybe_type_error(Map.get(runtime, "completed_nodes"), "workflow runtime completed_nodes must be a list", &is_list/1)
      |> maybe_type_error(Map.get(runtime, "progress_events"), "workflow runtime progress_events must be a list", &is_list/1)
      |> maybe_type_error(Map.get(runtime, "failure"), "workflow runtime failure must be an object", &is_map/1)

    if errors == [] do
      {:ok,
       %{
         "workflow_id" => Map.get(runtime, "workflow_id"),
         "run_id" => Map.get(runtime, "run_id"),
         "status" => Map.get(runtime, "status"),
         "current_node" => Map.get(runtime, "current_node"),
         "completed_nodes" => Map.get(runtime, "completed_nodes", []),
         "progress_events" => Map.get(runtime, "progress_events", []),
         "failure" => Map.get(runtime, "failure")
       }}
    else
      {:error, Error.validation(errors)}
    end
  end

  def normalize_runtime(_payload),
    do: {:error, Error.validation(["workflow result payload must be an object"])}

  def normalize_progression(history, result_payload \\ nil)
  def normalize_progression(history, result_payload) when is_list(history) do
    snapshots =
      history
      |> Enum.with_index()
      |> Enum.flat_map(fn
        {%{"job" => job}, index} when is_map(job) ->
          current_node = Map.get(job, "current_node")
          completed_nodes = Map.get(job, "completed_nodes", [])
          progress_events = Map.get(job, "progress_events", [])

          cond do
            not is_nil(current_node) and not is_binary(current_node) ->
              raise Error.validation(["workflow progression current_node must be a string"])

            not is_list(completed_nodes) ->
              raise Error.validation(["workflow progression completed_nodes must be a list"])

            not is_list(progress_events) ->
              raise Error.validation(["workflow progression progress_events must be a list"])

            true ->
              [
                %{
                  "index" => index,
                  "job_id" => Map.get(job, "job_id"),
                  "status" => Map.get(job, "status"),
                  "progress" => Map.get(job, "progress"),
                  "current_node" => current_node,
                  "completed_nodes" => completed_nodes,
                  "progress_events" => progress_events
                }
              ]
          end

        _ ->
          []
      end)

    latest =
      case result_payload do
        payload when is_map(payload) ->
          case normalize_runtime(payload) do
            {:ok, runtime} -> runtime
            _ -> List.last(snapshots)
          end

        _ ->
          List.last(snapshots)
      end

    {:ok, %{"snapshots" => snapshots, "latest" => latest}}
  end

  def normalize_progression(_history, _result_payload),
    do: {:error, Error.validation(["workflow progression history must be a list"])}

  defp extract_artifacts(%{"artifacts" => artifacts}) when is_map(artifacts), do: {:ok, artifacts}
  defp extract_artifacts(%{"result" => %{"artifacts" => artifacts}}) when is_map(artifacts), do: {:ok, artifacts}

  defp extract_artifacts(_payload),
    do: {:error, Error.validation(["workflow result payload must include an 'artifacts' object"])}

  defp extract_runtime_payload(%{"result" => result}) when is_map(result) do
    if Enum.any?(~w(workflow_id run_id status current_node completed_nodes progress_events failure), &Map.has_key?(result, &1)) do
      result
    else
      %{"result" => result}
      |> payload_fallback()
    end
  end

  defp extract_runtime_payload(payload), do: payload

  defp find_artifact(artifacts, output) do
    Enum.find_value(
      [output["key"], output["dataset_value"], output["artifact_type"]],
      fn
        key when is_binary(key) -> Map.get(artifacts, key)
        _ -> nil
      end
    )
  end

  defp artifact_errors(output, artifact) when is_map(artifact) do
    []
    |> maybe_add_artifact_type_error(output, artifact)
    |> maybe_add_dataset_value_error(output, artifact)
  end

  defp artifact_errors(_output, _artifact), do: []

  defp maybe_add_artifact_type_error(errors, output, artifact) do
    case Map.get(artifact, "artifact_type") do
      nil -> errors
      value ->
        if value == Map.get(output, "artifact_type") do
          errors
        else
          errors ++ ["workflow result artifact #{inspect(output["key"])} has mismatched artifact_type"]
        end
    end
  end

  defp maybe_add_dataset_value_error(errors, output, artifact) do
    case {Map.get(output, "dataset_value"), Map.get(artifact, "dataset_value")} do
      {nil, _} -> errors
      {_expected, nil} -> errors
      {expected, expected} -> errors
      {_expected, _actual} -> errors ++ ["workflow result artifact #{inspect(output["key"])} has mismatched dataset_value"]
    end
  end

  defp maybe_type_error(errors, nil, _message, _predicate), do: errors
  defp maybe_type_error(errors, value, message, predicate) when is_function(predicate, 1) do
    if predicate.(value), do: errors, else: errors ++ [message]
  end

  defp payload_fallback(payload), do: payload
end
