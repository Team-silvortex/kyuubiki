defmodule KyuubikiWeb.Jobs.AtomicCompletionSqlTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault

  @moduletag skip: not Storage.sql?()

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    :ok
  end

  @tag skip: not Storage.sqlite?()
  test "a database insert failure rolls back the completed job and permits a clean retry" do
    job = create_job("insert-failure")
    drop = AnalysisCommitFault.reject_result(job.job_id)

    assert {:error, {:completion_persistence_failed, reason}} =
             Store.complete_with_result(job.job_id, "worker", %{"ok" => true})

    assert reason =~ "injected result write failure"
    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)

    drop.()
    assert {:ok, %{status: :completed}} = Store.complete_with_result(job.job_id, "worker", %{})
    assert {:ok, %{}} = AnalysisResultStore.get(job.job_id)
  end

  @tag capture_log: true
  test "killing a publisher between its job and result writes rolls back both" do
    job = create_job("killed-publisher")
    handler = AnalysisCommitFault.pause_after_job_write(job.job_id)

    {pid, ref} =
      spawn_monitor(fn ->
        Store.complete_with_result(job.job_id, "interrupted-worker", %{"interrupted" => true})
      end)

    assert_receive {:completion_paused, ^handler, ^pid}, 2_000
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000
    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)

    assert {:ok, %{status: :completed, worker_id: "retry-worker"}} =
             Store.complete_with_result(job.job_id, "retry-worker", %{"retry" => true})

    assert {:ok, %{"retry" => true}} = AnalysisResultStore.get(job.job_id)
  end

  test "a completion that owns the row cannot be partially cancelled" do
    job = create_job("completion-first")
    handler = AnalysisCommitFault.pause_after_job_write(job.job_id)

    publisher =
      Task.async(fn -> Store.complete_with_result(job.job_id, "worker", %{"ok" => true}) end)

    assert_receive {:completion_paused, ^handler, pid}, 2_000

    canceller =
      Task.async(fn ->
        Store.apply_progress(%{job_id: job.job_id, stage: :cancelled, progress: 1.0})
      end)

    send(pid, :resume_completion)
    assert {:ok, %{status: :completed}} = Task.await(publisher)
    assert {:error, _reason} = Task.await(canceller)
    assert {:ok, %{status: :completed, worker_id: "worker"}} = Store.get(job.job_id)
    assert {:ok, %{"ok" => true}} = AnalysisResultStore.get(job.job_id)
  end

  defp create_job(id) do
    assert {:ok, job} =
             Store.create(%{
               job_id: id,
               project_id: "atomic-sql",
               simulation_case_id: "case",
               status: :solving,
               progress: 0.9
             })

    job
  end
end
