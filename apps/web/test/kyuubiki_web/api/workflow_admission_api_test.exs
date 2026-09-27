defmodule KyuubikiWeb.Api.WorkflowAdmissionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.TestSupport.{StorageOutageFixture, WorkflowPreflightContract}

  @moduletag skip: Storage.postgres?()

  setup do
    StorageOutageFixture.setup()
  end

  test "failed admission is an API error with no partial task and a clean retry succeeds" do
    restore = reject_admission()
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    {422, rejected} = submit()
    assert rejected["error"] =~ "completion_persistence_failed"
    assert Store.list() == []
    assert AnalysisResultStore.list() == []
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator

    restore.()
    {202, submitted} = submit()
    job_id = submitted["job"]["job_id"]
    result = WorkflowApi.wait_for_job(job_id, @opts)
    assert result["job"]["status"] == "completed"
    assert result["result"]["artifacts"]["output.payload"] == %{"value" => 42}
    assert length(Store.list()) == 1
    assert length(AnalysisResultStore.list()) == 1
  end

  defp submit do
    conn =
      :post
      |> conn("/api/v1/workflows/graph/jobs", Jason.encode!(WorkflowPreflightContract.request()))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end

  defp reject_admission do
    if Storage.sqlite?() do
      repo = Storage.repo_module!()
      name = "reject_api_admission_#{System.unique_integer([:positive])}"
      drop = fn -> Ecto.Adapters.SQL.query!(repo, "DROP TRIGGER IF EXISTS #{name}") end
      on_exit(drop)

      Ecto.Adapters.SQL.query!(repo, """
      CREATE TRIGGER #{name} BEFORE INSERT ON kyuubiki_analysis_results
      BEGIN SELECT RAISE(ABORT, 'injected API admission failure'); END
      """)

      drop
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
