defmodule KyuubikiWeb.Api.TrussSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

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
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 3
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
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["nodes"]) == 4
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
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "failed"
    assert result_payload["job"]["message"] =~ "small-deformation limit"
  end
end
