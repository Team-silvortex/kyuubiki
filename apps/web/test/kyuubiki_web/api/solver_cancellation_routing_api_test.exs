defmodule KyuubikiWeb.Api.SolverCancellationRoutingApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Playground.AgentExecutionGate
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  test "public cancellation reaches the executing agent rather than an idle pool peer" do
    endpoints = Enum.map(["owner", "bystander"], &start_agent/1)
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
    AgentPool.reload()

    response =
      conn(:post, "/api/v1/fem/truss-2d/jobs", Jason.encode!(%{"nodes" => [], "elements" => []}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202
    job_id = Jason.decode!(response.resp_body)["job"]["job_id"]
    assert_receive {:agent_request, "owner", solver, %{"method" => "solve_truss_2d"}}, 2_000

    cancelled = conn(:post, "/api/v1/jobs/#{job_id}/cancel") |> Router.call(@opts)
    assert cancelled.status == 200

    assert_receive {:agent_request, "owner", _,
                    %{"method" => "cancel_job", "params" => %{"job_id" => ^job_id}}},
                   1_000

    refute_receive {:agent_request, "bystander", _, _}, 50

    receipt = Jason.decode!(cancelled.resp_body)
    assert receipt["job"]["status"] == "cancelled"
    assert receipt["cancellation"]["status"] == "requested"
    refute receipt["cancellation"]["execution_terminal_confirmed"]
    assert receipt["cancellation"]["targets"] |> hd() |> Map.get("agent_id") == "owner"
    assert AgentExecutionGate.snapshot().active_lease_count == 1

    send(solver, {:reply, %{"ok" => true, "result" => %{"nodes" => [], "elements" => []}}})
    await_released()
    assert :error = AnalysisResultStore.get(job_id)
    assert {:ok, %{status: :cancelled}} = Store.get(job_id)
  end

  defp start_agent(name) do
    start_supervised!({CancellableSolverAgent, {self(), name}})
    assert_receive {:cancellable_agent_ready, ^name, port}
    %{id: name, host: "127.0.0.1", port: port, capacity: 1}
  end

  defp await_released(attempts \\ 200)
  defp await_released(0), do: flunk("cancelled solver did not release its capacity lease")

  defp await_released(attempts) do
    if AgentExecutionGate.snapshot().active_lease_count != 0 do
      Process.sleep(10)
      await_released(attempts - 1)
    end
  end
end
