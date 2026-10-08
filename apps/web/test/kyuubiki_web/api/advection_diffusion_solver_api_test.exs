defmodule KyuubikiWeb.Api.AdvectionDiffusionSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @path "/api/v1/fem/advection-diffusion-bar-1d/jobs"
  @input %{
    "nodes" => [
      %{"x" => 0.0, "fix_concentration" => true, "concentration" => 2.0},
      %{"x" => 0.5, "fix_concentration" => false, "source" => 4.0},
      %{"x" => 1.0, "fix_concentration" => true, "concentration" => 2.0}
    ],
    "elements" => [
      %{"node_i" => 0, "node_j" => 1, "area" => 1.0, "diffusivity" => 1.0, "velocity" => 0.5},
      %{"node_i" => 1, "node_j" => 2, "area" => 1.0, "diffusivity" => 1.0, "velocity" => 0.5}
    ]
  }

  test "route normalizes identifiers and dispatches the transport RPC" do
    result = %{"nodes" => [%{"id" => "n1", "concentration" => 3.0}], "max_total_flux" => 3.25}

    {:ok, _} =
      FakePlaygroundAgent.start_link({:capture, self(), [%{"ok" => true, "result" => result}]})

    WorkflowApi.configure_fake_agent_pool(await_fake_agent_port())
    response = submit(@input)
    assert response.status == 202
    job_id = Jason.decode!(response.resp_body)["job"]["job_id"]
    terminal = WorkflowApi.wait_for_job(job_id, @opts)
    assert terminal["job"]["status"] == "completed"
    assert terminal["result"] == result
    assert_receive {:fake_agent_request, request}
    assert request["method"] == "solve_advection_diffusion_bar_1d"
    assert Enum.map(request["params"]["nodes"], & &1["id"]) == ["n0", "n1", "n2"]
    assert Enum.map(request["params"]["elements"], & &1["id"]) == ["e0", "e1"]
    assert request["params"]["nodes"] |> Enum.at(1) |> Map.get("source") == 4.0
  end

  test "invalid graph fails before any job admission" do
    for input <- [
          %{"nodes" => "not-a-graph", "elements" => []},
          put_in(@input, ["nodes", Access.at(1), "id"], "n0"),
          put_in(@input, ["elements", Access.at(0), "id"], nil)
        ] do
      assert submit(input).status == 422
    end

    response = conn(:get, "/api/v1/jobs") |> Router.call(@opts)
    assert Jason.decode!(response.resp_body)["jobs"] == []
  end

  test "protocol and workflow catalogs advertise the same executable operator" do
    assert "solve_advection_diffusion_bar_1d" in KyuubikiWeb.Protocol.solver_rpc_protocol()[
             "methods"
           ]

    response =
      conn(:get, "/api/v1/operators/solve.advection_diffusion_bar_1d") |> Router.call(@opts)

    assert response.status == 200
    operator = Jason.decode!(response.resp_body)["operator"]
    assert operator["domain"] == "transport"
    assert operator["family"] == "advection_diffusion_bar_1d"
    assert hd(operator["inputs"])["artifact_type"] == "model/advection_diffusion_bar_1d"
    assert hd(operator["outputs"])["artifact_type"] == "result/advection_diffusion_bar_1d"
  end

  test "upwind scheme is retained and invalid choices create no jobs" do
    for scheme <- [nil, true, %{}, "auto", "UPWIND"] do
      assert submit(Map.put(@input, "scheme", scheme)).status == 422
    end

    response = conn(:get, "/api/v1/jobs") |> Router.call(@opts)
    assert Jason.decode!(response.resp_body)["jobs"] == []

    {:ok, _} =
      FakePlaygroundAgent.start_link({:capture, self(), [%{"ok" => true, "result" => %{}}]})

    WorkflowApi.configure_fake_agent_pool(await_fake_agent_port())
    response = submit(Map.put(@input, "scheme", "upwind"))
    assert response.status == 202
    job_id = Jason.decode!(response.resp_body)["job"]["job_id"]
    assert WorkflowApi.wait_for_job(job_id, @opts)["job"]["status"] == "completed"
    assert_receive {:fake_agent_request, request}
    assert request["params"]["scheme"] == "upwind"
  end

  defp submit(input) do
    conn(:post, @path, Jason.encode!(input))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end
end
