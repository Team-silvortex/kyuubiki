defmodule KyuubikiWeb.Orchestra.WorkflowCancellationIntent do
  @moduledoc "Bounded private workflow expiry cancellation intent; never an execution receipt."
  alias KyuubikiWeb.Playground.{AgentDispatchCancellation, AgentExecutionCancellation}
  alias KyuubikiWeb.Storage.DurableJson

  @key "cancellation_intent"
  @schema "kyuubiki.workflow-cancellation-intent/v1"
  @fields ~w(schema_version intent_id job_id job_created_at envelope_sha256 generation owner_session_id entries sha256)
  @entry ~w(request_id endpoint_fingerprint execution_target delivery_id attempts status reason)
  @states ~w(pending dispatching requested target_not_observed exhausted)
  @reasons ~w(original_endpoint_not_configured original_endpoint_unreachable request_not_observed_active agent_observation_invalid execution_target_invalid cancellation_outcome_unknown agent_acknowledgement_invalid retry_budget_exhausted)
  @max_bytes 262_144
  @max_attempts 3

  def key, do: @key
  def max_attempts, do: @max_attempts

  def policy,
    do: %{
      schema_version: @schema,
      max_entries: 128,
      max_bytes: @max_bytes,
      max_attempts: @max_attempts,
      credentials_stored: false,
      execution_terminal_confirmed: false
    }

  def attach(recovery, job, snapshot) do
    with :ok <- AgentDispatchCancellation.validate_snapshot(job.job_id, snapshot),
         false <- Map.has_key?(recovery, @key) do
      if snapshot.requests == [] do
        {:ok, recovery}
      else
        entries =
          Enum.map(snapshot.requests, fn request ->
            %{
              "request_id" => request.request_id,
              "endpoint_fingerprint" =>
                AgentExecutionCancellation.endpoint_fingerprint(request.endpoint),
              "execution_target" => nil,
              "delivery_id" => nil,
              "attempts" => 0,
              "status" => "pending",
              "reason" => nil
            }
          end)

        bucket =
          %{
            "schema_version" => @schema,
            "intent_id" => nonce(),
            "job_id" => job.job_id,
            "job_created_at" => DateTime.to_iso8601(job.created_at),
            "envelope_sha256" => recovery["envelope_sha256"],
            "generation" => recovery["generation"],
            "owner_session_id" => recovery["owner_session_id"],
            "entries" => entries
          }
          |> seal()

        updated = Map.put(recovery, @key, bucket)
        with :ok <- verify(updated, job), do: {:ok, updated}
      end
    else
      _ -> {:error, :invalid_workflow_cancellation_intent}
    end
  end

  def verify(recovery, job) when is_map(recovery) do
    bucket = recovery[@key]

    with true <- is_map(bucket) and closed?(bucket, @fields),
         true <- bucket["schema_version"] == @schema and hex?(bucket["intent_id"], 32),
         true <-
           bucket["job_id"] == job.job_id and
             AgentExecutionCancellation.valid_identity?(job.job_id),
         true <- bucket["job_created_at"] == DateTime.to_iso8601(job.created_at),
         true <-
           hex?(bucket["envelope_sha256"], 64) and
             bucket["envelope_sha256"] == recovery["envelope_sha256"],
         true <-
           is_integer(bucket["generation"]) and bucket["generation"] >= 0 and
             bucket["generation"] == recovery["generation"],
         true <- bucket["owner_session_id"] == recovery["owner_session_id"],
         true <-
           is_nil(bucket["owner_session_id"]) or
             AgentExecutionCancellation.valid_identity?(bucket["owner_session_id"]),
         entries when is_list(entries) <- bucket["entries"],
         true <- length(entries) in 1..128 and Enum.all?(entries, &valid_entry?(&1, job.job_id)),
         true <- MapSet.size(MapSet.new(entries, &identity/1)) == length(entries),
         true <- MapSet.size(MapSet.new(entries, & &1["endpoint_fingerprint"])) <= 32,
         true <- hex?(bucket["sha256"], 64),
         true <- byte_size(DurableJson.encode!(bucket)) <= @max_bytes,
         true <- seal(bucket)["sha256"] == bucket["sha256"] do
      :ok
    else
      _ -> {:error, :invalid_workflow_cancellation_intent}
    end
  end

  def verify(_, _), do: {:error, :invalid_workflow_cancellation_intent}

  def pending?(%{"entries" => entries}) when is_list(entries) and length(entries) <= 128,
    do: Enum.any?(entries, &(is_map(&1) and &1["status"] in ~w(pending dispatching)))

  def pending?(_), do: false

  def reserve(bucket, id) do
    change(bucket, id, fn entry ->
      cond do
        entry["status"] not in ~w(pending dispatching) ->
          {:error, :cancellation_already_settled}

        entry["attempts"] >= @max_attempts ->
          {:ok,
           Map.merge(entry, %{"status" => "exhausted", "reason" => "retry_budget_exhausted"})}

        true ->
          {:ok,
           Map.merge(entry, %{
             "status" => "dispatching",
             "attempts" => entry["attempts"] + 1,
             "delivery_id" => nonce(),
             "reason" => nil
           })}
      end
    end)
  end

  def bind(bucket, token, target) do
    change_owned(bucket, token, fn entry ->
      if AgentExecutionCancellation.valid_target?(target) and target["job_id"] == bucket["job_id"] and
           target["request_id"] == entry["request_id"] and
           entry["execution_target"] in [nil, target],
         do: {:ok, Map.put(entry, "execution_target", target)},
         else: {:error, :cancellation_target_changed}
    end)
  end

  def finish(bucket, token, outcome) do
    change_owned(bucket, token, fn entry ->
      cond do
        outcome in ~w(requested target_not_observed) and is_map(entry["execution_target"]) ->
          {:ok, Map.merge(entry, %{"status" => outcome, "reason" => nil})}

        outcome in @reasons ->
          status = if entry["attempts"] >= @max_attempts, do: "exhausted", else: "pending"
          {:ok, Map.merge(entry, %{"status" => status, "reason" => outcome})}

        true ->
          {:error, :invalid_cancellation_outcome}
      end
    end)
  end

  def owned_entry(bucket, %{id: id, delivery_id: delivery_id}) do
    case Enum.find(bucket["entries"], &(identity(&1) == id)) do
      %{"status" => "dispatching", "delivery_id" => delivery} = entry
      when delivery == delivery_id ->
        {:ok, entry}

      _ ->
        {:error, :stale_cancellation_delivery}
    end
  end

  def owned_entry(_, _), do: {:error, :stale_cancellation_delivery}

  def identity(entry), do: {entry["endpoint_fingerprint"], entry["request_id"]}

  defp change_owned(bucket, token, update) do
    with {:ok, entry} <- owned_entry(bucket, token), do: change(bucket, identity(entry), update)
  end

  defp change(bucket, id, update) do
    case Enum.find(bucket["entries"], &(identity(&1) == id)) do
      nil ->
        {:error, :cancellation_request_not_found}

      entry ->
        with {:ok, next} <- update.(entry) do
          entries = Enum.map(bucket["entries"], &if(identity(&1) == id, do: next, else: &1))
          {:ok, bucket |> Map.put("entries", entries) |> seal(), next}
        end
    end
  end

  def summary(recovery) do
    case recovery[@key] do
      %{"entries" => entries} = bucket when is_list(entries) and length(entries) <= 128 ->
        counts =
          Enum.frequencies_by(entries, fn entry ->
            if is_map(entry) and entry["status"] in @states, do: entry["status"], else: "invalid"
          end)

        status =
          cond do
            entries == [] or Map.has_key?(counts, "invalid") -> "invalid"
            pending?(bucket) -> "pending"
            Map.has_key?(counts, "exhausted") -> "unresolved"
            true -> "acknowledged"
          end

        %{
          "status" => status,
          "counts" => counts,
          "execution_terminal_confirmed" => false,
          "policy" => policy()
        }

      nil ->
        nil

      _ ->
        %{"status" => "invalid", "execution_terminal_confirmed" => false, "policy" => policy()}
    end
  end

  defp valid_entry?(entry, job_id) when is_map(entry) do
    closed?(entry, @entry) and AgentExecutionCancellation.valid_identity?(entry["request_id"]) and
      hex?(entry["endpoint_fingerprint"], 64) and entry["status"] in @states and
      is_integer(entry["attempts"]) and entry["attempts"] in 0..@max_attempts and
      (is_nil(entry["reason"]) or entry["reason"] in @reasons) and
      valid_delivery?(entry) and valid_target?(entry, job_id)
  end

  defp valid_entry?(_, _), do: false

  defp valid_delivery?(%{"attempts" => 0} = entry),
    do:
      is_nil(entry["delivery_id"]) and entry["status"] == "pending" and
        is_nil(entry["execution_target"])

  defp valid_delivery?(entry), do: hex?(entry["delivery_id"], 32)

  defp valid_target?(%{"execution_target" => nil, "status" => state}, _),
    do: state not in ~w(requested target_not_observed)

  defp valid_target?(entry, job_id) do
    target = entry["execution_target"]

    AgentExecutionCancellation.valid_target?(target) and target["job_id"] == job_id and
      target["request_id"] == entry["request_id"]
  end

  defp seal(value),
    do:
      Map.put(
        value,
        "sha256",
        :crypto.hash(:sha256, DurableJson.encode!(Map.delete(value, "sha256")))
        |> Base.encode16(case: :lower)
      )

  defp closed?(value, keys), do: Enum.sort(Map.keys(value)) == Enum.sort(keys)

  defp hex?(value, size),
    do: is_binary(value) and byte_size(value) == size and Regex.match?(~r/\A[0-9a-f]+\z/, value)

  defp nonce, do: :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)
end
