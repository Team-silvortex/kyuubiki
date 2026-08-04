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

  test "does not revive a terminal job with a late agent heartbeat" do
    {:ok, job} =
      Job.new(%{
        job_id: "job-terminal",
        project_id: "project-1",
        simulation_case_id: "case-1",
        status: :failed,
        progress: 0.4,
        message: "watchdog marked job stalled"
      })

    {:ok, heartbeat} =
      ProgressEvent.new(%{
        job_id: "job-terminal",
        stage: "solving",
        progress: 0.7,
        message: "agent heartbeat: solver still active"
      })

    assert Job.apply_progress(job, heartbeat) == job
  end

  test "records separate queue and execution deadlines" do
    created_at = ~U[2026-08-04 12:00:00.000000Z]

    {:ok, job} =
      Job.new(%{
        job_id: "job-timing",
        project_id: "project-1",
        simulation_case_id: "case-1",
        queue_timeout_ms: 120_000,
        execution_timeout_ms: 600_000,
        created_at: created_at,
        updated_at: created_at
      })

    assert %{
             "phase" => "queue",
             "queue_wait_ms" => 0,
             "execution_elapsed_ms" => nil,
             "total_elapsed_ms" => 0,
             "effective_timeout_ms" => 120_000,
             "job_submission_deadline" => "2026-08-04T12:02:00.000000Z"
           } = Job.status_detail(job)["timing"]

    {:ok, event} =
      ProgressEvent.new(%{
        job_id: "job-timing",
        stage: "preprocessing",
        progress: 0.01,
        emitted_at: ~U[2026-08-04 12:01:00.000000Z]
      })

    running = Job.apply_progress(job, event)

    assert running.execution_started_at == ~U[2026-08-04 12:01:00.000000Z]

    assert %{
             "phase" => "execution",
             "queue_wait_ms" => 60_000,
             "execution_elapsed_ms" => 0,
             "total_elapsed_ms" => 60_000,
             "effective_timeout_ms" => 600_000,
             "effective_deadline" => "2026-08-04T12:11:00.000000Z"
           } = Job.status_detail(running)["timing"]
  end
end
