defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint do
  @moduledoc "Bounded native success receipts tied to durable attempts, never replay authority."

  alias KyuubikiWeb.Orchestra.{OperatorTaskCompletion, WorkflowNativeDispatchEvidence}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.WorkflowJsonBudget

  @key "native_result_checkpoints"
  @schema "kyuubiki.workflow-native-result-checkpoints/v1"
  @max_entries 32
  @max_receipt_bytes 524_288
  @max_bytes 2_097_152
  @max_receipt_nodes 65_536
  @identity ~w(attempt_id task_id task_digest operator_id program_id)
  @request @identity ++ ~w(observation_sha256)
  @bucket ~w(schema_version job_id envelope_sha256 entries sha256)
  @entry @request ++ ~w(receipt_json receipt_sha256 producer_claim)
  @claim ~w(generation attempt owner_session_id)

  def key, do: @key

  def request(record),
    do: Map.put(Map.take(record, @identity), "observation_sha256", digest(record))

  def policy do
    %{
      "schema_version" => @schema,
      "mode" => "bounded_verified_native_receipts",
      "max_entries" => @max_entries,
      "max_receipt_bytes" => @max_receipt_bytes,
      "max_bytes" => @max_bytes,
      "max_receipt_json_nodes" => @max_receipt_nodes,
      "oversize_action" => "not_retained_original_receipt_required",
      "journal_required" => true,
      "automatic_replay_authorized" => false
    }
  end

  def fetch(record, opts) do
    case invoke(opts, {:lookup, request(record)}, :missing) do
      :missing -> :missing
      {:ok, receipt} -> {:ok, receipt}
      {:error, reason} -> blocked(reason, :result_checkpoint_unavailable)
      _ -> blocked(:invalid, :result_checkpoint_invalid)
    end
  end

  def commit(record, receipt, opts) do
    case invoke(opts, {:commit, request(record), receipt}, :ok) do
      :ok -> :ok
      {:error, reason} -> blocked(reason, :result_checkpoint_commit_failed)
      _ -> blocked(:invalid, :result_checkpoint_invalid)
    end
  end

  defp invoke(opts, action, default) do
    case Keyword.get(opts, :native_result_checkpoint) do
      nil -> default
      callback when is_function(callback, 1) -> callback.(action)
      _ -> {:error, :invalid}
    end
  rescue
    _ -> {:error, :storage_unavailable}
  catch
    _, _ -> {:error, :storage_unavailable}
  end

  defp blocked(reason, fallback),
    do:
      {:error,
       {:native_workflow_recovery_blocked,
        if(reason in [:invalid, :conflict, :input_changed],
          do: :result_checkpoint_invalid,
          else: fallback
        )}}

  def verify(recovery, job_id) do
    case Map.fetch(recovery, @key) do
      :error ->
        :ok

      {:ok, bucket} ->
        with :ok <- WorkflowNativeDispatchEvidence.verify(recovery, job_id),
             do: verify_bucket(bucket, recovery, job_id)
    end
  end

  def lookup(recovery, query, claim) do
    with :ok <- admissible(recovery, query, claim),
         :ok <- verify(recovery, parent(recovery)) do
      case get_in(recovery, [@key, "entries", query["attempt_id"]]) do
        nil ->
          :missing

        entry ->
          if Map.take(entry, @request) == query and
               entry["producer_claim"]["generation"] < claim["generation"],
             do: {:ok, Jason.decode!(entry["receipt_json"])},
             else: {:error, :conflict}
      end
    end
  end

  def put(recovery, query, receipt, claim) do
    with :ok <- admissible(recovery, query, claim),
         :ok <- verify(recovery, parent(recovery)),
         {:ok, %{"status" => "executed"}} <- OperatorTaskCompletion.agent_receipt(query, receipt),
         :ok <- WorkflowJsonBudget.validate_output(receipt, "native receipt checkpoint") do
      bucket = Map.get(recovery, @key, new_bucket(recovery))

      if WorkflowJsonBudget.within_encoded_budget?(
           receipt,
           @max_receipt_bytes,
           @max_receipt_nodes
         ) do
        json = DurableJson.encode!(receipt)

        entry =
          Map.merge(query, %{
            "receipt_json" => json,
            "receipt_sha256" => digest_bytes(json),
            "producer_claim" => claim
          })

        case Map.fetch(bucket["entries"], query["attempt_id"]) do
          {:ok, ^entry} -> {:ok, recovery, :unchanged}
          {:ok, _} -> {:error, :conflict}
          :error -> retain(recovery, bucket, entry)
        end
      else
        if Map.has_key?(bucket["entries"], query["attempt_id"]),
          do: {:error, :conflict},
          else: {:ok, recovery, :not_retained}
      end
    else
      _ -> {:error, :invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] -> {:error, :invalid}
  end

  defp admissible(recovery, query, claim) do
    with true <- valid_claim?(claim) and Envelope.fenced?(recovery, claim),
         :ok <- WorkflowNativeDispatchEvidence.verify(recovery, parent(recovery)),
         true <- valid_request?(query, recovery) do
      :ok
    else
      _ -> {:error, :invalid}
    end
  end

  defp retain(recovery, bucket, entry) do
    updated = put_in(bucket, ["entries", entry["attempt_id"]], entry) |> seal()

    if map_size(updated["entries"]) <= @max_entries and
         byte_size(DurableJson.encode!(updated)) <= @max_bytes,
       do: {:ok, Map.put(recovery, @key, updated), :retained},
       else: {:ok, recovery, :not_retained}
  end

  defp new_bucket(recovery),
    do: %{
      "schema_version" => @schema,
      "job_id" => parent(recovery),
      "envelope_sha256" => recovery["envelope_sha256"],
      "entries" => %{}
    }

  defp verify_bucket(bucket, recovery, job_id) when is_map(bucket) do
    with true <- closed?(bucket, @bucket),
         true <- bucket["schema_version"] == @schema and bucket["job_id"] == job_id,
         true <- bucket["envelope_sha256"] == recovery["envelope_sha256"],
         true <- is_map(bucket["entries"]) and map_size(bucket["entries"]) <= @max_entries,
         true <- Enum.all?(bucket["entries"], &valid_entry?(&1, recovery)),
         true <- byte_size(DurableJson.encode!(bucket)) <= @max_bytes,
         true <- bucket["sha256"] == digest(Map.delete(bucket, "sha256")) do
      :ok
    else
      _ -> {:error, :invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] -> {:error, :invalid}
  end

  defp verify_bucket(_, _, _), do: {:error, :invalid}

  defp valid_entry?({id, entry}, recovery) when is_map(entry) do
    with true <- closed?(entry, @entry) and entry["attempt_id"] == id,
         true <- valid_request?(Map.take(entry, @request), recovery),
         claim <- entry["producer_claim"],
         true <- valid_claim?(claim),
         source <-
           get_in(recovery, [
             WorkflowNativeDispatchEvidence.key(),
             "entries",
             id,
             "producer_claim"
           ]),
         true <-
           claim["generation"] >= source["generation"] and claim["attempt"] >= source["attempt"],
         true <-
           claim["generation"] != source["generation"] or
             claim["owner_session_id"] == source["owner_session_id"],
         true <-
           claim["generation"] <= recovery["generation"] and
             claim["attempt"] <= recovery["attempt"],
         true <-
           claim["generation"] != recovery["generation"] or
             claim["owner_session_id"] == recovery["owner_session_id"],
         json when is_binary(json) and byte_size(json) <= @max_receipt_bytes <-
           entry["receipt_json"],
         true <- digest_bytes(json) == entry["receipt_sha256"],
         {:ok, receipt} <- Jason.decode(json),
         :ok <- WorkflowJsonBudget.validate_output(receipt, "native receipt checkpoint"),
         :ok <-
           WorkflowJsonBudget.validate(receipt, "native receipt checkpoint", @max_receipt_nodes),
         {:ok, %{"status" => "executed"}} <- OperatorTaskCompletion.agent_receipt(entry, receipt) do
      true
    else
      _ -> false
    end
  end

  defp valid_entry?(_, _), do: false

  defp valid_request?(query, recovery) when is_map(query) do
    closed?(query, @request) and
      is_binary(query["observation_sha256"]) and
      Regex.match?(~r/\A[0-9a-f]{64}\z/, query["observation_sha256"]) and
      Map.take(query, @identity) in WorkflowNativeDispatchEvidence.expected(recovery)
  end

  defp valid_request?(_, _), do: false

  defp valid_claim?(claim) when is_map(claim) do
    closed?(claim, @claim) and is_integer(claim["generation"]) and claim["generation"] > 0 and
      is_integer(claim["attempt"]) and claim["attempt"] > 0 and
      is_binary(claim["owner_session_id"]) and byte_size(claim["owner_session_id"]) in 1..256
  end

  defp valid_claim?(_), do: false

  defp parent(recovery),
    do: Envelope.execution_value(recovery, ["orchestration_context", "job_id"])

  defp closed?(map, keys), do: Enum.sort(Map.keys(map)) == Enum.sort(keys)
  defp seal(bucket), do: Map.put(bucket, "sha256", digest(Map.delete(bucket, "sha256")))
  defp digest(value), do: value |> DurableJson.encode!() |> digest_bytes()
  defp digest_bytes(value), do: :crypto.hash(:sha256, value) |> Base.encode16(case: :lower)
end
