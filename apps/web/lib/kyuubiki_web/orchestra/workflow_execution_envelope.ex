defmodule KyuubikiWeb.Orchestra.WorkflowExecutionEnvelope do
  @moduledoc "Single-copy lossless execution storage for explicitly typed native workflows."

  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.WorkflowTaskDigestPolicy, as: Policy

  @legacy "kyuubiki.workflow-execution-envelope/v1"
  @lossless "kyuubiki.workflow-execution-envelope/v2"

  def pack(graph, inputs, context, options) do
    with {:ok, encoding} <- Policy.resolve(graph) do
      execution = %{
        "schema_version" => @legacy,
        "graph" => graph,
        "input_artifacts" => inputs,
        "orchestration_context" => context,
        "response_options" => options
      }

      stored =
        if encoding == Policy.legacy(),
          do: normalize(execution),
          else: %{
            "schema_version" => @lossless,
            "execution_json" => DurableJson.encode!(execution)
          }

      {:ok, stored, encoding}
    end
  end

  def unpack(%{"schema_version" => @legacy} = stored) do
    with :ok <- valid_execution(stored),
         {:ok, encoding} <- Policy.resolve(stored["graph"]),
         true <- encoding == Policy.legacy() do
      {:ok, stored}
    else
      _ -> {:error, :invalid_workflow_execution_envelope}
    end
  end

  def unpack(%{"schema_version" => @lossless, "execution_json" => json} = stored)
      when is_binary(json) and map_size(stored) == 2 do
    with {:ok, %{"schema_version" => @legacy} = execution} <- Jason.decode(json),
         :ok <- valid_execution(execution),
         {:ok, encoding} <- Policy.resolve(execution["graph"]),
         true <- encoding != Policy.legacy(),
         true <- DurableJson.encode!(execution) == json do
      {:ok, execution}
    else
      _ -> {:error, :invalid_workflow_execution_envelope}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] ->
      {:error, :invalid_workflow_execution_envelope}
  end

  def unpack(_), do: {:error, :invalid_workflow_execution_envelope}

  def value(recovery, path) do
    case unpack(Map.get(recovery, "envelope")) do
      {:ok, execution} -> get_in(execution, path)
      _ -> nil
    end
  end

  defp valid_execution(execution) do
    if Enum.all?(
         ~w(graph input_artifacts orchestration_context response_options),
         &is_map(Map.get(execution, &1))
       ),
       do: :ok,
       else: {:error, :invalid_workflow_execution_envelope}
  end

  # Legacy records deliberately retain their existing JSONB normalization contract.
  defp normalize(value) when is_map(value), do: Map.new(value, fn {k, v} -> {k, normalize(v)} end)
  defp normalize(value) when is_list(value), do: Enum.map(value, &normalize/1)

  defp normalize(value) when is_float(value),
    do: if(value == trunc(value), do: trunc(value), else: value)

  defp normalize(value), do: value
end
