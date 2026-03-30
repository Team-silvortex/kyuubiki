defmodule KyuubikiWeb.Jobs.JobTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Jobs.Job

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
end
