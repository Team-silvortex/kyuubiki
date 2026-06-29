defmodule KyuubikiWeb.Jobs.Watchdog do
  @moduledoc """
  Periodically scans active jobs and marks stale or timed-out jobs as failed so they do not
  remain stuck forever.
  """

  use GenServer

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentRegistry

  @active_statuses [:queued, :preprocessing, :partitioning, :solving, :postprocessing]
  @state_rank %{"unknown" => 0, "healthy" => 1, "watch" => 2, "critical" => 3}

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  def scan_now do
    GenServer.call(__MODULE__, :scan_now, 30_000)
  end

  def status_snapshot do
    state = config()
    jobs = Store.list()
    agent_runtime = safe_agent_runtime()
    orchestra_load = summarize_orchestra_load(jobs, state)
    agent_load = summarize_agent_load(agent_runtime, state)
    operator_load = summarize_operator_load(jobs, state)

    %{
      scan_interval_ms: state.scan_interval_ms,
      stale_job_ms: state.stale_job_ms,
      job_timeout_ms: state.job_timeout_ms,
      active_jobs: Enum.count(jobs, &(&1.status in @active_statuses)),
      stalled_jobs:
        Enum.count(jobs, fn job ->
          job.status == :failed and is_binary(job.message) and
            String.contains?(job.message, "watchdog marked job stalled")
        end),
      timed_out_jobs:
        Enum.count(jobs, fn job ->
          job.status == :failed and is_binary(job.message) and
            String.contains?(job.message, "watchdog timed out job")
        end),
      watchdog_state: worst_state([orchestra_load.state, agent_load.state, operator_load.state]),
      orchestra_load: orchestra_load,
      agent_load: agent_load,
      operator_load: operator_load
    }
  end

  @impl true
  def init(_opts) do
    state = config()
    schedule_scan(state.scan_interval_ms)
    {:ok, state}
  end

  @impl true
  def handle_call(:scan_now, _from, _state) do
    refreshed = config()
    {:reply, run_scan(refreshed), refreshed}
  end

  @impl true
  def handle_info(:scan, _state) do
    refreshed = config()
    _ = run_scan(refreshed)
    schedule_scan(refreshed.scan_interval_ms)
    {:noreply, refreshed}
  end

  defp config do
    env = Application.get_env(:kyuubiki_web, __MODULE__, [])

    %{
      scan_interval_ms: Keyword.get(env, :scan_interval_ms, 5_000),
      stale_job_ms: Keyword.get(env, :stale_job_ms, 30_000),
      job_timeout_ms: Keyword.get(env, :job_timeout_ms, 120_000),
      orchestra_warn_active_jobs: Keyword.get(env, :orchestra_warn_active_jobs, 25),
      orchestra_critical_active_jobs: Keyword.get(env, :orchestra_critical_active_jobs, 100),
      operator_warn_active_jobs: Keyword.get(env, :operator_warn_active_jobs, 8),
      operator_critical_active_jobs: Keyword.get(env, :operator_critical_active_jobs, 32),
      agent_warn_utilization: Keyword.get(env, :agent_warn_utilization, 0.75),
      agent_critical_utilization: Keyword.get(env, :agent_critical_utilization, 1.0)
    }
  end

  defp schedule_scan(interval_ms) when is_integer(interval_ms) and interval_ms > 0 do
    Process.send_after(self(), :scan, interval_ms)
  end

  defp run_scan(state) do
    now = DateTime.utc_now()

    Store.list()
    |> Enum.reduce(%{timed_out: 0, stalled: 0}, fn job, acc ->
      cond do
        job.status not in @active_statuses ->
          acc

        older_than?(job.created_at, now, state.job_timeout_ms) ->
          mark_failed(job, timeout_message(state.job_timeout_ms))
          %{acc | timed_out: acc.timed_out + 1}

        older_than?(job.updated_at, now, state.stale_job_ms) ->
          mark_failed(job, stall_message(state.stale_job_ms))
          %{acc | stalled: acc.stalled + 1}

        true ->
          acc
      end
    end)
  end

  defp mark_failed(job, message) do
    _ =
      Store.apply_progress(%{
        job_id: job.job_id,
        stage: "failed",
        progress: clamp_progress(job.progress),
        message: message
      })

    :ok
  end

  defp older_than?(%DateTime{} = timestamp, %DateTime{} = now, limit_ms)
       when is_integer(limit_ms) and limit_ms > 0 do
    DateTime.diff(now, timestamp, :millisecond) >= limit_ms
  end

  defp older_than?(_, _, _), do: false

  defp summarize_orchestra_load(jobs, state) do
    now = DateTime.utc_now()
    active_jobs = Enum.filter(jobs, &(&1.status in @active_statuses))

    stale_active_jobs =
      Enum.count(active_jobs, &older_than?(&1.updated_at, now, state.stale_job_ms))

    timed_out_active_jobs =
      Enum.count(active_jobs, &older_than?(&1.created_at, now, state.job_timeout_ms))

    status =
      cond do
        timed_out_active_jobs > 0 or length(active_jobs) >= state.orchestra_critical_active_jobs ->
          "critical"

        stale_active_jobs > 0 or length(active_jobs) >= state.orchestra_warn_active_jobs ->
          "watch"

        true ->
          "healthy"
      end

    %{
      state: status,
      active_jobs: length(active_jobs),
      queued_jobs: Enum.count(active_jobs, &(&1.status == :queued)),
      stale_active_jobs: stale_active_jobs,
      timed_out_active_jobs: timed_out_active_jobs,
      active_by_status: count_by(active_jobs, &Atom.to_string(&1.status)),
      thresholds: %{
        warn_active_jobs: state.orchestra_warn_active_jobs,
        critical_active_jobs: state.orchestra_critical_active_jobs
      }
    }
  end

  defp summarize_agent_load(%{available: false} = runtime, _state) do
    %{
      state: "unknown",
      available: false,
      reason: runtime.reason,
      total_agents: 0,
      active_agents: 0,
      stale_agents: 0,
      capacity_slots: 0,
      leased_slots: 0,
      utilization: 0.0
    }
  end

  defp summarize_agent_load(%{snapshot: snapshot, agents: agents}, state) do
    capacity_slots =
      agents
      |> Enum.filter(&(&1["execution_state"] != "lease_stale"))
      |> Enum.map(&agent_capacity/1)
      |> Enum.sum()

    leased_slots = Map.get(snapshot, :active_execution_lease_count, 0)
    stale_leases = Map.get(snapshot, :stale_execution_lease_count, 0)
    utilization = utilization(leased_slots, capacity_slots)

    status =
      cond do
        stale_leases > 0 or utilization >= state.agent_critical_utilization ->
          "critical"

        Map.get(snapshot, :stale_agents, 0) > 0 or utilization >= state.agent_warn_utilization ->
          "watch"

        true ->
          "healthy"
      end

    %{
      state: status,
      available: true,
      total_agents: Map.get(snapshot, :total_agents, 0),
      active_agents: Map.get(snapshot, :active_agents, 0),
      stale_agents: Map.get(snapshot, :stale_agents, 0),
      stale_after_ms: Map.get(snapshot, :stale_after_ms),
      capacity_slots: capacity_slots,
      leased_slots: leased_slots,
      utilization: utilization,
      stale_execution_lease_count: stale_leases,
      control_modes: Map.get(snapshot, :control_modes, %{}),
      thresholds: %{
        warn_utilization: state.agent_warn_utilization,
        critical_utilization: state.agent_critical_utilization
      }
    }
  end

  defp summarize_operator_load(jobs, state) do
    active_jobs = Enum.filter(jobs, &(&1.status in @active_statuses))
    counts = count_by(active_jobs, &operator_key/1)

    hotspots =
      counts
      |> Enum.map(fn {operator, active_jobs} ->
        %{
          operator: operator,
          active_jobs: active_jobs,
          state: operator_state(active_jobs, state)
        }
      end)
      |> Enum.sort_by(&{-&1.active_jobs, &1.operator})
      |> Enum.take(8)

    status = worst_state(Enum.map(hotspots, & &1.state))

    %{
      state: status,
      active_jobs: length(active_jobs),
      active_operator_count: map_size(counts),
      active_by_operator: counts,
      hotspots: hotspots,
      thresholds: %{
        warn_active_jobs: state.operator_warn_active_jobs,
        critical_active_jobs: state.operator_critical_active_jobs
      }
    }
  end

  defp operator_state(count, state) when count >= state.operator_critical_active_jobs,
    do: "critical"

  defp operator_state(count, state) when count >= state.operator_warn_active_jobs, do: "watch"
  defp operator_state(_count, _state), do: "healthy"

  defp safe_agent_runtime do
    if Process.whereis(AgentRegistry) do
      %{
        available: true,
        snapshot: AgentRegistry.status_snapshot(),
        agents: AgentRegistry.public_agents()
      }
    else
      %{available: false, reason: "agent_registry_unavailable"}
    end
  rescue
    error -> %{available: false, reason: inspect(error)}
  catch
    :exit, reason -> %{available: false, reason: inspect(reason)}
  end

  defp agent_capacity(%{"capacity" => capacity}) when is_integer(capacity) and capacity > 0,
    do: capacity

  defp agent_capacity(_agent), do: 1

  defp utilization(_leased_slots, 0), do: 0.0
  defp utilization(leased_slots, capacity_slots), do: leased_slots / capacity_slots

  defp count_by(values, fun) do
    Enum.reduce(values, %{}, fn value, acc ->
      key = fun.(value)
      Map.update(acc, key, 1, &(&1 + 1))
    end)
  end

  defp operator_key(job) do
    cond do
      is_binary(job.worker_id) and job.worker_id != "" ->
        "worker:#{job.worker_id}"

      is_binary(job.model_version_id) and job.model_version_id != "" ->
        "model:#{job.model_version_id}"

      is_binary(job.simulation_case_id) and job.simulation_case_id != "" ->
        "case:#{job.simulation_case_id}"

      true ->
        "unknown_operator"
    end
  end

  defp worst_state([]), do: "healthy"

  defp worst_state(states) do
    Enum.max_by(states, &Map.get(@state_rank, &1, 0), fn -> "healthy" end)
  end

  defp clamp_progress(progress) when is_float(progress), do: max(0.0, min(progress, 1.0))
  defp clamp_progress(progress) when is_integer(progress), do: clamp_progress(progress * 1.0)
  defp clamp_progress(_), do: 0.0

  defp timeout_message(limit_ms) do
    "watchdog timed out job after #{limit_ms} ms"
  end

  defp stall_message(limit_ms) do
    "watchdog marked job stalled after #{limit_ms} ms without progress"
  end
end
