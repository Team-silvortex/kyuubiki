defmodule KyuubikiWeb.Api.JobRecordRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.StorageOutageFixture

  setup do
    StorageOutageFixture.setup()
  end

  test "explicit null clears optional version without changing execution or result" do
    job = job()
    result = %{"metric" => 42}
    assert :ok = AnalysisResultStore.put(job.job_id, result)

    assert {200, %{"job" => saved}} = request(:patch, job.job_id, %{"model_version_id" => nil})
    assert saved["job_id"] == job.job_id
    assert saved["model_version_id"] == nil
    assert saved["status"] == Atom.to_string(job.status)
    assert saved["progress"] == job.progress
    assert saved["project_id"] == job.project_id
    assert saved["simulation_case_id"] == job.simulation_case_id
    assert {:ok, current} = Store.get(job.job_id)
    assert current.model_version_id == nil
    assert {:ok, ^result} = AnalysisResultStore.get(job.job_id)

    assert Map.drop(Map.from_struct(current), [:model_version_id, :updated_at]) ==
             Map.drop(Map.from_struct(job), [:model_version_id, :updated_at])
  end

  test "omitted version is unchanged and is distinct from clearing" do
    job = job()
    assert {200, %{"job" => saved}} = request(:patch, job.job_id, %{"message" => "reviewed"})
    assert saved["model_version_id"] == "retained-version"
    assert {:ok, current} = Store.get(job.job_id)
    assert current.model_version_id == job.model_version_id
    assert current.message == "reviewed"
  end

  for field <- ["project_id", "simulation_case_id"], value <- [nil, "", "  "] do
    test "required #{field} rejects #{inspect(value)} without changing either record" do
      job = job()
      result = %{"unchanged" => true}
      assert :ok = AnalysisResultStore.put(job.job_id, result)

      assert {422, %{"error" => error}} =
               request(:patch, job.job_id, %{unquote(field) => unquote(value)})

      assert is_binary(error)
      assert {:ok, ^job} = Store.get(job.job_id)
      assert {:ok, ^result} = AnalysisResultStore.get(job.job_id)
    end
  end

  test "confirmed job deletion removes its result, preserves other work and cannot be repeated" do
    deleted = job()
    retained = job()
    assert :ok = AnalysisResultStore.put(deleted.job_id, %{"deleted" => true})
    assert :ok = AnalysisResultStore.put(retained.job_id, %{"retained" => true})
    assert {200, %{"deleted" => true, "job" => receipt}} = request(:delete, deleted.job_id)
    assert receipt["job_id"] == deleted.job_id
    assert :error = Store.get(deleted.job_id)
    assert :error = AnalysisResultStore.get(deleted.job_id)
    assert {:ok, ^retained} = Store.get(retained.job_id)
    assert {:ok, %{"retained" => true}} = AnalysisResultStore.get(retained.job_id)
    assert {404, %{"error" => "job_not_found"}} = request(:delete, deleted.job_id)
  end

  test "confirmed deletion stays committed when later collection reads lose storage" do
    deleted = job()
    assert :ok = AnalysisResultStore.put(deleted.job_id, %{"deleted" => true})
    assert {200, %{"deleted" => true}} = request(:delete, deleted.job_id)
    assert :error = Store.get(deleted.job_id)
    assert :error = AnalysisResultStore.get(deleted.job_id)

    StorageOutageFixture.without(StorageOutageFixture.store_child(), fn ->
      for path <- ["/api/v1/jobs", "/api/v1/results"] do
        conn = conn(:get, path) |> Router.call(@opts)
        assert conn.status == 503
        assert %{"error" => "analysis_store_unavailable"} = Jason.decode!(conn.resp_body)
      end
    end)
  end

  defp job do
    assert {:ok, job} =
             Store.create(%{
               job_id: "job-record-#{System.unique_integer([:positive])}",
               project_id: "project",
               simulation_case_id: "case",
               model_version_id: "retained-version",
               message: "original"
             })

    job
  end

  defp request(method, id, body \\ %{}) do
    conn =
      conn(method, "/api/v1/jobs/#{id}", Jason.encode!(body))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end
end
