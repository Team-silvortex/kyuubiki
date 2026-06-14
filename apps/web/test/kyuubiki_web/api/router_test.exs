defmodule KyuubikiWeb.Playground.RouterTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a two dimensional plane triangle job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving plane stress system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 1.0e-7, "uy" => 0.0},
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "ux" => 1.2e-7,
                "uy" => -2.0e-7
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "p0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "strain_x" => 1.0e-7,
                "strain_y" => -2.0e-7,
                "gamma_xy" => 0.0,
                "stress_x" => 5.0e3,
                "stress_y" => -3.0e3,
                "tau_xy" => 0.0,
                "von_mises" => 6.0e3
              }
            ],
            "max_displacement" => 2.3e-7,
            "max_stress" => 6.0e3,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/plane-triangle-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "p0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a plane quad job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.65,
            "iteration" => 2,
            "message" => "assembling quad plane system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 8.0e-8, "uy" => 0.0},
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "ux" => 1.1e-7,
                "uy" => -2.1e-7
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.0,
                "y" => 1.0,
                "ux" => 0.0,
                "uy" => -8.0e-8
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "q0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "strain_x" => 8.0e-8,
                "strain_y" => -2.1e-7,
                "gamma_xy" => 0.0,
                "stress_x" => 4.8e3,
                "stress_y" => -3.2e3,
                "tau_xy" => 0.0,
                "von_mises" => 6.4e3
              }
            ],
            "max_displacement" => 2.4e-7,
            "max_stress" => 6.4e3,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/plane-quad-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            },
            %{
              "id" => "n3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "q0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal plane triangle job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.6,
            "iteration" => 2,
            "message" => "assembling thermal plane triangle system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 0.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tp0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "temperature_delta" => 40.0,
                "average_temperature_delta" => 40.0,
                "thermal_strain_x" => 4.8e-4,
                "thermal_strain_y" => 4.8e-4,
                "mechanical_strain_x" => -4.8e-4,
                "mechanical_strain_y" => -4.8e-4,
                "mechanical_gamma_xy" => 0.0,
                "total_strain_x" => 0.0,
                "total_strain_y" => 0.0,
                "total_gamma_xy" => 0.0,
                "stress_x" => -5.0e7,
                "stress_y" => -5.0e7,
                "tau_xy" => 0.0,
                "von_mises" => 5.0e7
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 5.0e7,
            "max_temperature_delta" => 40.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-plane-triangle-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 40.0
            }
          ],
          "elements" => [
            %{
              "id" => "tp0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33,
              "thermal_expansion" => 12.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal plane quad job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.6,
            "iteration" => 2,
            "message" => "assembling thermal plane quad system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 1.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 3, "id" => "n3", "x" => 0.0, "y" => 1.0, "ux" => 0.0, "uy" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "temperature_delta" => 30.0,
                "average_temperature_delta" => 30.0,
                "thermal_strain_x" => 3.3e-4,
                "thermal_strain_y" => 3.3e-4,
                "mechanical_strain_x" => -3.3e-4,
                "mechanical_strain_y" => -3.3e-4,
                "mechanical_gamma_xy" => 0.0,
                "total_strain_x" => 0.0,
                "total_strain_y" => 0.0,
                "total_gamma_xy" => 0.0,
                "stress_x" => -3.4e7,
                "stress_y" => -3.4e7,
                "tau_xy" => 0.0,
                "von_mises" => 3.4e7
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 3.4e7,
            "max_temperature_delta" => 30.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-plane-quad-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n2",
              "x" => 1.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            },
            %{
              "id" => "n3",
              "x" => 0.0,
              "y" => 1.0,
              "fix_x" => true,
              "fix_y" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "temperature_delta" => 30.0
            }
          ],
          "elements" => [
            %{
              "id" => "tq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "youngs_modulus" => 7.0e10,
              "poisson_ratio" => 0.33,
              "thermal_expansion" => 11.0e-6
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 30.0
    assert length(result_payload["result"]["elements"]) == 1
  end
end
