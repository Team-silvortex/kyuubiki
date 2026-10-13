defmodule KyuubikiWeb.Orchestra.WorkflowCompletionCheckpoint do
  @moduledoc "Private final-publication handoff, never authority to execute a graph again."

  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.WorkflowJsonBudget

  @key "pending_completion"
  @schema "kyuubiki.workflow-completion-checkpoint/v1"
  @claim_keys ~w(generation attempt owner_session_id)
  @keys ~w(schema_version job_id job_created_at_us envelope_sha256 producer_claim result_json sha256)
  @max_bytes 67_108_864
  @max_nodes 2_000_000

  def key, do: @key

  def policy do
    %{
      "schema_version" => @schema,
      "max_result_bytes" => @max_bytes,
      "max_json_nodes" => @max_nodes,
      "oversize_action" => "reject_without_claiming_durable_completion",
      "recovery_action" => "publish_only_no_execution"
    }
  end

  def prepare(job, recovery, claim, result) do
    with true <- valid_claim?(claim) and Envelope.fenced?(recovery, claim),
         :ok <- valid_parent(job, recovery),
         true <- is_map(result) and is_list(Map.get(result, "failed_nodes", [])),
         true <- WorkflowJsonBudget.within_encoded_budget?(result, @max_bytes, @max_nodes) do
      record = %{
        "schema_version" => @schema,
        "job_id" => job.job_id,
        "job_created_at_us" => DateTime.to_unix(job.created_at, :microsecond),
        "envelope_sha256" => recovery["envelope_sha256"],
        "producer_claim" => Map.take(claim, @claim_keys),
        "result_json" => DurableJson.encode!(result)
      }

      record = Map.put(record, "sha256", digest(record))

      case Map.fetch(recovery, @key) do
        :error -> {:ok, Map.put(recovery, @key, record), :retained}
        {:ok, ^record} -> {:ok, recovery, :unchanged}
        _ -> {:error, :workflow_completion_checkpoint_conflict}
      end
    else
      _ -> {:error, :workflow_completion_checkpoint_invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] ->
      {:error, :workflow_completion_checkpoint_invalid}
  end

  def read(job, recovery) do
    case Map.fetch(recovery, @key) do
      :error -> :none
      {:ok, record} -> verify(job, recovery, record)
    end
  end

  def matches(job, recovery, result) do
    case read(job, recovery) do
      :none -> :ok
      {:ok, _claim, ^result} -> :ok
      {:ok, _, _} -> {:error, :workflow_completion_checkpoint_conflict}
      error -> error
    end
  end

  defp verify(job, recovery, record) when is_map(record) do
    with true <- Enum.sort(Map.keys(record)) == Enum.sort(@keys),
         true <- record["schema_version"] == @schema,
         true <- record["job_id"] == job.job_id,
         true <- record["job_created_at_us"] == DateTime.to_unix(job.created_at, :microsecond),
         true <- record["envelope_sha256"] == recovery["envelope_sha256"],
         claim <- record["producer_claim"],
         true <- valid_claim?(claim) and Envelope.fenced?(recovery, claim),
         :ok <- valid_parent(job, recovery),
         json when is_binary(json) and byte_size(json) <= @max_bytes <- record["result_json"],
         true <- digest(Map.delete(record, "sha256")) == record["sha256"],
         {:ok, result} when is_map(result) <- Jason.decode(json),
         true <- is_list(Map.get(result, "failed_nodes", [])),
         :ok <- WorkflowJsonBudget.validate(result, "completion checkpoint", @max_nodes) do
      {:ok, claim, result}
    else
      _ -> {:error, :workflow_completion_checkpoint_invalid}
    end
  rescue
    _ in [Jason.EncodeError, Protocol.UndefinedError, ArgumentError] ->
      {:error, :workflow_completion_checkpoint_invalid}
  end

  defp verify(_job, _recovery, _record), do: {:error, :workflow_completion_checkpoint_invalid}

  defp valid_parent(job, recovery) do
    with :ok <- Envelope.verify(recovery),
         true <-
           Envelope.execution_value(recovery, ["orchestration_context", "job_id"]) == job.job_id do
      :ok
    else
      _ -> {:error, :workflow_completion_checkpoint_invalid}
    end
  end

  defp valid_claim?(claim) when is_map(claim) do
    Enum.sort(Map.keys(claim)) == Enum.sort(@claim_keys) and
      is_integer(claim["generation"]) and claim["generation"] > 0 and
      is_integer(claim["attempt"]) and claim["attempt"] > 0 and
      is_binary(claim["owner_session_id"]) and byte_size(claim["owner_session_id"]) in 1..256
  end

  defp valid_claim?(_claim), do: false

  defp digest(value),
    do:
      value
      |> DurableJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)
end
