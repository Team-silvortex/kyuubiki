defmodule KyuubikiWeb.Api.ElectricConductionSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @voltage_v 3.36e-5
  @conductivity_s_m 59_523_809.52380952
  @current_density_a_m2 66_666.66666666667
  @joule_power_w 6.72e-5

  test "runs a voltage-driven conductor through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link(
        {:capture, self(),
         [
           solver_progress(),
           %{"ok" => true, "result" => solved_result()}
         ]}
      )

    configure_solver_agent()

    conn =
      :post
      |> conn(
        "/api/v1/fem/electric-conduction-plane-quad-2d/jobs",
        Jason.encode!(request())
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert_receive {:fake_agent_request, rpc_request}, 1_000
    assert rpc_request["method"] == "solve_electric_conduction_plane_quad_2d"
    assert rpc_request["params"]["elements"] == request()["elements"]

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)
    result = result_payload["result"]
    [element] = result["elements"]

    assert result_payload["job"]["status"] == "completed"
    assert_in_delta result["max_electric_field_v_m"], 0.00112, 1.0e-12
    assert_in_delta result["max_current_density_a_m2"], @current_density_a_m2, 1.0e-8
    assert_in_delta result["total_joule_power_w"], @joule_power_w, 1.0e-15
    assert_in_delta element["volumetric_joule_heating_w_m3"], 74.66666666666667, 1.0e-12
    assert_in_delta element["joule_power_w"], @joule_power_w, 1.0e-15
  end

  defp request do
    %{
      "nodes" => [
        node("n0", 0.0, 0.0, 0.0),
        node("n1", 0.03, 0.0, @voltage_v),
        node("n2", 0.03, 0.03, @voltage_v),
        node("n3", 0.0, 0.03, 0.0)
      ],
      "elements" => [
        %{
          "id" => "conductor",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.001,
          "electrical_conductivity_s_m" => @conductivity_s_m
        }
      ]
    }
  end

  defp solved_result do
    %{
      "input" => request(),
      "nodes" => [],
      "elements" => [
        %{
          "index" => 0,
          "id" => "conductor",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "area_m2" => 0.0009,
          "average_electric_potential_v" => @voltage_v / 2.0,
          "electric_potential_gradient_x_v_m" => 0.00112,
          "electric_potential_gradient_y_v_m" => 0.0,
          "electric_field_x_v_m" => -0.00112,
          "electric_field_y_v_m" => 0.0,
          "electric_field_magnitude_v_m" => 0.00112,
          "current_density_x_a_m2" => -@current_density_a_m2,
          "current_density_y_a_m2" => 0.0,
          "current_density_magnitude_a_m2" => @current_density_a_m2,
          "volumetric_joule_heating_w_m3" => 74.66666666666667,
          "joule_power_w" => @joule_power_w
        }
      ],
      "max_electric_potential_v" => @voltage_v,
      "max_electric_field_v_m" => 0.00112,
      "max_current_density_a_m2" => @current_density_a_m2,
      "total_joule_power_w" => @joule_power_w
    }
  end

  defp node(id, x, y, potential_v) do
    %{
      "id" => id,
      "x" => x,
      "y" => y,
      "fix_electric_potential" => true,
      "electric_potential_v" => potential_v,
      "current_source_a" => 0.0
    }
  end

  defp configure_solver_agent do
    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end

  defp solver_progress do
    %{
      "event" => "progress",
      "progress" => %{
        "job_id" => "solver-session",
        "stage" => "solving",
        "progress" => 0.5,
        "iteration" => 1,
        "message" => "solving electric conduction plane quad"
      }
    }
  end
end
