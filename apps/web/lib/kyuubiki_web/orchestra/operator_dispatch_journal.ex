defmodule KyuubikiWeb.Orchestra.OperatorDispatchJournal do
  @moduledoc "Bounded dispatch observations, never an execution receipt or replay authority."
  use GenServer

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  @terminal_states ~w(observed_executed observed_failed observed_blocked not_dispatched)
  @terminal_retention 128

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
      "task_digest" => identity(task, "integrity", "task_digest"),
      "operator_id" => identity(task, "operator", "id"),
      "program_id" => identity(task, "execution_program", "program_id"),
      "endpoint_fingerprint" => Files.fingerprint(endpoint),
      "state" => "dispatch_boundary_unconfirmed",
      "created_at_ms" => now,
      "updated_at_ms" => now
    }

    call(server, {:begin, record})
  end

  def begin_dispatch(_request, _endpoint, _server),
    do: {:error, :operator_task_dispatch_journal_invalid}

  def finish_dispatch(attempt_id, status, server \\ __MODULE__),
    do: call(server, {:finish, attempt_id, status})

  def lookup(task_id, task_digest, server \\ __MODULE__),
    do: call(server, {:lookup, task_id, task_digest})

  def policy do
    Files.limits()
    |> Map.merge(%{
      terminal_retention: @terminal_retention,
      storage: "operator-task-dispatches/*.json",
      single_writer: true,
      corrupt_generation_policy: "fail_closed_no_rollback",
      unresolved_retention: "never_automatically_pruned",
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

  def handle_call({:begin, record}, _from, state) do
    with true <- Files.valid?(record),
         {:ok, pruned} <- prune(state) do
      reserve(record, pruned)
    else
      false -> {:reply, {:error, :operator_task_dispatch_journal_invalid}, state}
      {:error, error} -> {:reply, {:error, error}, %{state | error: error}}
    end
  end

  def handle_call({:finish, id, status}, _from, state) do
    case Map.fetch(state.records, id) do
      {:ok, record} ->
        updated =
          record
          |> Map.put("state", status)
          |> Map.put(
            "updated_at_ms",
            max(record["created_at_ms"], System.system_time(:millisecond))
          )

        case Files.write(state.root, updated, record) do
          :ok -> {:reply, :ok, %{state | records: Map.put(state.records, id, updated)}}
          {:error, error} -> {:reply, {:error, error}, %{state | error: error}}
        end

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

    {:reply, {:ok, records}, state}
  end

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

  defp prune(state) do
    expired =
      state.records
      |> Map.values()
      |> Enum.filter(&(&1["state"] in @terminal_states))
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
