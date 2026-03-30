defmodule KyuubikiWeb.Jobs.JobTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Jobs.Job
  alias KyuubikiWeb.Jobs.ProgressEvent

  test "rejects unknown statuses" do
    assert {:error, {:invalid_status, "mystery"}} =
             Job.new(%{
               job_id: "job-1",
               project_id: "project-1",
               simulation_case_id: "case-1",
               status: "mystery"
             })
  end

  test "rejects progress outside the 0..1 range" do
    assert {:error, {:invalid_progress, :progress}} =
             Job.new(%{
               job_id: "job-1",
               project_id: "project-1",
               simulation_case_id: "case-1",
               progress: 1.2
             })
  end

  test "keeps the last solver metrics when a completion event omits them" do
    {:ok, job} =
      Job.new(%{
        job_id: "job-1",
        project_id: "project-1",
        simulation_case_id: "case-1",
        residual: 0.25,
        iteration: 3
      })

    {:ok, progress_event} =
      ProgressEvent.new(%{
        job_id: "job-1",
        stage: "completed",
        progress: 1.0
      })

    updated = Job.apply_progress(job, progress_event)

    assert updated.status == :completed
    assert updated.progress == 1.0
    assert updated.iteration == 3
    assert updated.residual == 0.25
  end
end
