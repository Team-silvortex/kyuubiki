defmodule KyuubikiWeb.AnalysisSolverSubmissionsTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisSolverSubmissions
  alias KyuubikiWeb.Jobs.Store

  setup do
    Store.reset()

    {:ok, job} =
      Store.create(%{
        job_id: "agent-progress-projection",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    {:ok, running} =
      Store.apply_progress(%{
        job_id: job.job_id,
        stage: "solving",
        progress: 0.6,
        message: "first agent reached the solve phase"
      })

    %{job: running}
  end

  for stage <- ["completed", "failed", "cancelled"] do
    test "agent #{stage} progress is not a final job receipt", %{job: job} do
      assert :ok =
               AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
                 "stage" => unquote(stage),
                 "progress" => 1.0,
                 "iteration" => 9,
                 "residual" => 0.01,
                 "message" => "agent attempt ended"
               })

      assert {:ok, current} = Store.get(job.job_id)
      assert current.status in [:solving, :postprocessing]
      assert current.progress < 1.0
      assert current.progress >= job.progress
      assert current.iteration == 9
      assert current.residual == 0.01
    end
  end

  test "ordinary progress also reserves completion for the final result receipt", %{job: job} do
    assert :ok =
             AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
               "stage" => "postprocessing",
               "progress" => 1.0
             })

    assert {:ok, %{status: :postprocessing, progress: progress}} = Store.get(job.job_id)
    assert progress < 1.0
  end

  for {label, value} <- [{"negative", -1}, {"excessive", 1.1}, {"text", "1.0"}, {"null", nil}] do
    test "#{label} progress is rejected rather than clamped into a valid receipt", %{job: job} do
      assert {:error, _reason} =
               AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
                 "stage" => "completed",
                 "progress" => unquote(value)
               })

      assert {:ok, ^job} = Store.get(job.job_id)
    end
  end

  test "late attempt notifications cannot change an already cancelled job", %{job: job} do
    assert {:ok, cancelled} =
             Store.apply_progress(%{
               job_id: job.job_id,
               stage: :cancelled,
               progress: job.progress,
               message: "operator cancelled"
             })

    for stage <- ["completed", "failed", "cancelled", "solving"] do
      assert :ok =
               AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
                 "stage" => stage,
                 "progress" => 1.0,
                 "message" => "too late"
               })

      assert {:ok, ^cancelled} = Store.get(job.job_id)
    end
  end

  test "projects failover signals without regressing the persisted job", %{job: job} do
    assert :ok =
             AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
               "stage" => "recovering",
               "progress" => 0.01,
               "message" => "retrying on the next healthy agent"
             })

    assert {:ok, recovering} = Store.get(job.job_id)
    assert recovering.status == :solving
    assert recovering.progress == 0.6
    assert recovering.message == "retrying on the next healthy agent"

    assert :ok =
             AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
               "stage" => "preprocessing",
               "progress" => 0.01,
               "message" => "replacement agent accepted the task"
             })

    assert {:ok, redispatched} = Store.get(job.job_id)
    assert redispatched.status == :solving
    assert redispatched.progress == 0.6
    assert redispatched.message == "replacement agent accepted the task"

    assert :ok =
             AnalysisSolverSubmissions.apply_agent_progress(job.job_id, %{
               "stage" => "solving",
               "progress" => 0.7
             })

    assert {:ok, advanced} = Store.get(job.job_id)
    assert advanced.status == :solving
    assert advanced.progress == 0.7
  end
end
