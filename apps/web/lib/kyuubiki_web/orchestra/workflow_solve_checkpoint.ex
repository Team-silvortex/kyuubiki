defmodule KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint do
  @moduledoc "Bounded, internal successful-solve artifacts; not replay authority or a graph snapshot."

  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.WorkflowJsonBudget

  @key "solve_checkpoints"
  @schema "kyuubiki.workflow-solve-checkpoints/v1"
  @max_entries 32
  @max_output_bytes 524_288
  @max_bytes 2_097_152
  @max_json_nodes 250_000
  @max_output_json_nodes 65_536
  @bucket_keys ~w(schema_version job_id envelope_sha256 entries sha256)
  @entry_keys ~w(node_id node_sha256 invocation_sha256 result_sha256 result_json producer_claim)
  @request_keys ~w(node_id node_sha256 invocation_sha256)
  @claim_keys ~w(generation attempt owner_session_id)
  @reasons ~w(invalid input_changed conflict storage_unavailable commit_failed)a

  def key, do: @key
  def blocking_reason?(reason), do: reason in @reasons

  def policy do
    %{
      "schema_version" => @schema,
      "mode" => "bounded_successful_solve_artifacts",
      "max_entries" => @max_entries,
      "max_output_bytes" => @max_output_bytes,
      "max_bytes" => @max_bytes,
      "max_json_nodes" => @max_json_nodes,
      "max_output_json_nodes" => @max_output_json_nodes,
      "oversize_action" => "not_retained_original_replay_policy_applies"
    }
  end

  def request(node, payload, context) do
    %{
      "node_id" => node["id"],
      "node_sha256" => digest(node),
      "invocation_sha256" =>
        %{"node" => normalize(node), "input" => payload, "context" => normalize(context)}
        |> DurableJson.encode!()
        |> sha256()
    }
  end

  # Callbacks are internal execution options, never accepted from graph request JSON.
  def run(node, payload, context, options, execute) do
    case Keyword.get(options, :solve_checkpoint) do
      nil ->
        execute.()

      callback when is_function(callback, 1) ->
        request = request(node, payload, context)

        case callback.({:lookup, request}) do
          :execute -> execute_and_commit(request, callback, execute)
          {:ok, result} -> {:ok, result}
          {:error, reason} -> blocked(reason, :storage_unavailable)
          _ -> blocked(:invalid, :invalid)
        end

      _ ->
        blocked(:invalid, :invalid)
    end
  end

  def verify(recovery, job_id) do
    with {:ok, execution} <- WorkflowRecoveryEnvelope.execution(recovery),
         do: verify_with_execution(recovery, job_id, execution)
  end

  defp verify_with_execution(recovery, job_id, execution) do
    case Map.fetch(recovery, @key) do
      :error -> :ok
      {:ok, bucket} -> verify_bucket(bucket, recovery, job_id, execution)
    end
  end

  def lookup(recovery, request, claim) do
    with {:ok, execution} <- WorkflowRecoveryEnvelope.execution(recovery),
         true <- valid_claim?(claim),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
         :ok <- verify_request(execution, request),
         :ok <- verify_with_execution(recovery, parent(execution), execution) do
      case get_in(recovery, [@key, "entries", request["node_id"]]) do
        nil ->
          :execute

        entry ->
          cond do
            entry["invocation_sha256"] != request["invocation_sha256"] -> {:error, :input_changed}
            entry["producer_claim"]["generation"] >= claim["generation"] -> {:error, :conflict}
            true -> {:ok, Jason.decode!(entry["result_json"])}
          end
      end
    else
      false -> {:error, :invalid}
      error -> error
    end
  end

  def put(recovery, request, result, claim) do
    with {:ok, execution} <- WorkflowRecoveryEnvelope.execution(recovery),
         :ok <- verify_request(execution, request),
         :ok <- verify_with_execution(recovery, parent(execution), execution),
         true <- valid_claim?(claim),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim),
         :ok <- WorkflowJsonBudget.validate_output(result, "solve checkpoint output") do
      bucket = Map.get(recovery, @key, new_bucket(recovery, execution))

      if not within_output_budget?(result) do
        if Map.has_key?(bucket["entries"], request["node_id"]),
          do: {:error, :conflict},
          else: {:ok, recovery, :not_retained}
      else
        result_json = DurableJson.encode!(result)

        entry =
          Map.merge(request, %{
            "result_json" => result_json,
            "result_sha256" => sha256(result_json),
            "producer_claim" => Map.take(claim, @claim_keys)
          })

        case Map.fetch(bucket["entries"], request["node_id"]) do
          {:ok, ^entry} -> {:ok, recovery, :unchanged}
          {:ok, _} -> {:error, :conflict}
          :error -> retain_bounded(recovery, bucket, entry)
        end
      end
    else
      _ -> {:error, :invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] -> {:error, :invalid}
  end

  defp execute_and_commit(request, callback, execute) do
    case execute.() do
      {:ok, result} ->
        with :ok <- WorkflowJsonBudget.validate_output(result, "workflow solve output") do
          case callback.({:commit, request, result}) do
            :ok -> {:ok, result}
            {:error, reason} -> blocked(reason, :commit_failed)
            _ -> blocked(:invalid, :invalid)
          end
        end

      other ->
        other
    end
  end

  defp blocked(reason, fallback) do
    {:error,
     {:workflow_solve_checkpoint_blocked,
      if(blocking_reason?(reason), do: reason, else: fallback)}}
  end

  defp retain_bounded(recovery, bucket, entry) do
    next = put_in(bucket, ["entries", entry["node_id"]], entry) |> seal()

    if map_size(next["entries"]) > @max_entries or
         byte_size(DurableJson.encode!(next)) > @max_bytes or
         WorkflowJsonBudget.validate(next, "solve checkpoints", @max_json_nodes) != :ok do
      {:ok, recovery, :not_retained}
    else
      {:ok, Map.put(recovery, @key, next), :retained}
    end
  end

  defp new_bucket(recovery, execution) do
    %{
      "schema_version" => @schema,
      "job_id" => parent(execution),
      "envelope_sha256" => recovery["envelope_sha256"],
      "entries" => %{}
    }
  end

  defp verify_bucket(bucket, recovery, job_id, execution) when is_map(bucket) do
    with true <- closed?(bucket, @bucket_keys),
         true <- bucket["schema_version"] == @schema,
         true <- bucket["job_id"] == job_id and job_id == parent(execution),
         true <- bucket["envelope_sha256"] == recovery["envelope_sha256"],
         true <- is_map(bucket["entries"]) and map_size(bucket["entries"]) <= @max_entries,
         :ok <- WorkflowJsonBudget.validate(bucket, "solve checkpoints", @max_json_nodes),
         true <- byte_size(DurableJson.encode!(bucket)) <= @max_bytes,
         true <- valid_digest?(bucket["sha256"]),
         true <- digest(Map.delete(bucket, "sha256")) == bucket["sha256"],
         true <- Enum.all?(bucket["entries"], &valid_entry?(&1, recovery, execution)) do
      :ok
    else
      _ -> {:error, :invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] -> {:error, :invalid}
  end

  defp verify_bucket(_bucket, _recovery, _job_id, _execution), do: {:error, :invalid}

  defp valid_entry?({id, entry}, recovery, execution) when is_map(entry) do
    closed?(entry, @entry_keys) and entry["node_id"] == id and
      verify_request(execution, Map.take(entry, @request_keys)) == :ok and
      valid_claim?(entry["producer_claim"]) and
      entry["producer_claim"]["generation"] <= recovery["generation"] and
      entry["producer_claim"]["attempt"] <= recovery["attempt"] and
      valid_output?(entry)
  end

  defp valid_entry?(_entry, _recovery, _execution), do: false

  # Encoded JSON is a value, not a path: JSONB must not turn 1.0 into 1 and change
  # the downstream native TaskIR digest after a restart.
  defp valid_output?(entry) do
    with json when is_binary(json) <- entry["result_json"],
         true <- byte_size(json) <= @max_output_bytes,
         true <- valid_digest?(entry["result_sha256"]),
         true <- sha256(json) == entry["result_sha256"],
         {:ok, output} <- Jason.decode(json),
         :ok <- WorkflowJsonBudget.validate_output(output, "solve checkpoint output"),
         :ok <-
           WorkflowJsonBudget.validate(output, "solve checkpoint output", @max_output_json_nodes) do
      true
    else
      _ -> false
    end
  end

  defp within_output_budget?(value) do
    WorkflowJsonBudget.validate(value, "solve checkpoint output", @max_output_json_nodes) == :ok and
      case encoded_size(value, 0) do
        {:ok, bytes} -> bytes <= @max_output_bytes
        _ -> false
      end
  end

  # Stop counting as soon as the byte cap is crossed, before building a large JSON copy.
  defp encoded_size(_value, count) when count > @max_output_bytes, do: :too_large

  defp encoded_size(value, count) when is_map(value) do
    Enum.reduce_while(value, {:ok, count + 2}, fn {key, item}, {:ok, bytes} ->
      result = with {:ok, bytes} <- encoded_size(key, bytes + 2), do: encoded_size(item, bytes)
      size_step(result)
    end)
  end

  defp encoded_size(value, count) when is_list(value) do
    Enum.reduce_while(value, {:ok, count + 2}, fn item, {:ok, bytes} ->
      size_step(encoded_size(item, bytes + 1))
    end)
  end

  defp encoded_size(value, count) do
    if is_binary(value) and byte_size(value) + count > @max_output_bytes do
      :too_large
    else
      bytes = count + byte_size(Jason.encode!(value))
      if bytes <= @max_output_bytes, do: {:ok, bytes}, else: :too_large
    end
  end

  defp size_step({:ok, _} = result), do: {:cont, result}
  defp size_step(:too_large), do: {:halt, :too_large}

  defp verify_request(execution, request) when is_map(request) do
    nodes = get_in(execution, ["graph", "nodes"]) || []

    node =
      if is_list(nodes) and Enum.all?(nodes, &is_map/1),
        do: Enum.find(nodes, &(&1["id"] == request["node_id"]))

    if closed?(request, @request_keys) and is_map(node) and node["kind"] == "solve" and
         request["node_sha256"] == digest(node) and valid_digest?(request["invocation_sha256"]),
       do: :ok,
       else: {:error, :invalid}
  end

  defp verify_request(_execution, _request), do: {:error, :invalid}

  defp valid_claim?(claim) when is_map(claim) do
    closed?(claim, @claim_keys) and is_integer(claim["generation"]) and claim["generation"] > 0 and
      is_integer(claim["attempt"]) and claim["attempt"] > 0 and
      is_binary(claim["owner_session_id"]) and byte_size(claim["owner_session_id"]) in 1..256
  end

  defp valid_claim?(_claim), do: false

  defp parent(execution), do: get_in(execution, ["orchestration_context", "job_id"])

  defp closed?(map, keys), do: Map.keys(map) |> Enum.sort() == Enum.sort(keys)
  defp seal(bucket), do: Map.put(bucket, "sha256", digest(Map.delete(bucket, "sha256")))

  defp digest(value) do
    value |> normalize() |> DurableJson.encode!() |> sha256()
  end

  defp sha256(bytes), do: :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)

  # Normalize envelope/node metadata only; result JSON and invocation inputs stay lossless.
  defp normalize(value) when is_map(value), do: Map.new(value, fn {k, v} -> {k, normalize(v)} end)
  defp normalize(value) when is_list(value), do: Enum.map(value, &normalize/1)

  defp normalize(value) when is_float(value),
    do: if(value == trunc(value), do: trunc(value), else: value)

  defp normalize(value), do: value
  defp valid_digest?(value) when is_binary(value), do: Regex.match?(~r/\A[0-9a-f]{64}\z/, value)
  defp valid_digest?(_value), do: false
end
