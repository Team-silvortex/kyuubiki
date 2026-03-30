defmodule KyuubikiWeb.Jobs.StoreTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.Store

  setup do
    Store.reset()
    :ok
  end

  test "creates a job and applies progress updates" do
    {:ok, job} =
      Store.create(%{
        job_id: "job-1",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    assert job.status == :queued

    {:ok, updated} =
      Store.apply_progress(%{
        job_id: "job-1",
        stage: "solving",
        progress: 0.5,
        iteration: 12,
        residual: 1.0e-4
      })

    assert updated.status == :solving
    assert updated.progress == 0.5
    assert updated.iteration == 12
    assert updated.residual == 1.0e-4
  end
end
