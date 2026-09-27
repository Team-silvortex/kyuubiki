defmodule KyuubikiWeb.Orchestra.WorkflowAdmissionSqlTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault
  alias KyuubikiWeb.TestSupport.{StorageOutageFixture, WorkflowPreflightContract}

  @moduletag skip: not Storage.sqlite?()

  setup do
    StorageOutageFixture.setup()
  end

  test "rejected runtime insertion needs no compensating job delete" do
    repo = Storage.repo_module!()
    suffix = System.unique_integer([:positive])
    insert_trigger = "reject_admission_result_#{suffix}"
    delete_trigger = "reject_admission_cleanup_#{suffix}"

    drop = fn ->
      for name <- [insert_trigger, delete_trigger] do
        Ecto.Adapters.SQL.query!(repo, "DROP TRIGGER IF EXISTS #{name}")
      end
    end

    on_exit(drop)

    Ecto.Adapters.SQL.query!(repo, """
    CREATE TRIGGER #{insert_trigger} BEFORE INSERT ON kyuubiki_analysis_results
    BEGIN SELECT RAISE(ABORT, 'injected admission result failure'); END
    """)

    Ecto.Adapters.SQL.query!(repo, """
    CREATE TRIGGER #{delete_trigger} BEFORE DELETE ON kyuubiki_jobs
    BEGIN SELECT RAISE(ABORT, 'compensating deletion is not atomic admission'); END
    """)

    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    assert {:error, _} = Analysis.submit_workflow_graph(WorkflowPreflightContract.request())
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert Store.list() == []
    assert AnalysisResultStore.list() == []

    drop.()
    assert {:ok, submitted} = Analysis.submit_workflow_graph(WorkflowPreflightContract.request())
    id = submitted["job"]["job_id"]
    StorageOutageFixture.eventually(fn -> match?({:ok, %{status: :completed}}, Store.get(id)) end)
    assert length(Store.list()) == 1
    assert length(AnalysisResultStore.list()) == 1
  end

  @tag capture_log: true
  test "process loss after the job insert rolls back admission and frees its lease transaction" do
    attrs = %{
      job_id: "interrupted-admission",
      project_id: "admission",
      simulation_case_id: "case"
    }

    assert {:ok, lease} = LeaseStore.acquire("admission-probe", LeaseStore.instance_id(), 120_000)
    on_exit(fn -> LeaseStore.release(lease) end)
    probe = AnalysisCommitFault.pause_after_job_insert(attrs.job_id)
    create = fn -> Store.create_with_result(attrs, %{"pending" => true}) end
    {pid, ref} = spawn_monitor(fn -> LeaseStore.with_lease(lease, create) end)
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    assert_receive {:completion_paused, ^probe, ^pid}, 2_000
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000

    assert :error = Store.get(attrs.job_id)
    assert :error = AnalysisResultStore.get(attrs.job_id)
    assert {:ok, job} = LeaseStore.with_lease(lease, create)
    assert job.status == :queued
    assert {:ok, %{"pending" => true}} = AnalysisResultStore.get(attrs.job_id)
  end
end
