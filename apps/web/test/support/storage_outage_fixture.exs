defmodule KyuubikiWeb.TestSupport.StorageOutageFixture do
  @moduledoc false

  import ExUnit.Assertions
  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Orchestra.{LeaseMemoryBackend, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.Storage.AnalysisMemoryState

  @observers [WorkflowRecoveryCoordinator, Watchdog]

  def setup do
    original = Map.new(@observers, &{&1, Application.get_env(:kyuubiki_web, &1, [])})
    Enum.each(@observers, &stop/1)
    Store.reset()
    AnalysisResultStore.reset()

    Application.put_env(:kyuubiki_web, WorkflowRecoveryCoordinator,
      lease_ttl_ms: 120_000,
      lease_heartbeat_ms: 60_000,
      lease_retry_ms: 100
    )

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 5_000,
      job_timeout_ms: 120_000
    )

    ExUnit.Callbacks.on_exit(fn ->
      Enum.each(@observers, &stop/1)
      start(store_child())
      if Storage.memory?(), do: start(LeaseMemoryBackend)
      Store.reset()
      AnalysisResultStore.reset()

      Enum.each(original, fn {child, config} ->
        Application.put_env(:kyuubiki_web, child, config)
      end)

      Enum.each(@observers, &start/1)
    end)

    Enum.each(@observers, &start/1)
    assert %{"lease" => %{"status" => "owner"}} = WorkflowRecoveryCoordinator.snapshot()

    :ok
  end

  def store_child, do: Storage.repo_module() || AnalysisMemoryState

  def without(child, callback) do
    stop(child)

    try do
      callback.()
    after
      start(child)
    end
  end

  def start(child) do
    case Supervisor.restart_child(KyuubikiWeb.Supervisor, child) do
      {:ok, _pid} -> :ok
      {:error, :running} -> :ok
    end
  end

  def stop(child), do: Supervisor.terminate_child(KyuubikiWeb.Supervisor, child)

  def eventually(check, attempts \\ 200)
  def eventually(_check, 0), do: flunk("storage recovery did not complete")

  def eventually(check, attempts) do
    if check.() do
      :ok
    else
      Process.sleep(10)
      eventually(check, attempts - 1)
    end
  end
end
