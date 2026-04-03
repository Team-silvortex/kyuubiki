defmodule KyuubikiWeb.Playground.RouterTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Playground.AgentRegistry
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  @opts Router.init([])

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    Library.reset()
    Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)

    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Enum.each(AgentRegistry.agents(), fn agent -> AgentRegistry.unregister(agent.id) end)
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      AgentPool.reload()
    end)

    :ok
  end

  test "supports CRUD for projects, models, and model versions" do
    create_project_conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Bridge Study", "description" => "macOS local"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_project_conn.status == 201
    project = Jason.decode!(create_project_conn.resp_body)["project"]
    project_id = project["project_id"]

    create_model_conn =
      :post
      |> conn(
        "/api/v1/projects/#{project_id}/models",
        Jason.encode!(%{
          "name" => "Roof Truss",
          "kind" => "truss_2d",
          "material" => "Steel",
          "model_schema_version" => "kyuubiki.model/v1",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_model_conn.status == 201
    model = Jason.decode!(create_model_conn.resp_body)["model"]
    model_id = model["model_id"]
    assert model["latest_version_number"] == 1

    create_version_conn =
      :post
      |> conn(
        "/api/v1/models/#{model_id}/versions",
        Jason.encode!(%{
          "name" => "Checkpoint A",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss v2",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [%{"id" => "n0", "x" => 0.0, "y" => 0.0}],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_version_conn.status == 201
    version = Jason.decode!(create_version_conn.resp_body)["version"]
    assert version["version_number"] == 2

    projects_conn =
      :get
      |> conn("/api/v1/projects")
      |> Router.call(@opts)

    assert projects_conn.status == 200
    projects = Jason.decode!(projects_conn.resp_body)["projects"]
    assert length(projects) == 1
    assert hd(projects)["models"] |> length() == 1

    get_model_conn =
      :get
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert get_model_conn.status == 200
    returned_model = Jason.decode!(get_model_conn.resp_body)["model"]
    assert length(returned_model["versions"]) == 2

    update_project_conn =
      :patch
      |> conn(
        "/api/v1/projects/#{project_id}",
        Jason.encode!(%{"name" => "Bridge Study Updated"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_project_conn.status == 200

    assert Jason.decode!(update_project_conn.resp_body)["project"]["name"] ==
             "Bridge Study Updated"

    delete_version_conn =
      :delete
      |> conn("/api/v1/model-versions/#{version["version_id"]}")
      |> Router.call(@opts)

    assert delete_version_conn.status == 200

    delete_model_conn =
      :delete
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert delete_model_conn.status == 200

    delete_project_conn =
      :delete
      |> conn("/api/v1/projects/#{project_id}")
      |> Router.call(@opts)

    assert delete_project_conn.status == 200
  end

  test "cancels an active job through the API" do
    {:ok, job} =
      Store.create(%{
        job_id: "job-cancel",
        project_id: "project-cancel",
        simulation_case_id: "case-cancel"
      })

    assert job.status == :queued

    Store.apply_progress(%{
      job_id: "job-cancel",
      stage: "solving",
      progress: 0.6,
      message: "solving structural system"
    })

    cancel_conn =
      :post
      |> conn("/api/v1/jobs/job-cancel/cancel")
      |> Router.call(@opts)

    assert cancel_conn.status == 200

    payload = Jason.decode!(cancel_conn.resp_body)
    assert payload["job"]["status"] == "cancelled"
    assert payload["job"]["message"] == "job cancelled by operator"

    assert {:ok, cancelled_job} = Store.get("job-cancel")
    assert cancelled_job.status == :cancelled
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

    result_payload = wait_for_job(job_id)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["job"]["worker_id"] == "rust-agent-rpc@agent-a"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "binds a submitted job to the selected model version" do
    {:ok, project} = Library.create_project(%{"name" => "Version-bound study"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Saved roof",
        "kind" => "truss_2d",
        "model_schema_version" => "kyuubiki.model/v1",
        "payload" => %{
          "kind" => "truss_2d",
          "model_schema_version" => "kyuubiki.model/v1",
          "name" => "Saved roof",
          "material" => "Steel",
          "youngs_modulus_gpa" => 210,
          "nodes" => [],
          "elements" => []
        }
      })

    {:ok, version} =
      Library.create_version(model["model_id"], %{
        "name" => "Frozen solve input",
        "kind" => "truss_2d",
        "model_schema_version" => "kyuubiki.model/v1",
        "payload" => %{
          "kind" => "truss_2d",
          "model_schema_version" => "kyuubiki.model/v1",
          "name" => "Saved roof",
          "material" => "Steel",
          "youngs_modulus_gpa" => 210,
          "nodes" => [],
          "elements" => []
        }
      })

    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [],
            "elements" => [],
            "max_displacement" => 0.0,
            "max_stress" => 0.0,
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
        "/api/v1/fem/truss-2d/jobs",
        Jason.encode!(%{
          "project_id" => "ignored-project-id",
          "model_version_id" => version["version_id"],
          "nodes" => [],
          "elements" => []
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    assert payload["job"]["project_id"] == project["project_id"]
    assert payload["job"]["model_version_id"] == version["version_id"]
    assert payload["job"]["simulation_case_id"] == version["version_id"]
  end

  test "runs a two dimensional truss job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving structural system"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "y" => 0.0, "ux" => 0.0, "uy" => 0.0},
              %{"index" => 2, "id" => "n2", "x" => 0.5, "y" => 0.75, "ux" => 0.0, "uy" => -1.0e-6}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "e0",
                "node_i" => 0,
                "node_j" => 2,
                "length" => 0.9,
                "strain" => 1.0e-6,
                "stress" => 7.0e4,
                "axial_force" => 700.0
              }
            ],
            "max_displacement" => 1.0e-6,
            "max_stress" => 7.0e4,
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
        "/api/v1/fem/truss-2d/jobs",
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
              "x" => 0.5,
              "y" => 0.75,
              "fix_x" => false,
              "fix_y" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "e0",
              "node_i" => 0,
              "node_j" => 2,
              "area" => 0.01,
              "youngs_modulus" => 7.0e10
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 3
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
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a three dimensional truss job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 3,
            "message" => "solving spatial truss system"
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
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 0.0,
                "y" => 1.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => 0.0
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.2,
                "y" => 0.2,
                "z" => 1.0,
                "ux" => 0.0,
                "uy" => 0.0,
                "uz" => -1.0e-6
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "e0",
                "node_i" => 0,
                "node_j" => 3,
                "length" => 1.0,
                "strain" => 1.0e-6,
                "stress" => 7.0e4,
                "axial_force" => 700.0
              }
            ],
            "max_displacement" => 1.0e-6,
            "max_stress" => 7.0e4,
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
        "/api/v1/fem/truss-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 1.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n2",
              "x" => 0.0,
              "y" => 1.0,
              "z" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_z" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0
            },
            %{
              "id" => "n3",
              "x" => 0.2,
              "y" => 0.2,
              "z" => 1.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_z" => false,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => -1000.0
            }
          ],
          "elements" => [
            %{
              "id" => "e0",
              "node_i" => 0,
              "node_j" => 3,
              "area" => 0.01,
              "youngs_modulus" => 7.0e10
            }
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 4
  end

  test "exposes orchestrator health" do
    conn =
      :get
      |> conn("/api/health")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)

    assert payload["service"] == "kyuubiki-orchestrator"
    assert payload["status"] == "ok"
    assert payload["deployment"]["mode"] == "local"
    assert payload["deployment"]["discovery"] == "static"
    assert payload["remote_solver_registry"]["active_agents"] == 0
    assert payload["transport"]["http"] == 4000
    assert payload["transport"]["solver_agent_tcp"] == 5001
  end

  test "registers, heartbeats, and removes remote agents through the API" do
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [])
    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "solver-remote-a",
          "host" => "10.20.0.11",
          "port" => 6101,
          "region" => "ap-shanghai"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 201
    payload = Jason.decode!(conn.resp_body)
    assert payload["agent"]["id"] == "solver-remote-a"

    conn =
      :post
      |> conn(
        "/api/v1/agents/solver-remote-a/heartbeat",
        Jason.encode!(%{
          "host" => "10.20.0.11",
          "port" => 6101,
          "zone" => "rack-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["agent"]["zone"] == "rack-a"

    conn =
      :get
      |> conn("/api/v1/agents")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["summary"]["active_agents"] == 1
    assert Enum.map(payload["agents"], & &1["id"]) == ["solver-remote-a"]

    conn =
      :delete
      |> conn("/api/v1/agents/solver-remote-a")
      |> Router.call(@opts)

    assert conn.status == 200
    assert Jason.decode!(conn.resp_body)["status"] == "removed"
  end

  test "surfaces solver failure messages through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => false,
          "error" => %{
            "code" => "solver_failed",
            "message" =>
              "truss response exceeds the small-deformation limit; check supports or connectivity"
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
        "/api/v1/fem/truss-2d/jobs",
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
            }
          ],
          "elements" => []
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = wait_for_job(payload["job"]["job_id"])

    assert result_payload["job"]["status"] == "failed"
    assert result_payload["job"]["message"] =~ "small-deformation limit"
  end

  test "lists persisted jobs in reverse chronological order" do
    {:ok, _job_1} =
      Store.create(%{
        job_id: "job-old",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    {:ok, _job_1_completed} =
      Store.apply_progress(%{
        job_id: "job-old",
        stage: "completed",
        progress: 1.0,
        iteration: 5,
        residual: 1.0e-3
      })

    :ok =
      AnalysisResultStore.put("job-old", %{
        "kind" => "axial_bar_1d",
        "max_displacement" => 1.0e-6
      })

    Process.sleep(5)

    {:ok, _job_2} =
      Store.create(%{
        job_id: "job-new",
        project_id: "project-2",
        simulation_case_id: "case-2"
      })

    {:ok, _job_2_updated} =
      Store.apply_progress(%{
        job_id: "job-new",
        stage: "solving",
        progress: 0.5,
        iteration: 2,
        residual: 5.0e-1
      })

    conn =
      :get
      |> conn("/api/v1/jobs")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)
    jobs = payload["jobs"]

    assert Enum.map(jobs, & &1["job_id"]) == ["job-new", "job-old"]

    assert Enum.at(jobs, 0)["status"] == "solving"
    assert Enum.at(jobs, 0)["has_result"] == false
    assert Enum.at(jobs, 1)["status"] == "completed"
    assert Enum.at(jobs, 1)["has_result"] == true
    assert is_binary(Enum.at(jobs, 0)["updated_at"])
    assert is_binary(Enum.at(jobs, 1)["created_at"])
  end

  test "supports CRUD and export for persisted jobs and results" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-admin",
        project_id: "project-admin",
        simulation_case_id: "case-admin",
        message: "queued"
      })

    :ok =
      AnalysisResultStore.put("job-admin", %{
        "kind" => "truss_2d",
        "max_displacement" => 2.0e-6
      })

    update_job_conn =
      :patch
      |> conn("/api/v1/jobs/job-admin", Jason.encode!(%{"message" => "reviewed"}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_job_conn.status == 200
    assert Jason.decode!(update_job_conn.resp_body)["job"]["message"] == "reviewed"

    list_results_conn =
      :get
      |> conn("/api/v1/results")
      |> Router.call(@opts)

    assert list_results_conn.status == 200
    assert [%{"job_id" => "job-admin"}] = Jason.decode!(list_results_conn.resp_body)["results"]

    update_result_conn =
      :patch
      |> conn("/api/v1/results/job-admin", Jason.encode!(%{"result" => %{"kind" => "truss_2d", "max_displacement" => 4.0e-6}}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_result_conn.status == 200
    assert Jason.decode!(update_result_conn.resp_body)["result"]["max_displacement"] == 4.0e-6

    export_conn =
      :get
      |> conn("/api/v1/export/database")
      |> Router.call(@opts)

    assert export_conn.status == 200
    export_payload = Jason.decode!(export_conn.resp_body)
    assert [%{"job_id" => "job-admin"}] = export_payload["jobs"]
    assert [%{"job_id" => "job-admin"}] = export_payload["results"]
    assert is_list(export_payload["projects"])
    assert is_list(export_payload["models"])
    assert is_list(export_payload["model_versions"])

    delete_result_conn =
      :delete
      |> conn("/api/v1/results/job-admin")
      |> Router.call(@opts)

    assert delete_result_conn.status == 200

    delete_job_conn =
      :delete
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert delete_job_conn.status == 200

    missing_job_conn =
      :get
      |> conn("/api/v1/jobs/job-admin")
      |> Router.call(@opts)

    assert missing_job_conn.status == 422
  end

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end

  defp wait_for_job(job_id, attempts \\ 20)

  defp wait_for_job(job_id, attempts) when attempts > 0 do
    conn =
      :get
      |> conn("/api/v1/jobs/#{job_id}")
      |> Router.call(@opts)

    payload = Jason.decode!(conn.resp_body)

    if payload["job"]["status"] in ["completed", "failed"] do
      payload
    else
      Process.sleep(10)
      wait_for_job(job_id, attempts - 1)
    end
  end

  defp wait_for_job(_job_id, 0), do: flunk("timed out waiting for async job completion")
end
