defmodule KyuubikiWeb.Jobs.Watchdog do
  @moduledoc """
  Periodically scans active jobs and marks stale or timed-out jobs as failed so they do not
  remain stuck forever.
  """

  use GenServer

  alias KyuubikiWeb.Jobs.Store

  @active_statuses [:queued, :preprocessing, :partitioning, :solving, :postprocessing]

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  def scan_now do
    GenServer.call(__MODULE__, :scan_now, 30_000)
  end

  def status_snapshot do
    state = config()
    jobs = Store.list()

    %{
      scan_interval_ms: state.scan_interval_ms,
      stale_job_ms: state.stale_job_ms,
      job_timeout_ms: state.job_timeout_ms,
      active_jobs: Enum.count(jobs, &(&1.status in @active_statuses)),
      stalled_jobs:
        Enum.count(jobs, fn job ->
          job.status == :failed and is_binary(job.message) and String.contains?(job.message, "watchdog marked job stalled")
        end),
      timed_out_jobs:
        Enum.count(jobs, fn job ->
          job.status == :failed and is_binary(job.message) and String.contains?(job.message, "watchdog timed out job")
        end)
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
      job_timeout_ms: Keyword.get(env, :job_timeout_ms, 120_000)
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
