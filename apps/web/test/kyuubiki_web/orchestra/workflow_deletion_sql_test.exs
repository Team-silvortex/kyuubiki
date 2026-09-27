defmodule KyuubikiWeb.Orchestra.WorkflowDeletionSqlTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, StorageOutageFixture}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.sqlite?()

  setup do
    StorageOutageFixture.setup()
  end

  for table <- ["kyuubiki_jobs", "kyuubiki_analysis_results"], terminal <- [false, true] do
    test "rejected #{table} deletion preserves both records with terminal=#{terminal}" do
      fixture = Fixture.claimed_job()

      fixture =
        if unquote(terminal) do
          assert :ok =
                   WorkflowRecoveryCoordinator.commit_result(
                     fixture.id,
                     fixture.claim,
                     Fixture.result()
                   )

          {:ok, job} = Store.get(fixture.id)
          {:ok, runtime} = AnalysisResultStore.get(fixture.id)
          %{fixture | job: job, runtime: runtime}
        else
          fixture
        end

      restore = AnalysisCommitFault.reject_delete(fixture.id, unquote(table))
      coordinator = Process.whereis(WorkflowRecoveryCoordinator)
      assert {:error, _} = Analysis.delete_job(fixture.id)
      Fixture.unchanged(fixture)
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator

      restore.()
      assert {:ok, %{"deleted" => true}} = Analysis.delete_job(fixture.id)
      assert :error = Store.get(fixture.id)
      assert :error = AnalysisResultStore.get(fixture.id)
    end
  end

  @tag capture_log: true
  test "process loss after DELETE rolls back both records and releases the lease transaction" do
    fixture = Fixture.claimed_job()
    assert {:ok, lease} = LeaseStore.acquire("deletion-probe", LeaseStore.instance_id(), 120_000)
    on_exit(fn -> LeaseStore.release(lease) end)
    probe = AnalysisCommitFault.pause_after_job_delete(fixture.id)
    delete = fn -> Store.delete_with_result(fixture.id) end
    {pid, ref} = spawn_monitor(fn -> LeaseStore.with_lease(lease, delete) end)
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)

    assert_receive {:completion_paused, ^probe, ^pid}, 2_000
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000
    Fixture.unchanged(fixture)
    assert {:ok, _job} = LeaseStore.with_lease(lease, delete)
    assert :error = Store.get(fixture.id)
    assert :error = AnalysisResultStore.get(fixture.id)
  end
end
