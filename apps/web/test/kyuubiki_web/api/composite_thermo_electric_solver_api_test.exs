defmodule KyuubikiWeb.Api.CompositeThermoElectricSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits every coupled payload block to the Rust agent" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link(
        {:capture, self(), [%{"ok" => true, "result" => solved_result()}]}
      )

    configure_solver_agent()
    request = request()

    conn =
      :post
      |> conn(
        "/api/v1/fem/composite-thermo-electric-panel/jobs",
        Jason.encode!(request)
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert_receive {:fake_agent_request, rpc_request}, 1_000
    assert rpc_request["method"] == "solve_composite_thermo_electric_panel"
    assert rpc_request["params"] == request

    submission = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(submission["job"]["job_id"], @opts)
    assert result_payload["job"]["status"] == "completed"

    assert result_payload["result"]["schema_version"] ==
             "kyuubiki.composite-thermo-electric-panel-result/v1"
  end

  test "rejects an incomplete coupled payload before creating a job" do
    assert {:error, :invalid_composite_thermo_electric_panel_model} =
             KyuubikiWeb.FemModelNormalizer.normalize_composite_thermo_electric_panel(%{
               "electrostatic_model" => %{}
             })
  end

  defp request do
    %{
      "research" => %{"candidate_id" => "panel-a"},
      "electrostatic_model" => %{},
      "electric_conduction_model" => %{},
      "heat_model" => %{},
      "thermal_model" => %{},
      "electrothermal_loss" => %{},
      "electrothermal_feedback" => %{},
      "electric_conduction_feedback" => %{},
      "thermal_expansion_feedback" => %{}
    }
  end

  defp solved_result do
    %{
      "schema_version" => "kyuubiki.composite-thermo-electric-panel-result/v1",
      "research" => %{"candidate_id" => "panel-a"},
      "electrostatic" => %{},
      "electric_conduction" => %{},
      "heat" => %{},
      "thermal" => %{}
    }
  end

  defp configure_solver_agent do
    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end
end
