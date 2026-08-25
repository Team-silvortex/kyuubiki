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
