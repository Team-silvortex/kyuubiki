alias KyuubikiWeb.Jobs.Store

# Seed retained, incomplete runtime records through the real transactional store.
# This is an opt-in owned test fixture, never a simulated scientific result.
for status <- [:queued, :solving, :failed, :cancelled] do
  id = "owned-result-gate-#{status}"

  runtime = %{
    "workflow_id" => id,
    "artifacts" => %{"partial" => %{"fixture" => true}},
    "completed_nodes" => 0,
    "total_nodes" => 2
  }

  {:ok, _} =
    Store.create_with_result(
      %{job_id: id, project_id: "owned-result-fixture", simulation_case_id: "case"},
      runtime
    )

  if status != :queued do
    {:ok, _} = Store.apply_progress(%{job_id: id, stage: status, progress: 0.25})
  end
end
