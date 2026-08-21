defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope do
  @moduledoc """
  Durable, digest-bound execution context used to recover asynchronous workflows.

  Recovery is deliberately conservative: the first dispatch is always allowed, while a
  replay after Orchestra process loss requires an idempotent graph or a verified checkpoint.
  """

  @schema_version "kyuubiki.workflow-recovery/v1"
  @envelope_schema "kyuubiki.workflow-execution-envelope/v1"
  @checkpoint_contract "kyuubiki.operator_task_batch_checkpoint_verification/v1"
  @internal_key "_workflow_recovery"
  @pure_export_operators [
    "export.summary_json",
    "export.summary_csv",
    "export.alert_markdown",
    "export.diagnostics_bundle_markdown"
  ]
  @active_states ["pending", "running"]
  @terminal_states ["completed", "failed", "cancelled"]

  @spec internal_key() :: String.t()
  def internal_key, do: @internal_key

  @spec new(map(), map(), map(), map()) :: {:ok, map()} | {:error, term()}
  def new(graph, input_artifacts, orchestration_context, response_options)
      when is_map(graph) and is_map(input_artifacts) and is_map(orchestration_context) and
             is_map(response_options) do
    envelope = %{
      "schema_version" => @envelope_schema,
      "graph" => graph,
      "input_artifacts" => input_artifacts,
      "orchestration_context" => orchestration_context,
      "response_options" => response_options
    }

    {retry_safety, checkpoint} = recovery_policy(graph)
    now = timestamp()

    {:ok,
     %{
       "schema_version" => @schema_version,
       "digest_algorithm" => "sha256",
       "envelope_sha256" => digest(envelope),
       "envelope" => envelope,
       "workflow_id" => Map.get(graph, "id"),
       "retry_safety" => retry_safety,
       "checkpoint" => checkpoint,
       "checkpoint_digest" => checkpoint_digest(checkpoint),
       "state" => "pending",
       "generation" => 0,
       "attempt" => 0,
       "owner_session_id" => nil,
       "created_at" => now,
       "updated_at" => now,
       "history" => [event("prepared", 0, 0, now, %{})]
     }}
  end

  def new(_graph, _input_artifacts, _orchestration_context, _response_options),
    do: {:error, :invalid_workflow_recovery_envelope}

  @spec verify(map()) :: :ok | {:error, term()}
  def verify(
        %{
          "schema_version" => @schema_version,
          "digest_algorithm" => "sha256",
          "envelope_sha256" => expected,
          "envelope" => %{"schema_version" => @envelope_schema} = envelope,
          "retry_safety" => retry_safety,
          "state" => state,
          "generation" => generation,
          "attempt" => attempt
        } = recovery
      )
      when is_binary(expected) and
             retry_safety in [
               "idempotent",
               "checkpointed",
               "checkpoint_required"
             ] and is_binary(state) and is_integer(generation) and generation >= 0 and
             is_integer(attempt) and attempt >= 0 do
    {expected_safety, expected_checkpoint} = recovery_policy(Map.get(envelope, "graph", %{}))

    cond do
      not valid_digest?(expected) ->
        {:error, :invalid_workflow_recovery_digest}

      digest(envelope) != expected ->
        {:error, :workflow_recovery_digest_mismatch}

      not valid_envelope?(envelope) ->
        {:error, :invalid_workflow_execution_envelope}

      retry_safety != expected_safety or Map.get(recovery, "checkpoint") != expected_checkpoint ->
        {:error, :workflow_recovery_policy_mismatch}

      Map.get(recovery, "workflow_id") != get_in(envelope, ["graph", "id"]) ->
        {:error, :workflow_recovery_identity_mismatch}

      Map.get(recovery, "checkpoint_digest") != checkpoint_digest(expected_checkpoint) ->
        {:error, :workflow_recovery_checkpoint_mismatch}

      retry_safety == "checkpointed" and not verified_checkpoint?(expected_checkpoint) ->
        {:error, :workflow_recovery_checkpoint_missing_or_invalid}

      true ->
        :ok
    end
  end

  def verify(_recovery), do: {:error, :invalid_workflow_recovery_record}

  @spec claim(map(), String.t(), atom()) :: {:ok, map(), map()} | {:error, term()}
  def claim(recovery, session_id, reason)
      when is_map(recovery) and is_binary(session_id) and byte_size(session_id) > 0 and
             reason in [:initial, :process_restart, :runner_loss] do
    with :ok <- verify(recovery),
         :ok <- claimable_state(recovery),
         :ok <- replay_allowed(recovery, reason) do
      generation = recovery["generation"] + 1
      attempt = recovery["attempt"] + 1
      now = timestamp()

      claimed =
        recovery
        |> Map.put("state", "running")
        |> Map.put("generation", generation)
        |> Map.put("attempt", attempt)
        |> Map.put("owner_session_id", session_id)
        |> Map.put("updated_at", now)
        |> append_event(
          event("claimed", generation, attempt, now, %{
            "reason" => Atom.to_string(reason),
            "session_id" => session_id
          })
        )

      claim = %{
        "generation" => generation,
        "owner_session_id" => session_id,
        "attempt" => attempt
      }

      {:ok, claimed, claim}
    end
  end

  @spec fenced?(map(), map()) :: boolean()
  def fenced?(recovery, claim) when is_map(recovery) and is_map(claim) do
    recovery["state"] == "running" and
      recovery["generation"] == claim["generation"] and
      recovery["owner_session_id"] == claim["owner_session_id"]
  end

  def fenced?(_recovery, _claim), do: false

  @spec replayable?(map()) :: boolean()
  def replayable?(%{"retry_safety" => "idempotent"}), do: true

  def replayable?(%{"retry_safety" => "checkpointed", "checkpoint" => checkpoint}),
    do: verified_checkpoint?(checkpoint)

  def replayable?(_recovery), do: false

  @spec transition(map(), String.t(), map()) :: map()
  def transition(recovery, state, details \\ %{})
      when is_map(recovery) and is_binary(state) and is_map(details) do
    now = timestamp()

    recovery
    |> Map.put("state", state)
    |> Map.put("updated_at", now)
    |> append_event(
      event(
        state,
        Map.get(recovery, "generation", 0),
        Map.get(recovery, "attempt", 0),
        now,
        details
      )
    )
    |> maybe_prune_terminal_envelope(state)
  end

  @spec public_result(map()) :: map()
  def public_result(result) when is_map(result) do
    case Map.pop(result, @internal_key) do
      {recovery, public} when is_map(recovery) ->
        Map.put(public, "recovery", public_summary(recovery))

      {_missing, public} ->
        public
    end
  end

  @spec public_summary(map()) :: map()
  def public_summary(recovery) when is_map(recovery) do
    %{
      "schema_version" => Map.get(recovery, "schema_version"),
      "workflow_id" => Map.get(recovery, "workflow_id"),
      "state" => Map.get(recovery, "state"),
      "retry_safety" => Map.get(recovery, "retry_safety"),
      "checkpoint_digest" => Map.get(recovery, "checkpoint_digest"),
      "generation" => Map.get(recovery, "generation", 0),
      "attempt" => Map.get(recovery, "attempt", 0),
      "envelope_sha256" => Map.get(recovery, "envelope_sha256"),
      "envelope_retained" => is_map(Map.get(recovery, "envelope")),
      "updated_at" => Map.get(recovery, "updated_at"),
      "history" => public_history(Map.get(recovery, "history", []))
    }
  end

  defp valid_envelope?(envelope) do
    is_map(Map.get(envelope, "graph")) and
      is_map(Map.get(envelope, "input_artifacts")) and
      is_map(Map.get(envelope, "orchestration_context")) and
      is_map(Map.get(envelope, "response_options"))
  end

  defp claimable_state(%{"state" => state}) when state in @active_states, do: :ok
  defp claimable_state(%{"state" => state}), do: {:error, {:workflow_recovery_not_active, state}}
  defp claimable_state(_recovery), do: {:error, :invalid_workflow_recovery_state}

  defp replay_allowed(_recovery, :initial), do: :ok

  defp replay_allowed(recovery, _reason) do
    if replayable?(recovery),
      do: :ok,
      else: {:error, {:workflow_replay_blocked, Map.get(recovery, "retry_safety")}}
  end

  defp recovery_policy(graph) do
    policy = Map.get(graph, "recovery_policy", %{})
    checkpoint = Map.get(policy, "checkpoint")

    retry_safety =
      case Map.get(policy, "retry_safety") do
        value when value in ["idempotent", "checkpointed", "checkpoint_required"] -> value
        _ -> infer_retry_safety(Map.get(graph, "nodes", []))
      end

    {retry_safety, checkpoint}
  end

  defp infer_retry_safety(nodes) when is_list(nodes) do
    nodes
    |> Enum.map(&node_retry_safety/1)
    |> Enum.reduce("idempotent", &stricter_retry_safety/2)
  end

  defp infer_retry_safety(_nodes), do: "checkpoint_required"

  defp node_retry_safety(node) when is_map(node) do
    explicit = Map.get(node, "retry_safety") || Map.get(node, "replay_safety")

    cond do
      explicit in ["idempotent", "checkpointed", "checkpoint_required"] ->
        explicit

      Map.get(node, "kind") == "export" ->
        export_retry_safety(Map.get(node, "operator_id"))

      Map.get(node, "kind") in ["input", "solve", "transform", "extract", "condition", "output"] ->
        "idempotent"

      true ->
        "checkpoint_required"
    end
  end

  defp node_retry_safety(_node), do: "checkpoint_required"

  defp export_retry_safety(operator_id) when operator_id in @pure_export_operators,
    do: "idempotent"

  defp export_retry_safety(_operator_id), do: "checkpoint_required"

  defp stricter_retry_safety("checkpoint_required", _current), do: "checkpoint_required"
  defp stricter_retry_safety(_next, "checkpoint_required"), do: "checkpoint_required"
  defp stricter_retry_safety("checkpointed", _current), do: "checkpointed"
  defp stricter_retry_safety(_next, "checkpointed"), do: "checkpointed"
  defp stricter_retry_safety(_next, _current), do: "idempotent"

  defp verified_checkpoint?(%{
         "operator_task_batch_checkpoint_verification_contract" => @checkpoint_contract,
         "status" => "verified",
         "checkpoint_digest" => digest
       })
       when is_binary(digest),
       do: valid_digest?(digest)

  defp verified_checkpoint?(_checkpoint), do: false

  defp checkpoint_digest(checkpoint) do
    if verified_checkpoint?(checkpoint), do: Map.get(checkpoint, "checkpoint_digest"), else: nil
  end

  defp maybe_prune_terminal_envelope(recovery, state) when state in @terminal_states,
    do: Map.delete(recovery, "envelope")

  defp maybe_prune_terminal_envelope(recovery, _state), do: recovery

  defp append_event(recovery, entry) do
    Map.update(recovery, "history", [entry], fn history ->
      (List.wrap(history) ++ [entry]) |> Enum.take(-16)
    end)
  end

  defp public_history(history) when is_list(history) do
    Enum.flat_map(history, fn
      entry when is_map(entry) -> [Map.drop(entry, ["session_id"])]
      _entry -> []
    end)
  end

  defp public_history(_history), do: []

  defp event(kind, generation, attempt, at, details) do
    %{
      "event" => kind,
      "generation" => generation,
      "attempt" => attempt,
      "at" => at
    }
    |> Map.merge(details)
  end

  defp digest(value) do
    value
    |> canonical_json_value()
    |> Jason.encode!()
    |> then(&:crypto.hash(:sha256, &1))
    |> Base.encode16(case: :lower)
  end

  defp canonical_json_value(value) when is_map(value) do
    value
    |> Enum.map(fn {key, item} -> {to_string(key), canonical_json_value(item)} end)
    |> Enum.sort_by(&elem(&1, 0))
    |> Jason.OrderedObject.new()
  end

  defp canonical_json_value(value) when is_list(value),
    do: Enum.map(value, &canonical_json_value/1)

  defp canonical_json_value(value), do: value

  defp valid_digest?(digest), do: Regex.match?(~r/\A[0-9a-f]{64}\z/, digest)
  defp timestamp, do: DateTime.utc_now(:second) |> DateTime.to_iso8601()
end
