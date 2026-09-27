defmodule KyuubikiWeb.Storage.AnalysisMemoryStateTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Persistence, Storage}
  alias KyuubikiWeb.Jobs.{Job, Store}
  alias KyuubikiWeb.Storage.AnalysisMemoryState

  @moduletag skip: not Storage.memory?()
  @observers [KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, KyuubikiWeb.Jobs.Watchdog]

  setup do
    original = System.get_env("KYUUBIKI_DATA_DIR")

    directory =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-analysis-state-#{System.unique_integer([:positive])}"
      )

    # These tests manually remove the store; background scans are not part of
    # the snapshot-restart fixture and must not race its temporary data directory.
    Enum.each(@observers, fn child ->
      :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, child)
    end)

    stop_state()
    System.put_env("KYUUBIKI_DATA_DIR", directory)

    on_exit(fn ->
      stop_state()

      if original,
        do: System.put_env("KYUUBIKI_DATA_DIR", original),
        else: System.delete_env("KYUUBIKI_DATA_DIR")

      start_state()

      Enum.each(@observers, fn child ->
        assert {:ok, _pid} = Supervisor.restart_child(KyuubikiWeb.Supervisor, child)
      end)

      File.rm_rf!(directory)
    end)

    :ok
  end

  test "a completed job and result survive restart as one verified snapshot" do
    start_state()
    job = create_job()
    assert {:ok, completed} = Store.complete_with_result(job.job_id, "worker", %{"value" => 42})
    restart_state()
    assert {:ok, ^completed} = Store.get(job.job_id)
    assert {:ok, %{"value" => 42}} = AnalysisResultStore.get(job.job_id)
    refute File.exists?(Persistence.jobs_path())
    refute File.exists?(Persistence.results_path())
  end

  test "shared-state no-op detection preserves integer-to-float result changes" do
    start_state()
    job = create_job()
    assert :ok = AnalysisResultStore.put(job.job_id, %{"value" => 1})

    assert :ok =
             AnalysisResultStore.compare_and_swap(job.job_id, %{"value" => 1}, %{"value" => 1.0})

    assert {:ok, result} = AnalysisResultStore.get(job.job_id)
    assert result["value"] === 1.0
    restart_state()
    assert {:ok, reloaded} = AnalysisResultStore.get(job.job_id)
    assert reloaded["value"] === 1.0
  end

  test "legacy jobs and results migrate once without reviving stale copies on restart" do
    {:ok, legacy} =
      Job.new(%{job_id: "legacy", project_id: "migration", simulation_case_id: "case"})

    Persistence.write_json!(Persistence.jobs_path(), %{
      legacy.job_id => Job.to_persisted_map(legacy)
    })

    Persistence.write_json!(Persistence.results_path(), %{"legacy" => %{"legacy" => true}})
    start_state()
    assert {:ok, ^legacy} = Store.get("legacy")
    assert {:ok, %{"legacy" => true}} = AnalysisResultStore.get("legacy")

    assert {:ok, _} = Store.apply_progress(%{job_id: "legacy", stage: :cancelled, progress: 1.0})
    assert {:ok, _} = AnalysisResultStore.delete("legacy")
    restart_state()
    assert {:ok, %{status: :cancelled}} = Store.get("legacy")
    assert :error = AnalysisResultStore.get("legacy")
    assert Persistence.read_json(Persistence.jobs_path(), %{})["legacy"]["status"] == "queued"
  end

  test "a corrupt primary recovers both collections from the same previous generation" do
    start_state()
    job = create_job()
    assert {:ok, _} = Store.complete_with_result(job.job_id, "worker", %{"value" => 42})
    stop_state()
    File.write!(Persistence.analysis_state_path(), "{broken")
    start_state()

    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
    receipt = File.read!(Persistence.analysis_state_path() <> ".recovery.json") |> Jason.decode!()
    assert receipt["status"] == "recovered_previous_generation"
    assert {:ok, _} = Store.complete_with_result(job.job_id, "retry", %{"retried" => true})
  end

  test "an interrupted rename cannot combine a pending result with the previous job state" do
    start_state()
    job = create_job()
    assert {:ok, _} = Store.complete_with_result(job.job_id, "worker", %{"pending" => true})
    stop_state()
    path = Persistence.analysis_state_path()
    File.rename!(path, path <> ".next")
    start_state()

    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
    assert {:ok, _} = Store.complete_with_result(job.job_id, "retry", %{"retried" => true})
    restart_state()
    assert {:ok, %{status: :completed, worker_id: "retry"}} = Store.get(job.job_id)
    assert {:ok, %{"retried" => true}} = AnalysisResultStore.get(job.job_id)
  end

  test "a filesystem write failure leaves both collections unchanged and the store alive" do
    start_state()
    job = create_job()
    pid = Process.whereis(AnalysisMemoryState)
    next = Persistence.analysis_state_path() <> ".next"
    File.mkdir_p!(next)
    File.write!(Path.join(next, "blocker"), "injected fault")

    assert {:error, {:completion_persistence_failed, _}} =
             Store.complete_with_result(job.job_id, "failed-worker", %{"bad" => true})

    assert Process.whereis(AnalysisMemoryState) == pid
    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
    File.rm_rf!(next)
    restart_state()
    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)

    assert {:ok, _} = Store.complete_with_result(job.job_id, "retry", %{"ok" => true})
    restart_state()
    assert {:ok, %{status: :completed, worker_id: "retry"}} = Store.get(job.job_id)
    assert {:ok, %{"ok" => true}} = AnalysisResultStore.get(job.job_id)
  end

  @tag capture_log: true
  test "unrecoverable shared state fails closed instead of importing stale legacy files" do
    {:ok, legacy} =
      Job.new(%{job_id: "legacy", project_id: "migration", simulation_case_id: "case"})

    Persistence.write_json!(Persistence.jobs_path(), %{"legacy" => Job.to_persisted_map(legacy)})
    start_state()
    assert {:ok, _} = Store.complete_with_result("legacy", "worker", %{"ok" => true})
    stop_state()
    path = Persistence.analysis_state_path()
    File.write!(path, "{broken")
    File.write!(path <> ".previous", "{broken")

    assert {:error, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, AnalysisMemoryState)
    assert Process.whereis(AnalysisMemoryState) == nil
    assert {:error, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, AnalysisMemoryState)
    assert Process.whereis(AnalysisMemoryState) == nil
  end

  @tag capture_log: true
  test "unrecoverable legacy state is not silently migrated as an empty database" do
    Persistence.ensure_dir!()
    File.write!(Persistence.jobs_path(), "{broken")
    assert {:error, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, AnalysisMemoryState)
    refute File.exists?(Persistence.analysis_state_path())
  end

  defp create_job do
    assert {:ok, job} =
             Store.create(%{
               job_id: "memory",
               project_id: "atomic-memory",
               simulation_case_id: "case",
               status: :solving,
               progress: 0.9
             })

    job
  end

  defp stop_state do
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, AnalysisMemoryState)
  end

  defp start_state do
    assert {:ok, _pid} = Supervisor.restart_child(KyuubikiWeb.Supervisor, AnalysisMemoryState)
  end

  defp restart_state do
    stop_state()
    start_state()
  end
end
