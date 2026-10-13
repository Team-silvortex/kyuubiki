defmodule KyuubikiWeb.Orchestra.OperatorDispatchJournal do
  @moduledoc "Bounded dispatch observations, never an execution receipt or replay authority."
  use GenServer

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchRetention, as: Retention
  @terminal_states ~w(observed_executed observed_failed observed_blocked not_dispatched)
  @states ["dispatch_boundary_unconfirmed", "outcome_unknown" | @terminal_states]
  @terminal_retention 128
  @evidence_fields ~w(attempt_id task_id task_digest operator_id program_id)

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: Keyword.get(opts, :name, __MODULE__))
  end

  def begin_dispatch(request, endpoint, server \\ __MODULE__)

  def begin_dispatch(%{"params" => %{"task_ir" => %{} = task}} = request, endpoint, server)
      when is_map(endpoint) do
    now = System.system_time(:millisecond)

    record = %{
      "schema_version" => Files.schema(),
      "attempt_id" => :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower),
      "request_id" => request["id"],
      "task_id" => task["task_id"],
      "job_id" => request["params"]["job_id"],
      "task_digest" => identity(task, "integrity", "task_digest"),
      "operator_id" => identity(task, "operator", "id"),
      "program_id" => identity(task, "execution_program", "program_id"),
      "endpoint_fingerprint" => Files.fingerprint(endpoint),
      "state" => "dispatch_boundary_unconfirmed",
      "created_at_ms" => now,
      "updated_at_ms" => now
    }

    if Files.valid?(record) do
      # Never wait for storage inside the journal: lease holders may need this server.
      with {:ok, candidates} <- call(server, :retention_candidates),
           {:ok, releasable} <- Retention.releasable(candidates) do
        call(server, {:begin, record, releasable})
      end
    else
      {:error, :operator_task_dispatch_journal_invalid}
    end
  end

  def begin_dispatch(_request, _endpoint, _server),
    do: {:error, :operator_task_dispatch_journal_invalid}

  def finish_dispatch(attempt_id, status, server \\ __MODULE__),
    do: call(server, {:finish, attempt_id, status})

  def lookup(task_id, task_digest, server \\ __MODULE__),
    do: call(server, {:lookup, task_id, task_digest})

  def lookup_task(task_id, server \\ __MODULE__),
    do: call(server, {:lookup_task, task_id})

  def verify_observation(record, server \\ __MODULE__) do
    if Files.valid?(record),
      do: call(server, {:verify_observation, record}),
      else: {:error, :operator_task_dispatch_journal_invalid}
  end

  def verify_evidence(job_id, expected, server \\ __MODULE__) do
    if valid_expected?(job_id, expected),
      do: call(server, {:verify_evidence, job_id, expected}),
      else: {:error, :operator_task_dispatch_journal_invalid}
  end

  def workflow_replay_fence(job_id, task_ids, server \\ __MODULE__)
      when is_binary(job_id) and is_list(task_ids),
      do: workflow_replay_fence(job_id, task_ids, [], server)

  def workflow_replay_fence(job_id, task_ids, expected, server)
      when is_binary(job_id) and is_list(task_ids) do
    if valid_expected?(job_id, expected),
      do: call(server, {:workflow_replay_fence, job_id, MapSet.new(task_ids), expected}),
      else: {:error, :operator_task_dispatch_journal_invalid}
  end

  def policy do
    Files.limits()
    |> Map.merge(%{
      terminal_retention: @terminal_retention,
      storage: "operator-task-dispatches/*.json",
      single_writer: true,
      corrupt_generation_policy: "fail_closed_no_rollback",
      state_transition_policy: "unconfirmed_to_one_observation",
      repeated_observation_policy: "verify_without_rewrite",
      invalid_transition_policy: "reject_without_poisoning_other_attempts",
      read_integrity_policy: "verify_inventory_and_selected_records",
      unresolved_retention: "never_automatically_pruned",
      active_parent_retention: "protected_within_existing_capacity",
      retention_store_failure: "reject_dispatch_without_pruning_or_quarantine",
      inputs_results_credentials_stored: false
    })
  end

  def root do
    data_root =
      Application.get_env(:kyuubiki_web, :test_database_root) ||
        Application.get_env(:kyuubiki_web, :operator_dispatch_journal_test_root) ||
        System.get_env("KYUUBIKI_DATA_DIR") ||
        KyuubikiWeb.Persistence.data_dir()

    Path.join(data_root, "operator-task-dispatches")
  end

  @impl true
  def init(opts) do
    root = Keyword.get(opts, :root, root())

    case Files.load(root) do
      {:ok, records} -> {:ok, %{root: root, records: records, error: nil}}
      {:error, error} -> {:ok, %{root: root, records: %{}, error: error}}
    end
  end

  @impl true
  def handle_call(_message, _from, %{error: error} = state) when not is_nil(error),
    do: {:reply, {:error, error}, state}

  def handle_call(:retention_candidates, _from, state) do
    records = Enum.filter(Map.values(state.records), &(&1["state"] in @terminal_states))
    candidates = if length(records) >= @terminal_retention, do: records, else: []
    {:reply, {:ok, candidates}, state}
  end

  def handle_call({:begin, record, releasable}, _from, state) do
    with true <- Files.valid?(record),
         :ok <- Files.verify_inventory(state.root, state.records),
         {:ok, pruned} <- prune(state, releasable) do
      reserve(record, pruned)
    else
      false -> {:reply, {:error, :operator_task_dispatch_journal_invalid}, state}
      {:error, error} -> {:reply, {:error, error}, %{state | error: error}}
    end
  end

  def handle_call({:finish, _id, status}, _from, state) when status not in @states,
    do: {:reply, {:error, :operator_task_dispatch_state_invalid}, state}

  def handle_call({:finish, id, status}, _from, state) do
    case Map.fetch(state.records, id) do
      {:ok, record} ->
        finish(record, status, state)

      :error ->
        {:reply, {:error, :operator_task_dispatch_attempt_missing}, state}
    end
  end

  def handle_call({:lookup, task_id, digest}, _from, state) do
    records =
      state.records
      |> Map.values()
      |> Enum.filter(&(&1["task_id"] == task_id and &1["task_digest"] == digest))
      |> Enum.sort_by(&{&1["created_at_ms"], &1["attempt_id"]}, :desc)

    verified_reply(records, {:ok, records}, state)
  end

  def handle_call({:lookup_task, task_id}, _from, state) do
    records =
      state.records
      |> Map.values()
      |> Enum.filter(&(&1["task_id"] == task_id))
      |> Enum.sort_by(&{&1["created_at_ms"], &1["attempt_id"]}, :desc)

    verified_reply(records, {:ok, records}, state)
  end

  def handle_call({:workflow_replay_fence, job_id, task_ids, expected}, _from, state) do
    relevant =
      state.records
      |> Map.values()
      |> Enum.filter(fn record ->
        Files.job_id(record) == job_id or
          (is_nil(Files.job_id(record)) and MapSet.member?(task_ids, record["task_id"]))
      end)

    unresolved? =
      Enum.any?(relevant, &(&1["state"] in ~w(dispatch_boundary_unconfirmed outcome_unknown)))

    # Legacy task names can deny a replay, never authorize cancellation or execution.
    reply =
      cond do
        evidence_missing?(state, job_id, expected) ->
          {:error, :workflow_dispatch_evidence_missing}

        unresolved? ->
          {:error, :workflow_dispatch_outcome_unconfirmed}

        true ->
          :ok
      end

    verified_reply(relevant, reply, state)
  end

  def handle_call({:verify_evidence, job_id, expected}, _from, state) do
    ids = MapSet.new(expected, & &1["attempt_id"])
    records = Enum.filter(Map.values(state.records), &MapSet.member?(ids, &1["attempt_id"]))

    reply =
      if evidence_missing?(state, job_id, expected),
        do: {:error, :workflow_dispatch_evidence_missing},
        else: :ok

    verified_reply(records, reply, state)
  end

  def handle_call({:verify_observation, expected}, _from, state) do
    current = state.records[expected["attempt_id"]]

    reply =
      if current == expected,
        do: :ok,
        else: {:error, :operator_task_dispatch_observation_changed}

    # A legitimate transition/eviction rejects this read, not the whole journal.
    verified_reply(if(current, do: [current], else: []), reply, state)
  end

  defp evidence_missing?(state, job_id, expected) do
    Enum.any?(expected, fn identity ->
      case state.records[identity["attempt_id"]] do
        %{} = record ->
          Files.job_id(record) != job_id or Map.take(record, Map.keys(identity)) != identity

        _ ->
          true
      end
    end)
  end

  defp valid_expected?(job_id, expected) do
    is_binary(job_id) and is_list(expected) and length(expected) <= Files.limits().max_records and
      Enum.all?(expected, fn entry ->
        is_map(entry) and Enum.sort(Map.keys(entry)) == Enum.sort(@evidence_fields) and
          Enum.all?(entry, fn {_key, value} ->
            is_binary(value) and byte_size(value) in 1..1024
          end) and
          Regex.match?(~r/\A[0-9a-f]{32}\z/, entry["attempt_id"]) and
          Regex.match?(~r/\A[0-9a-f]{64}\z/, entry["task_digest"])
      end)
  end

  defp verified_reply(records, reply, state) do
    case Files.verify_selection(state.root, state.records, records) do
      :ok -> {:reply, reply, state}
      {:error, error} -> {:reply, {:error, error}, %{state | error: error}}
    end
  end

  defp finish(%{"state" => status} = record, status, state) do
    # A no-op still needs durable evidence, not just a surviving in-memory record.
    case Files.verify(state.root, record) do
      :ok -> {:reply, :ok, state}
      {:error, error} -> {:reply, {:error, error}, %{state | error: error}}
    end
  end

  defp finish(%{"state" => "dispatch_boundary_unconfirmed"} = record, status, state) do
    updated =
      record
      |> Map.put("state", status)
      |> Map.put("updated_at_ms", max(record["updated_at_ms"], System.system_time(:millisecond)))

    case Files.write(state.root, updated, record) do
      :ok ->
        {:reply, :ok, %{state | records: Map.put(state.records, record["attempt_id"], updated)}}

      {:error, error} ->
        {:reply, {:error, error}, %{state | error: error}}
    end
  end

  defp finish(_record, _status, state),
    do: {:reply, {:error, :operator_task_dispatch_transition_rejected}, state}

  defp reserve(record, state) do
    if map_size(state.records) < Files.limits().max_records do
      case Files.write(state.root, record) do
        :ok ->
          {:reply, {:ok, record["attempt_id"]},
           %{state | records: Map.put(state.records, record["attempt_id"], record)}}

        {:error, error} ->
          {:reply, {:error, error}, %{state | error: error}}
      end
    else
      {:reply, {:error, :operator_task_dispatch_journal_full}, state}
    end
  end

  defp prune(state, releasable) do
    expired =
      releasable
      # Concurrent reservations/observations cannot inherit an older pruning decision.
      |> Enum.filter(&(state.records[&1["attempt_id"]] == &1 and &1["state"] in @terminal_states))
      |> Enum.sort_by(&{&1["updated_at_ms"], &1["attempt_id"]}, :desc)
      |> Enum.drop(@terminal_retention - 1)

    Enum.reduce_while(expired, {:ok, state}, fn record, {:ok, current} ->
      case Files.delete(state.root, record) do
        :ok ->
          {:cont, {:ok, %{current | records: Map.delete(current.records, record["attempt_id"])}}}

        error ->
          {:halt, error}
      end
    end)
  end

  defp call(server, message) do
    GenServer.call(server, message)
  catch
    :exit, _reason -> {:error, :operator_task_dispatch_journal_unavailable}
  end

  defp identity(task, section, field) do
    case task[section] do
      %{} = fields -> fields[field]
      _ -> nil
    end
  end
end
