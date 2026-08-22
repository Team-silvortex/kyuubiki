defmodule KyuubikiWeb.Api.HeadlessHandoffApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "registers, lists, reads, and snapshots a handoff" do
    payload = handoff_payload()

    create =
      conn(:post, "/api/v1/headless/handoff", Jason.encode!(payload))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create.status == 201
    receipt = Jason.decode!(create.resp_body)
    assert receipt["accepted"]
    assert receipt["workflow_id"] == "workflow-test"
    assert receipt["step_count"] == 1
    handoff_id = receipt["handoff_id"]

    status = conn(:get, "/api/v1/headless/handoff/#{handoff_id}") |> Router.call(@opts)
    assert status.status == 200
    assert Jason.decode!(status.resp_body)["stage"] == "received"

    list = conn(:get, "/api/v1/headless/handoff") |> Router.call(@opts)
    assert [%{"handoff_id" => ^handoff_id}] = Jason.decode!(list.resp_body)["handoffs"]

    snapshot =
      conn(:get, "/api/v1/headless/handoff/#{handoff_id}/snapshot")
      |> Router.call(@opts)

    assert snapshot.status == 200
    assert Jason.decode!(snapshot.resp_body)["envelope"] == payload
  end

  test "rejects malformed handoff envelopes" do
    response =
      conn(:post, "/api/v1/headless/handoff", Jason.encode!(%{"workflow_id" => "missing"}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 400
    assert Jason.decode!(response.resp_body)["error"] == "invalid_handoff"
  end

  defp handoff_payload do
    %{
      "schema_version" => "kyuubiki.headless-orchestra-handoff/v1",
      "generated_at" => "2026-08-22T00:00:00Z",
      "workflow_id" => "workflow-test",
      "execution_batch" => %{"steps" => [%{"step_key" => "solve"}]},
      "dispatch_plan" => %{
        "steps" => [%{"step_key" => "solve", "chosen_agent_id" => "agent-1"}],
        "warnings" => []
      },
      "governance" => %{"config" => %{}, "diagnostics" => %{}},
      "runtime_manifest" => %{
        "authority_mode" => "single_orchestrator",
        "source_of_truth" => "central_orchestrator_library",
        "agent_library_replication" => "forbidden",
        "target_clusters" => ["local"],
        "target_runtime_modes" => ["local"]
      }
    }
  end
end
