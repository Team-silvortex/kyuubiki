defmodule KyuubikiWeb.Api.WorkflowDeletionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowJobRunner, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, StorageOutageFixture, StorageOutageWorkflow}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    StorageOutageFixture.setup()
  end

  @tag skip: Storage.postgres?()
  test "failed API deletion preserves the running task and permits a complete retry" do
    StorageOutageWorkflow.install(self())
    assert {:ok, submitted} = StorageOutageWorkflow.submit("idempotent")
    id = submitted["job"]["job_id"]
    assert_receive {:outage_solver_request, ^id, :hold}, 2_000
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    on_exit(fn -> if Process.alive?(runner), do: Process.exit(runner, :shutdown) end)
    assert {:ok, job} = Store.get(id)
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    restore = reject_deletion(id)

    {422, rejected} = delete(id)
    assert rejected["error"] =~ "completion_persistence_failed"
    assert Process.alive?(runner)
    assert {:ok, ^job} = Store.get(id)
    assert {:ok, ^runtime} = AnalysisResultStore.get(id)

    restore.()
    {200, deleted} = delete(id)
    assert deleted["deleted"] == true
    assert deleted["job"]["status"] == Atom.to_string(job.status)
    assert_receive {:DOWN, ^ref, :process, ^runner, _reason}, 2_000
    assert :error = Store.get(id)
    assert :error = AnalysisResultStore.get(id)
    assert {404, %{"error" => "job_not_found"}} = delete(id)

    for url <- ["/api/v1/jobs/#{id}", "/api/v1/results/#{id}"] do
      assert %{status: 404} = conn(:get, url) |> Router.call(@opts)
    end
  end

  test "the deletion API reports lost authority rather than a false success" do
    fixture = Fixture.claimed_job()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, old} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(old)
    assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)

    assert {422, %{"error" => ":orchestra_lease_lost"}} = delete(fixture.id)
    Fixture.unchanged(fixture)
  end

  defp delete(id) do
    conn = conn(:delete, "/api/v1/jobs/#{id}") |> Router.call(@opts)
    {conn.status, Jason.decode!(conn.resp_body)}
  end

  defp reject_deletion(id) do
    if Storage.sqlite?() do
      AnalysisCommitFault.reject_delete(id, "kyuubiki_jobs")
    else
      path = Persistence.analysis_state_path() <> ".next"
      restore = fn -> File.rm_rf!(path) end
      on_exit(restore)
      File.mkdir_p!(path)
      File.write!(Path.join(path, "blocker"), "injected fault")
      restore
    end
  end
end
