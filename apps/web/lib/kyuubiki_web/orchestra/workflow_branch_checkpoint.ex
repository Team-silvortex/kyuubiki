defmodule KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint do
  @moduledoc "Bounded branch decisions, never artifacts, scheduler snapshots, or replay authority."

  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryEnvelope, WorkflowSolveCheckpoint}
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.{WorkflowCondition, WorkflowJsonBudget}

  @key "branch_checkpoints"
  @schema "kyuubiki.workflow-branch-checkpoints/v1"
  @max_entries 64
  @max_bytes 65_536
  @bucket_keys ~w(schema_version job_id envelope_sha256 entries sha256)
  @request_keys ~w(node_id node_sha256 invocation_sha256)
  @entry_keys @request_keys ++ ~w(decision producer_claim)
  @claim_keys ~w(generation attempt owner_session_id)
  @reasons ~w(invalid input_changed conflict storage_unavailable commit_failed)a

  def key, do: @key
  def blocking_reason?(reason), do: reason in @reasons
  defdelegate request(node, payload, context), to: WorkflowSolveCheckpoint

  def policy do
    %{
      "schema_version" => @schema,
      "mode" => "bounded_successful_branch_decisions",
      "max_entries" => @max_entries,
      "max_bytes" => @max_bytes,
      "oversize_action" => "not_retained_original_replay_policy_applies"
    }
  end

  # Only the trusted engine supplies this callback; graph JSON cannot enable reuse.
  def run(node, payload, context, options, execute) do
    case Keyword.get(options, :branch_checkpoint) do
      nil ->
        execute.()

      callback when is_function(callback, 1) ->
        request = request(node, payload, context)

        case callback.({:lookup, request}) do
          :execute -> execute_and_commit(node, request, callback, execute)
          {:ok, decision} -> checked_decision(node, decision)
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
         {:ok, _node} <- verify_request(execution, request),
         :ok <- verify_with_execution(recovery, parent(execution), execution) do
      case get_in(recovery, [@key, "entries", request["node_id"]]) do
        nil ->
          :execute

        entry ->
          cond do
            entry["invocation_sha256"] != request["invocation_sha256"] -> {:error, :input_changed}
            entry["producer_claim"]["generation"] >= claim["generation"] -> {:error, :conflict}
            true -> {:ok, entry["decision"]}
          end
      end
    else
      false -> {:error, :invalid}
      error -> error
    end
  end

  def put(recovery, request, decision, claim) do
    with {:ok, execution} <- WorkflowRecoveryEnvelope.execution(recovery),
         {:ok, node} <- verify_request(execution, request),
         {:ok, _output} <- WorkflowCondition.validate_decision(node, decision),
         :ok <- verify_with_execution(recovery, parent(execution), execution),
         true <- valid_claim?(claim),
         true <- WorkflowRecoveryEnvelope.fenced?(recovery, claim) do
      bucket = Map.get(recovery, @key, new_bucket(recovery, execution))
      entry = Map.merge(request, %{"decision" => decision, "producer_claim" => claim})

      case Map.fetch(bucket["entries"], request["node_id"]) do
        {:ok, ^entry} -> {:ok, recovery, :unchanged}
        {:ok, _} -> {:error, :conflict}
        :error -> retain_bounded(recovery, bucket, entry)
      end
    else
      _ -> {:error, :invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] -> {:error, :invalid}
  end

  defp execute_and_commit(node, request, callback, execute) do
    case execute.() do
      {:ok, decision} ->
        with {:ok, decision} <- checked_decision(node, decision) do
          case callback.({:commit, request, decision}) do
            :ok -> {:ok, decision}
            {:error, reason} -> blocked(reason, :commit_failed)
            _ -> blocked(:invalid, :invalid)
          end
        end

      {:error, _} = error ->
        error

      _ ->
        blocked(:invalid, :invalid)
    end
  end

  defp checked_decision(node, decision) do
    case WorkflowCondition.validate_decision(node, decision) do
      {:ok, _} -> {:ok, decision}
      _ -> blocked(:invalid, :invalid)
    end
  end

  defp blocked(reason, fallback),
    do:
      {:error,
       {:workflow_branch_checkpoint_blocked,
        if(blocking_reason?(reason), do: reason, else: fallback)}}

  defp retain_bounded(recovery, bucket, entry) do
    next = put_in(bucket, ["entries", entry["node_id"]], entry) |> seal()

    if map_size(next["entries"]) > @max_entries or
         byte_size(DurableJson.encode!(next)) > @max_bytes do
      {:ok, recovery, :not_retained}
    else
      {:ok, Map.put(recovery, @key, next), :retained}
    end
  end

  defp new_bucket(recovery, execution),
    do: %{
      "schema_version" => @schema,
      "job_id" => parent(execution),
      "envelope_sha256" => recovery["envelope_sha256"],
      "entries" => %{}
    }

  defp verify_bucket(bucket, recovery, job_id, execution) when is_map(bucket) do
    with true <- closed?(bucket, @bucket_keys),
         true <- bucket["schema_version"] == @schema,
         true <- bucket["job_id"] == job_id and job_id == parent(execution),
         true <- bucket["envelope_sha256"] == recovery["envelope_sha256"],
         true <- is_map(bucket["entries"]) and map_size(bucket["entries"]) <= @max_entries,
         :ok <- WorkflowJsonBudget.validate(bucket, "branch checkpoints", 8_192),
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
    with true <- closed?(entry, @entry_keys) and entry["node_id"] == id,
         {:ok, node} <- verify_request(execution, Map.take(entry, @request_keys)),
         true <- valid_claim?(entry["producer_claim"]),
         true <- entry["producer_claim"]["generation"] <= recovery["generation"],
         true <- entry["producer_claim"]["attempt"] <= recovery["attempt"],
         {:ok, _} <- WorkflowCondition.validate_decision(node, entry["decision"]) do
      true
    else
      _ -> false
    end
  end

  defp valid_entry?(_entry, _recovery, _execution), do: false

  defp verify_request(execution, request) when is_map(request) do
    nodes = get_in(execution, ["graph", "nodes"]) || []

    node =
      if is_list(nodes) and Enum.all?(nodes, &is_map/1),
        do: Enum.find(nodes, &(&1["id"] == request["node_id"]))

    if closed?(request, @request_keys) and is_map(node) and node["kind"] == "condition" and
         request["node_sha256"] == request(node, nil, %{})["node_sha256"] and
         valid_digest?(request["invocation_sha256"]),
       do: {:ok, node},
       else: {:error, :invalid}
  end

  defp verify_request(_execution, _request), do: {:error, :invalid}

  defp valid_claim?(claim) when is_map(claim),
    do:
      closed?(claim, @claim_keys) and is_integer(claim["generation"]) and claim["generation"] > 0 and
        is_integer(claim["attempt"]) and claim["attempt"] > 0 and
        is_binary(claim["owner_session_id"]) and byte_size(claim["owner_session_id"]) in 1..256

  defp valid_claim?(_claim), do: false

  defp parent(execution), do: get_in(execution, ["orchestration_context", "job_id"])

  defp closed?(map, keys), do: Map.keys(map) |> Enum.sort() == Enum.sort(keys)
  defp seal(bucket), do: Map.put(bucket, "sha256", digest(Map.delete(bucket, "sha256")))

  defp digest(value),
    do: :crypto.hash(:sha256, DurableJson.encode!(value)) |> Base.encode16(case: :lower)

  defp valid_digest?(value) when is_binary(value), do: Regex.match?(~r/\A[0-9a-f]{64}\z/, value)
  defp valid_digest?(_value), do: false
end
