defmodule KyuubikiWeb.TaskDigestV2ApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.TaskDigestV2

  defp task do
    task =
      Path.expand("../../../../../schemas/examples.operator-task-ir.json", __DIR__)
      |> File.read!()
      |> Jason.decode!()
      |> Map.put("config", %{"nested" => [%{"value" => 1.0e-30}]})

    {:ok, task} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
    task
  end

  defp prepare(task) do
    :post
    |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  test "API accepts explicit typed encoding and reports its computed identity" do
    task = task()
    conn = prepare(task)
    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["task_digest"] == task["integrity"]["task_digest"]
  end

  test "API rejects tiny config drift instead of promoting a rounded identity" do
    conn = prepare(put_in(task(), ["config", "nested"], [%{"value" => 2.0e-30}]))
    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["error_code"] == "operator_task_digest_mismatch"
  end

  test "API encoding errors are explicit and the next valid task still succeeds" do
    for {field, value} <- [
          {"task_digest_encoding", "future"},
          {"task_digest_encoding", nil},
          {"digest_algorithm", "sha512"}
        ] do
      conn = prepare(put_in(task(), ["integrity", field], value))
      assert conn.status == 422
      assert Jason.decode!(conn.resp_body)["error_code"] == "operator_task_digest_invalid"
    end

    assert prepare(task()).status == 200
  end
end
