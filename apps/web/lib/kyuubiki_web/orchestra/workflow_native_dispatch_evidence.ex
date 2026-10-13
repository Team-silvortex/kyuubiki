defmodule KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence do
  @moduledoc "Bounded pre-RPC attempt identities in the existing private workflow record."

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.WorkflowNativeOperatorRuntime, as: Native

  @key "native_dispatch_evidence"
  @schema "kyuubiki.workflow-native-dispatch-evidence/v1"
  @max_entries 512
  @max_bytes 524_288
  @identity ~w(task_id task_digest operator_id program_id)
  @claim ~w(generation attempt owner_session_id)
  @bucket ~w(schema_version job_id envelope_sha256 entries sha256)
  @entry @identity ++ ~w(attempt_id producer_claim)

  def key, do: @key

  def policy do
    %{
      "schema_version" => @schema,
      "mode" => "persist_attempt_identity_before_rpc",
      "max_entries" => @max_entries,
      "max_bytes" => @max_bytes,
      "missing_evidence_policy" => "block_without_replay",
      "inputs_results_credentials_stored" => false
    }
  end

  def initialize(recovery, context) do
    Map.put(
      recovery,
      @key,
      seal(%{
        "schema_version" => @schema,
        "job_id" => context["job_id"],
        "envelope_sha256" => recovery["envelope_sha256"],
        "entries" => %{}
      })
    )
  end

  def check(summary, opts) do
    case Keyword.get(opts, :native_dispatch_evidence) do
      nil ->
        :ok

      callback when is_function(callback, 1) ->
        case callback.({:lookup, Map.take(summary, @identity)}) do
          :ok -> :ok
          {:error, :input_changed} -> blocked(:input_changed)
          {:error, :workflow_dispatch_evidence_missing} -> blocked(:dispatch_evidence_missing)
          {:error, :workflow_dispatch_outcome_unconfirmed} -> blocked(:dispatch_unconfirmed)
          {:error, :invalid} -> blocked(:dispatch_evidence_invalid)
          _ -> blocked(:journal_unavailable)
        end

      _ ->
        blocked(:dispatch_evidence_invalid)
    end
  end

  def dispatch_options(summary, opts) do
    case Keyword.get(opts, :native_dispatch_evidence) do
      callback when is_function(callback, 1) ->
        [
          on_dispatch_reserved: fn attempt ->
            callback.({:commit, Map.take(summary, @identity), attempt})
          end
        ]

      _ ->
        []
    end
  end

  def verify(recovery, job_id) do
    with {:ok, execution} <- Envelope.execution(recovery),
         :ok <- verify_bucket(recovery[@key], recovery, job_id, execution),
         do: :ok
  end

  def expected(recovery) do
    recovery[@key]["entries"]
    |> Map.values()
    |> Enum.map(&Map.take(&1, ["attempt_id" | @identity]))
  end

  def lookup(recovery, request, claim) do
    with {:ok, execution} <- Envelope.execution(recovery),
         true <- Envelope.fenced?(recovery, claim),
         true <- valid_request?(request, execution),
         :ok <- verify_bucket(recovery[@key], recovery, parent(execution), execution) do
      entries = Enum.filter(expected(recovery), &(&1["task_id"] == request["task_id"]))

      if Enum.all?(entries, &(Map.take(&1, @identity) == request)),
        do:
          Journal.workflow_replay_fence(parent(execution), [request["task_id"]], entries, Journal),
        else: {:error, :input_changed}
    else
      _ -> {:error, :invalid}
    end
  end

  def put(recovery, request, attempt, claim) do
    with {:ok, execution} <- Envelope.execution(recovery),
         true <- valid_claim?(claim) and Envelope.fenced?(recovery, claim),
         true <- valid_request?(request, execution) and hex?(attempt, 32),
         :ok <- verify_bucket(recovery[@key], recovery, parent(execution), execution) do
      entry = Map.merge(request, %{"attempt_id" => attempt, "producer_claim" => claim})
      bucket = recovery[@key]

      case Map.fetch(bucket["entries"], attempt) do
        {:ok, ^entry} ->
          with :ok <- Journal.verify_evidence(parent(execution), expected(recovery)),
               do: {:ok, recovery, :unchanged}

        {:ok, _} ->
          {:error, :conflict}

        :error ->
          retain(recovery, bucket, entry)
      end
    else
      {:error, _} = error -> error
      _ -> {:error, :invalid}
    end
  end

  defp retain(recovery, bucket, entry) do
    if map_size(bucket["entries"]) >= @max_entries do
      {:error, :full}
    else
      updated = put_in(bucket, ["entries", entry["attempt_id"]], entry) |> seal()

      if byte_size(DurableJson.encode!(updated)) <= @max_bytes do
        retained = Map.put(recovery, @key, updated)

        with :ok <- Journal.verify_evidence(bucket["job_id"], expected(retained)),
             do: {:ok, retained, :retained}
      else
        {:error, :full}
      end
    end
  end

  defp verify_bucket(bucket, recovery, job_id, execution) when is_map(bucket) do
    with true <- closed?(bucket, @bucket),
         true <- bucket["schema_version"] == @schema,
         true <- is_binary(job_id) and byte_size(job_id) in 1..1024,
         true <- bucket["job_id"] == job_id and job_id == parent(execution),
         true <- bucket["envelope_sha256"] == recovery["envelope_sha256"],
         entries when is_map(entries) <- bucket["entries"],
         true <- map_size(entries) <= @max_entries,
         allowed <- native_identities(execution),
         true <- Enum.all?(entries, &valid_entry?(&1, recovery, allowed)),
         true <- hex?(bucket["sha256"], 64),
         true <- byte_size(DurableJson.encode!(bucket)) <= @max_bytes,
         true <- bucket["sha256"] == digest(Map.delete(bucket, "sha256")) do
      :ok
    else
      _ -> {:error, :invalid}
    end
  end

  defp verify_bucket(_, _, _, _), do: {:error, :invalid}

  defp valid_entry?({id, entry}, recovery, allowed) when is_map(entry) do
    closed?(entry, @entry) and hex?(id, 32) and entry["attempt_id"] == id and
      valid_identity?(Map.take(entry, @identity), allowed) and
      valid_claim?(entry["producer_claim"]) and
      entry["producer_claim"]["generation"] <= recovery["generation"] and
      entry["producer_claim"]["attempt"] <= recovery["attempt"] and
      (entry["producer_claim"]["generation"] != recovery["generation"] or
         entry["producer_claim"]["owner_session_id"] == recovery["owner_session_id"])
  end

  defp valid_entry?(_, _, _), do: false

  defp valid_identity?(request, allowed) when is_map(request) do
    closed?(request, @identity) and hex?(request["task_digest"], 64) and
      Enum.all?(~w(task_id operator_id program_id), fn key ->
        is_binary(request[key]) and byte_size(request[key]) in 1..1024
      end) and MapSet.member?(allowed, {request["task_id"], request["operator_id"]})
  end

  defp valid_identity?(_, _), do: false

  defp valid_request?(request, execution),
    do: valid_identity?(request, native_identities(execution))

  defp native_identities(execution) do
    nodes = get_in(execution, ["graph", "nodes"])
    context = execution["orchestration_context"]

    if is_list(nodes) do
      nodes
      |> Enum.filter(&(is_map(&1) and Native.supports?(&1["kind"], &1["operator_id"])))
      |> MapSet.new(&{Native.task_id(context, &1, &1["operator_id"]), &1["operator_id"]})
    else
      MapSet.new()
    end
  end

  defp valid_claim?(claim) when is_map(claim) do
    closed?(claim, @claim) and is_integer(claim["generation"]) and claim["generation"] > 0 and
      is_integer(claim["attempt"]) and claim["attempt"] > 0 and
      is_binary(claim["owner_session_id"]) and byte_size(claim["owner_session_id"]) in 1..256
  end

  defp valid_claim?(_), do: false
  defp parent(execution), do: get_in(execution, ["orchestration_context", "job_id"])
  defp closed?(map, keys), do: Enum.sort(Map.keys(map)) == Enum.sort(keys)
  defp seal(bucket), do: Map.put(bucket, "sha256", digest(Map.delete(bucket, "sha256")))

  defp digest(value),
    do: :crypto.hash(:sha256, DurableJson.encode!(value)) |> Base.encode16(case: :lower)

  defp hex?(value, size),
    do: is_binary(value) and byte_size(value) == size and Regex.match?(~r/\A[0-9a-f]+\z/, value)

  defp blocked(reason), do: {:error, {:native_workflow_recovery_blocked, reason}}
end
