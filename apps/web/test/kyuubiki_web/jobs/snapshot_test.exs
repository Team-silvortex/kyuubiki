defmodule KyuubikiWeb.Jobs.SnapshotTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.Store

  setup do
    Store.reset()
    on_exit(fn -> Store.reset() end)
    :ok
  end

  test "a current snapshot may fail once but the old snapshot may not be reused" do
    job = create_job()
    event = failure(job)
    assert {:ok, %{status: :failed} = failed} = Store.apply_progress_if_current(event, job)

    assert {:error, {:stale_job_snapshot, "snapshot-job"}} =
             Store.apply_progress_if_current(event, job)

    assert {:ok, ^failed} = Store.get(job.job_id)
    assert {:ok, ^failed} = Store.apply_progress_if_current(event, failed)
  end

  test "changed progress invalidates a snapshot even without a changed timestamp" do
    job = create_job()

    assert {:ok, current} =
             Store.apply_progress(%{
               job_id: job.job_id,
               stage: :solving,
               progress: 0.6,
               emitted_at: job.updated_at
             })

    assert current.updated_at == job.updated_at

    assert {:error, {:stale_job_snapshot, "snapshot-job"}} =
             Store.apply_progress_if_current(failure(job), job)

    assert {:ok, ^current} = Store.get(job.job_id)
  end

  for mutation <- [:worker, :metadata] do
    test "changed #{mutation} invalidates a snapshot at the same timestamp" do
      job = create_job(DateTime.add(DateTime.utc_now(), 60, :second))

      {:ok, current} =
        case unquote(mutation) do
          :worker -> Store.assign_worker(job.job_id, "new-worker")
          :metadata -> Store.update_metadata(job.job_id, %{"message" => "new metadata"})
        end

      assert current.updated_at == job.updated_at
      event = Map.put(failure(job), :emitted_at, job.updated_at)

      assert {:error, {:stale_job_snapshot, "snapshot-job"}} =
               Store.apply_progress_if_current(event, job)

      assert {:ok, ^current} = Store.get(job.job_id)
    end
  end

  test "a deleted snapshot returns not-found and is never resurrected" do
    job = create_job()
    assert {:ok, _} = Store.delete(job.job_id)

    assert {:error, {:job_not_found, "snapshot-job"}} =
             Store.apply_progress_if_current(failure(job), job)

    assert :error = Store.get(job.job_id)
  end

  test "only one concurrent conditional writer can consume a snapshot" do
    job = create_job()

    results =
      1..8
      |> Enum.map(fn index ->
        Task.async(fn ->
          event = Map.put(failure(job), :message, "decision-#{index}")
          Store.apply_progress_if_current(event, job)
        end)
      end)
      |> Task.await_many(5_000)

    assert [{:ok, winner}] = Enum.filter(results, &match?({:ok, _}, &1))
    assert Enum.count(results, &match?({:error, {:stale_job_snapshot, "snapshot-job"}}, &1)) == 7
    assert {:ok, ^winner} = Store.get(job.job_id)
  end

  test "mismatched job identity and malformed events leave the snapshot unchanged" do
    job = create_job()

    assert {:error, {:job_id_mismatch, "snapshot-job", "other-job"}} =
             Store.apply_progress_if_current(Map.put(failure(job), :job_id, "other-job"), job)

    assert {:error, _} =
             Store.apply_progress_if_current(Map.put(failure(job), :progress, 2.0), job)

    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "invalid required metadata cannot corrupt a stored job" do
    job = create_job()
    assert {:error, _} = Store.update_metadata(job.job_id, %{"project_id" => nil})
    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "a snapshot returned on create works with second precision timestamps" do
    timestamp = DateTime.utc_now(:second)
    job = create_job(timestamp, false)
    assert {:ok, %{status: :failed}} = Store.apply_progress_if_current(failure(job), job)
    assert {:ok, %{status: :failed}} = Store.get(job.job_id)
  end

  test "worker and metadata writes never move a future progress clock backwards" do
    job = create_job(DateTime.add(DateTime.utc_now(), 60, :second))
    assert {:ok, assigned} = Store.assign_worker(job.job_id, "worker-future")
    assert {:ok, updated} = Store.update_metadata(job.job_id, %{"message" => "reviewed"})
    assert assigned.updated_at == job.updated_at
    assert updated.updated_at == job.updated_at
    assert updated.execution_started_at == job.execution_started_at
    assert updated.worker_id == "worker-future"
    assert {:ok, ^updated} = Store.get(job.job_id)
  end

  defp create_job(timestamp \\ DateTime.utc_now(), reload \\ true) do
    {:ok, job} =
      Store.create(%{
        job_id: "snapshot-job",
        project_id: "snapshot-contract",
        simulation_case_id: "case",
        status: :solving,
        progress: 0.1,
        created_at: timestamp,
        execution_started_at: timestamp,
        updated_at: timestamp
      })

    if reload do
      {:ok, current} = Store.get(job.job_id)
      current
    else
      job
    end
  end

  defp failure(job) do
    %{job_id: job.job_id, stage: :failed, progress: job.progress, message: "watchdog decision"}
  end
end
