defmodule KyuubikiWeb.Jobs.JointSnapshotTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
    {:ok, fixture: Fixture.claimed_job()}
  end

  test "a stale result rolls back job progress inside a lease transaction", %{fixture: fixture} do
    replacement = Map.put(fixture.runtime, "newer", true)
    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, replacement)
    assert {:ok, lease} = LeaseStore.acquire("snapshot-#{fixture.id}", "snapshot-owner", 60_000)
    on_exit(fn -> LeaseStore.release(lease) end)

    assert {:error, :stale_analysis_result} =
             LeaseStore.with_lease(lease, fn -> joint_update(fixture) end)

    Fixture.unchanged(%{fixture | runtime: replacement})
  end

  test "a missing result rolls back job progress", %{fixture: fixture} do
    assert {:ok, _} = AnalysisResultStore.delete(fixture.id)
    id = fixture.id
    assert {:error, {:result_not_found, ^id}} = joint_update(fixture)
    assert {:ok, job} = Store.get(id)
    assert job == fixture.job
    assert :error = AnalysisResultStore.get(id)
  end

  test "a stale job does not overwrite a result", %{fixture: fixture} do
    assert {:ok, changed} = Store.update_metadata(fixture.id, %{"message" => "newer metadata"})
    id = fixture.id
    assert {:error, {:stale_job_snapshot, ^id}} = joint_update(fixture)
    Fixture.unchanged(%{fixture | job: changed})
  end

  test "only one concurrent writer can consume a job and result snapshot", %{fixture: fixture} do
    outcomes =
      for index <- 1..8 do
        Task.async(fn ->
          Store.apply_progress_with_result(
            %{job_id: fixture.id, stage: :solving, progress: 0.25, message: "writer-#{index}"},
            fixture.job,
            fixture.runtime,
            Map.put(fixture.runtime, "writer", index)
          )
        end)
      end
      |> Task.await_many(10_000)

    assert [{:ok, winner}] = Enum.filter(outcomes, &match?({:ok, _}, &1))
    assert Enum.count(outcomes, &match?({:error, _}, &1)) == 7
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert winner.message == "writer-#{runtime["writer"]}"
  end

  test "unchanged progress can conditionally replace the result", %{fixture: fixture} do
    replacement = Map.put(fixture.runtime, "next", true)

    assert {:ok, same_job} =
             Store.apply_progress_with_result(
               %{
                 job_id: fixture.id,
                 stage: :solving,
                 progress: fixture.job.progress,
                 emitted_at: fixture.job.updated_at
               },
               fixture.job,
               fixture.runtime,
               replacement
             )

    assert same_job == fixture.job
    Fixture.unchanged(%{fixture | runtime: replacement})
  end

  defp joint_update(fixture) do
    Store.apply_progress_with_result(
      %{job_id: fixture.id, stage: :solving, progress: 0.25},
      fixture.job,
      fixture.runtime,
      Map.put(fixture.runtime, "new", true)
    )
  end
end
