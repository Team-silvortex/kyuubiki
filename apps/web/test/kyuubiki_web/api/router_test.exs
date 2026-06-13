defmodule KyuubikiWeb.Playground.RouterTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "rejects mutating routes without an API token when control-plane protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Locked Project", "description" => "protected"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "accepts mutating routes with a valid bearer token when control-plane protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Protected Project", "description" => "authorized"})
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("authorization", "Bearer secret-token")
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["project"]["name"] == "Protected Project"
  end

  test "rejects read routes without an API token when read protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: true
    )

    conn =
      :get
      |> conn("/api/v1/projects")
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "accepts read routes with a valid token when read protection is enabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: true
    )

    conn =
      :get
      |> conn("/api/health")
      |> put_req_header("x-kyuubiki-token", "secret-token")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["status"] == "ok"
  end

  test "keeps read routes open when read protection is disabled" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "secret-token",
      protect_reads?: false
    )

    conn =
      :get
      |> conn("/api/v1/export/database")
      |> Router.call(@opts)

    assert conn.status == 200
    assert is_map(Jason.decode!(conn.resp_body))
  end

  test "allows loopback mutating routes without a configured API token" do
    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Local Project", "description" => "loopback allowed"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["project"]["name"] == "Local Project"
  end

  test "rejects non-loopback mutating routes without a configured API token" do
    conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Remote Project", "description" => "should fail"})
      )
      |> put_req_header("content-type", "application/json")
      |> Map.put(:remote_ip, {203, 0, 113, 9})
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "unauthorized"
  end

  test "runs an axial bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving axial system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "tip_displacement" => 4.761904761904762e-7,
            "reaction_force" => -1000.0,
            "max_displacement" => 4.761904761904762e-7,
            "max_stress" => 100_000.0,
            "nodes" => [
              %{"index" => 0, "x" => 0.0, "displacement" => 0.0},
              %{"index" => 1, "x" => 1.0, "displacement" => 4.761904761904762e-7}
            ],
            "elements" => [
              %{
                "index" => 0,
                "x1" => 0.0,
                "x2" => 1.0,
                "strain" => 4.761904761904762e-7,
                "stress" => 100_000.0,
                "axial_force" => 1000.0
              }
            ],
            "input" => %{
              "length" => 1.0,
              "area" => 0.01,
              "youngs_modulus" => 210.0e9,
              "elements" => 1,
              "tip_force" => 1000.0
            }
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
        "/api/v1/fem/axial-bar/jobs",
        Jason.encode!(%{
          "length" => 1.0,
          "area" => 0.01,
          "youngs_modulus_gpa" => 210,
          "elements" => 3,
          "tip_force" => 1000
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    job_id = payload["job"]["job_id"]

    assert payload["job"]["status"] == "queued"

    result_payload = WorkflowApi.wait_for_job(job_id, @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["job"]["worker_id"] == "rust-agent-rpc@agent-a"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.65,
            "iteration" => 2,
            "message" => "solving thermal bar"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "ux" => 0.0, "temperature_delta" => 40.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "ux" => 0.0, "temperature_delta" => 40.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "average_temperature_delta" => 40.0,
                "thermal_strain" => 4.8e-4,
                "mechanical_strain" => -4.8e-4,
                "total_strain" => 0.0,
                "stress" => -1.008e8,
                "axial_force" => -1.008e6
              }
            ],
            "max_displacement" => 0.0,
            "max_stress" => 1.008e8,
            "max_axial_force" => 1.008e6,
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
        "/api/v1/fem/thermal-bar-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_x" => true,
              "load_x" => 0.0,
              "temperature_delta" => 40.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "fix_x" => true,
              "load_x" => 0.0,
              "temperature_delta" => 40.0
            }
          ],
          "elements" => [
            %{
              "id" => "tb0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.01,
              "youngs_modulus" => 2.1e11,
              "thermal_expansion" => 1.2e-5
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
    assert result_payload["result"]["max_axial_force"] > 0
    assert result_payload["result"]["max_temperature_delta"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a heat bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving heat bar"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "temperature" => 100.0,
                "heat_load" => 0.0
              },
              %{"index" => 1, "id" => "n1", "x" => 1.0, "temperature" => 0.0, "heat_load" => 0.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "hb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "average_temperature" => 50.0,
                "temperature_gradient" => -100.0,
                "heat_flux" => 5000.0
              }
            ],
            "max_temperature" => 100.0,
            "max_heat_flux" => 5000.0,
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
        "/api/v1/fem/heat-bar-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_temperature" => true,
              "temperature" => 100.0,
              "heat_load" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "fix_temperature" => true,
              "temperature" => 0.0,
              "heat_load" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "hb0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "conductivity" => 50.0
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
    assert result_payload["result"]["max_temperature"] == 100.0
    assert result_payload["result"]["max_heat_flux"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs an electrostatic bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving electrostatic bar"
          }
        },
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

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

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
            %{
              "id" => "eb0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
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

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_potential"] == 10.0
    assert result_payload["result"]["max_electric_field"] > 0
    assert result_payload["result"]["max_flux_density"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs an electrostatic plane triangle job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving electrostatic plane triangle"
          }
        },
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

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

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

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_potential"] == 10.0
    assert result_payload["result"]["max_electric_field"] > 0
    assert result_payload["result"]["max_flux_density"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs an electrostatic plane quad job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving electrostatic plane quad"
          }
        },
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

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

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

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_potential"] == 10.0
    assert result_payload["result"]["max_electric_field"] > 0
    assert result_payload["result"]["max_flux_density"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end


  test "serves chunked result windows for large payloads" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-chunked",
        project_id: "project-chunked",
        simulation_case_id: "case-chunked"
      })

    :ok =
      AnalysisResultStore.put("job-chunked", %{
        "nodes" => Enum.map(0..9, &%{"index" => &1, "id" => "n#{&1}"}),
        "elements" => Enum.map(0..4, &%{"index" => &1, "id" => "e#{&1}"}),
        "max_displacement" => 0.0,
        "max_stress" => 0.0
      })

    conn =
      :get
      |> conn("/api/v1/results/job-chunked/chunks/nodes?offset=3&limit=4")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    assert payload["kind"] == "nodes"
    assert payload["offset"] == 3
    assert payload["limit"] == 4
    assert payload["returned"] == 4
    assert payload["total"] == 10
    assert Enum.map(payload["items"], & &1["index"]) == [3, 4, 5, 6]
  end

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
