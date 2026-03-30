defmodule KyuubikiWeb.Workers.MockWorkerAdapterTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Workers.MockWorkerAdapter

  setup do
    Store.reset()

    {:ok, job} =
      Store.create(%{
        job_id: "job-1",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

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
end
