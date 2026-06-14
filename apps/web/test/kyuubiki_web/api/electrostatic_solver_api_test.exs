defmodule KyuubikiWeb.Api.ElectrostaticSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs an electrostatic bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        solver_progress("solving electrostatic bar"),
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "eb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "average_potential" => 5.0,
                "potential_gradient" => -10.0,
                "electric_field" => 10.0,
                "electric_flux_density" => 20.0
              }
            ],
            "max_potential" => 10.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    configure_solver_agent()

    conn =
      :post
      |> conn(
        "/api/v1/fem/electrostatic-bar-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_potential" => true,
              "potential" => 10.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "fix_potential" => true,
              "potential" => 0.0,
              "charge_density" => 0.0
            }
          ],
          "elements" => [
            %{"id" => "eb0", "node_i" => 0, "node_j" => 1, "area" => 0.02, "permittivity" => 2.0}
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)
    result = result_payload["result"]
    [element] = result["elements"]

    assert result_payload["job"]["status"] == "completed"
    assert result["max_potential"] == 10.0
    assert result["max_electric_field"] == 10.0
    assert result["max_flux_density"] == 20.0
    assert length(result["nodes"]) == 2
    assert element["average_potential"] == 5.0
    assert element["potential_gradient"] == -10.0
    assert element["electric_field"] == 10.0
    assert element["electric_flux_density"] == 20.0
  end

  test "runs an electrostatic plane triangle job through the orchestration API with benchmarked field outputs" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        solver_progress("solving electrostatic plane triangle"),
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "y" => 0.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 0.0,
                "y" => 1.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "ep0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "average_potential" => 6.666666666666667,
                "potential_gradient_x" => -10.0,
                "potential_gradient_y" => 0.0,
                "electric_field_x" => 10.0,
                "electric_field_y" => 0.0,
                "electric_field_magnitude" => 10.0,
                "electric_flux_density_x" => 20.0,
                "electric_flux_density_y" => 0.0,
                "electric_flux_density_magnitude" => 20.0
              }
            ],
            "max_potential" => 10.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    configure_solver_agent()

    conn =
      :post
      |> conn(
        "/api/v1/fem/electrostatic-plane-triangle-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_potential" => true,
              "potential" => 10.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_potential" => true,
              "potential" => 0.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "fix_potential" => true,
              "potential" => 10.0,
              "charge_density" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "ep0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)
    result = result_payload["result"]
    [element] = result["elements"]

    assert result_payload["job"]["status"] == "completed"
    assert result["max_potential"] == 10.0
    assert result["max_electric_field"] == 10.0
    assert result["max_flux_density"] == 20.0
    assert Enum.map(result["nodes"], & &1["potential"]) == [10.0, 0.0, 10.0]
    assert element["area"] == 0.5
    assert element["average_potential"] == 6.666666666666667
    assert element["potential_gradient_x"] == -10.0
    assert element["potential_gradient_y"] == 0.0
    assert element["electric_field_x"] == 10.0
    assert element["electric_field_y"] == 0.0
    assert element["electric_field_magnitude"] == 10.0
    assert element["electric_flux_density_x"] == 20.0
    assert element["electric_flux_density_y"] == 0.0
    assert element["electric_flux_density_magnitude"] == 20.0
  end

  test "runs an electrostatic plane quad job through the orchestration API with benchmarked field outputs" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        solver_progress("solving electrostatic plane quad"),
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "epq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "average_potential" => 5.0,
                "potential_gradient_x" => -10.0,
                "potential_gradient_y" => 0.0,
                "electric_field_x" => 10.0,
                "electric_field_y" => 0.0,
                "electric_field_magnitude" => 10.0,
                "electric_flux_density_x" => 20.0,
                "electric_flux_density_y" => 0.0,
                "electric_flux_density_magnitude" => 20.0
              }
            ],
            "max_potential" => 10.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    configure_solver_agent()

    conn =
      :post
      |> conn(
        "/api/v1/fem/electrostatic-plane-quad-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_potential" => true,
              "potential" => 10.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_potential" => true,
              "potential" => 0.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_potential" => true,
              "potential" => 0.0,
              "charge_density" => 0.0
            },
            %{
              "id" => "n3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_potential" => true,
              "potential" => 10.0,
              "charge_density" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "epq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)
    result = result_payload["result"]
    [element] = result["elements"]

    assert result_payload["job"]["status"] == "completed"
    assert result["max_potential"] == 10.0
    assert result["max_electric_field"] == 10.0
    assert result["max_flux_density"] == 20.0
    assert Enum.map(result["nodes"], & &1["potential"]) == [10.0]
    assert element["area"] == 1.0
    assert element["average_potential"] == 5.0
    assert element["potential_gradient_x"] == -10.0
    assert element["potential_gradient_y"] == 0.0
    assert element["electric_field_x"] == 10.0
    assert element["electric_field_y"] == 0.0
    assert element["electric_field_magnitude"] == 10.0
    assert element["electric_flux_density_x"] == 20.0
    assert element["electric_flux_density_y"] == 0.0
    assert element["electric_flux_density_magnitude"] == 20.0
  end

  defp configure_solver_agent do
    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end

  defp solver_progress(message) do
    %{
      "event" => "progress",
      "progress" => %{
        "job_id" => "solver-session",
        "stage" => "solving",
        "progress" => 0.5,
        "iteration" => 1,
        "message" => message
      }
    }
  end
end
