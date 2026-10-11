defmodule KyuubikiWeb.Api.ResultAdministrationApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.TestSupport.StorageOutageFixture
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    StorageOutageFixture.setup()
  end

  test "missing parent and missing result produce distinct 404s without creating data" do
    assert {404, %{"error" => "job_not_found"}} = patch("no-parent", %{"value" => 1})
    assert {404, %{"error" => "job_not_found"}} = delete("no-parent")
    assert :error = AnalysisResultStore.get("no-parent")
    job = plain_job()
    assert {404, %{"error" => "result_not_found"}} = patch(job.job_id, %{"value" => 1})
    assert {404, %{"error" => "result_not_found"}} = delete(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "ordinary result editing remains available, but cannot resurrect a deleted result" do
    job = plain_job()
    assert :ok = AnalysisResultStore.put(job.job_id, %{"value" => 1, "workflow_id" => nil})

    replacement = %{
      "value" => 2,
      "workflow_id" => nil,
      "recovery" => %{"solver_note" => "public solver data"}
    }

    assert {200, %{"result" => ^replacement}} = patch(job.job_id, replacement)
    assert {200, %{"result" => ^replacement, "deleted" => true}} = delete(job.job_id)
    assert {404, %{"error" => "result_not_found"}} = patch(job.job_id, replacement)
    assert :error = AnalysisResultStore.get(job.job_id)
    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "the API cannot inject private workflow recovery state into an ordinary result" do
    job = plain_job()
    original = %{"value" => 1}
    assert :ok = AnalysisResultStore.put(job.job_id, original)
    forged = Map.put(original, WorkflowRecoveryEnvelope.internal_key(), %{"state" => "pending"})

    assert {422, %{"error" => ":workflow_recovery_metadata_is_read_only"}} =
             patch(job.job_id, forged)

    assert {:ok, ^original} = AnalysisResultStore.get(job.job_id)
  end

  test "GET then PATCH preserves terminal identity and derives the real recovery summary" do
    fixture = Fixture.terminal_job()
    conn = conn(:get, "/api/v1/results/#{fixture.id}") |> Router.call(@opts)
    assert conn.status == 200
    original = Jason.decode!(conn.resp_body)["result"]

    replacement =
      original
      |> Map.put("reviewed", true)
      |> Map.put("workflow_id", "forged-workflow")
      |> Map.put("recovery", %{"state" => "running", "generation" => 999})

    assert {200, %{"result" => result}} = patch(fixture.id, replacement)
    assert result["reviewed"] == true
    assert result["workflow_id"] == original["workflow_id"]
    assert result["recovery"] == original["recovery"]
    key = WorkflowRecoveryEnvelope.internal_key()
    assert {:ok, stored} = AnalysisResultStore.get(fixture.id)
    assert stored[key] == fixture.runtime[key]
    assert {:ok, job} = Store.get(fixture.id)
    assert job == fixture.job
  end

  test "active workflow runtime cannot be replaced or deleted through result routes" do
    fixture = Fixture.claimed_job()
    assert {422, %{"error" => ":active_workflow_result_is_read_only"}} = patch(fixture.id, %{})
    assert {422, %{"error" => ":active_workflow_result_is_read_only"}} = delete(fixture.id)
    Fixture.unchanged(fixture)
  end

  for value <- [nil, [], 42, true, "not an object"] do
    test "nonobject result #{inspect(value)} is rejected without changing either record" do
      job = plain_job()
      original = %{"value" => 1}
      assert :ok = AnalysisResultStore.put(job.job_id, original)
      assert {422, %{"error" => "missing_result_payload"}} = patch(job.job_id, unquote(value))
      assert {:ok, ^original} = AnalysisResultStore.get(job.job_id)
      assert {:ok, ^job} = Store.get(job.job_id)
    end
  end

  for value <- [%{}, %{"nested" => [nil, [], %{}, true, "x", 1.0e-300]}] do
    test "valid object result #{inspect(value)} produces a matching receipt without mutating its job" do
      job = plain_job()
      result = unquote(Macro.escape(value))
      assert :ok = AnalysisResultStore.put(job.job_id, %{"original" => true})
      assert {200, %{"job_id" => id, "result" => ^result}} = patch(job.job_id, result)
      assert id == job.job_id
      assert {:ok, ^result} = AnalysisResultStore.get(job.job_id)
      assert {:ok, ^job} = Store.get(job.job_id)
    end
  end

  test "confirmed result deletion retains its job and another result and cannot be repeated" do
    deleted = plain_job()
    retained = plain_job()
    assert :ok = AnalysisResultStore.put(deleted.job_id, %{"deleted" => true})
    assert :ok = AnalysisResultStore.put(retained.job_id, %{"retained" => true})

    assert {200, %{"job_id" => id, "result" => %{"deleted" => true}, "deleted" => true}} =
             delete(deleted.job_id)

    assert id == deleted.job_id
    assert {:ok, ^deleted} = Store.get(deleted.job_id)
    assert {:ok, ^retained} = Store.get(retained.job_id)
    assert {:ok, %{"retained" => true}} = AnalysisResultStore.get(retained.job_id)
    assert :error = AnalysisResultStore.get(deleted.job_id)
    assert {404, %{"error" => "result_not_found"}} = delete(deleted.job_id)
  end

  for action <- [:save, :delete] do
    test "confirmed result #{action} stays committed when follow-up history loses storage" do
      job = plain_job()
      assert :ok = AnalysisResultStore.put(job.job_id, %{"value" => 1})

      if unquote(action) == :save do
        assert {200, %{"result" => %{"value" => 2}}} = patch(job.job_id, %{"value" => 2})
        assert {:ok, %{"value" => 2}} = AnalysisResultStore.get(job.job_id)
      else
        assert {200, %{"deleted" => true}} = delete(job.job_id)
        assert :error = AnalysisResultStore.get(job.job_id)
      end

      assert {:ok, ^job} = Store.get(job.job_id)

      StorageOutageFixture.without(StorageOutageFixture.store_child(), fn ->
        conn = conn(:get, "/api/v1/results") |> Router.call(@opts)
        assert conn.status == 503
        assert %{"error" => "analysis_store_unavailable"} = Jason.decode!(conn.resp_body)
      end)
    end
  end

  defp patch(id, result) do
    conn =
      conn(:patch, "/api/v1/results/#{id}", Jason.encode!(%{"result" => result}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {conn.status, Jason.decode!(conn.resp_body)}
  end

  defp delete(id) do
    conn = conn(:delete, "/api/v1/results/#{id}") |> Router.call(@opts)
    {conn.status, Jason.decode!(conn.resp_body)}
  end

  defp plain_job do
    assert {:ok, job} =
             Store.create(%{
               job_id: "result-api-#{System.unique_integer([:positive])}",
               project_id: "results",
               simulation_case_id: "case"
             })

    job
  end
end
