defmodule KyuubikiWeb.Workers.MockWorkerAdapterTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Workers.MockWorkerAdapter

  setup do
    original = Application.get_env(:kyuubiki_web, MockWorkerAdapter, [])
    Application.put_env(:kyuubiki_web, MockWorkerAdapter, Keyword.put(original, :enabled?, true))

    Store.reset()

    {:ok, job} =
      Store.create(%{
        job_id: "job-1",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, MockWorkerAdapter, original)
    end)

    %{job: job}
  end

  test "runs a worker command and persists streamed progress into the store", %{job: job} do
    output = """
    event|job-1|solving|0.50|2|0.125|576|mock solve step 2/4
    event|job-1|completed|1.00|||| 
    """

    runner = fn _job, _opts -> {output, 0} end

    assert {:ok, events} = MockWorkerAdapter.run_job(job, runner: runner)
    assert length(events) == 2

    assert {:ok, updated_job} = Store.get(job.job_id)
    assert updated_job.worker_id == "rust-cli"
    assert updated_job.status == :completed
    assert updated_job.progress == 1.0
  end

  test "returns an error when the worker exits unsuccessfully", %{job: job} do
    runner = fn _job, _opts -> {"worker exploded", 1} end

    assert {:error, {:worker_command_failed, 1, "worker exploded"}} =
             MockWorkerAdapter.run_job(job, runner: runner)
  end

  test "rejects malformed progress without assigning or mutating the job", %{job: job} do
    runner = fn _job, _opts ->
      {"event|job-1|solving|not-a-number|2|0.125|576|bad progress", 0}
    end

    assert {:error, {:invalid_worker_output, {:invalid_progress, "not-a-number"}, _line}} =
             MockWorkerAdapter.run_job(job, runner: runner)

    assert {:ok, unchanged} = Store.get(job.job_id)
    assert unchanged.status == :queued
    assert unchanged.worker_id == nil
  end

  test "rejects output for another job before persisting any event", %{job: job} do
    {:ok, _other_job} =
      Store.create(%{
        job_id: "job-2",
        project_id: "project-1",
        simulation_case_id: "case-2"
      })

    output = """
    event|job-1|solving|0.50|2|0.125|576|valid first event
    event|job-2|completed|1.00||||foreign completion
    """

    runner = fn _job, _opts -> {output, 0} end

    assert {:error, {:worker_job_mismatch, "job-1", "job-2"}} =
             MockWorkerAdapter.run_job(job, runner: runner)

    assert {:ok, unchanged} = Store.get(job.job_id)
    assert unchanged.status == :queued
    assert unchanged.worker_id == nil

    assert {:ok, other_unchanged} = Store.get("job-2")
    assert other_unchanged.status == :queued
  end

  test "rejects successful process output without a terminal event", %{job: job} do
    runner = fn _job, _opts ->
      {"event|job-1|solving|0.50|2|0.125|576|unfinished", 0}
    end

    assert {:error, :worker_output_missing_terminal_event} =
             MockWorkerAdapter.run_job(job, runner: runner)

    assert {:ok, unchanged} = Store.get(job.job_id)
    assert unchanged.status == :queued
    assert unchanged.worker_id == nil
  end

  test "rejects successful process output without progress events", %{job: job} do
    runner = fn _job, _opts -> {"worker exited without events", 0} end

    assert {:error, :worker_output_missing_progress} =
             MockWorkerAdapter.run_job(job, runner: runner)
  end

  test "refuses to run when transitional worker adapters are disabled", %{job: job} do
    Application.put_env(:kyuubiki_web, MockWorkerAdapter, enabled?: false)

    assert {:error, :transitional_worker_adapter_disabled} =
             MockWorkerAdapter.run_job(job)
  end
end
