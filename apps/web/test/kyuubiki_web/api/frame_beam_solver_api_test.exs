defmodule KyuubikiWeb.Api.FrameBeamSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a frame 2d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling frame stiffness"
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
                "ux" => 0.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "y" => 0.0,
                "ux" => 0.0,
                "uy" => -0.0015873015873015873,
                "rz" => -0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "f0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "axial_force_i" => 0.0,
                "shear_force_i" => 1000.0,
                "moment_i" => 2000.0,
                "axial_force_j" => 0.0,
                "shear_force_j" => -1000.0,
                "moment_j" => 0.0,
                "axial_stress" => 0.0,
                "max_bending_stress" => 1.25e7,
                "max_combined_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/frame-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "y" => 0.0,
              "fix_x" => true,
              "fix_y" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "y" => 0.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_rz" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "f0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4
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
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a frame 3d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling frame stiffness"
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
                "uz" => 0.0,
                "rx" => 0.0,
                "ry" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0,
                "rotation_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "y" => 0.0,
                "z" => 0.0,
                "ux" => 0.0,
                "uy" => -0.0015873015873015873,
                "uz" => 0.0,
                "rx" => 0.0,
                "ry" => 0.0,
                "rz" => -0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873,
                "rotation_magnitude" => 0.0011904761904761906
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "f0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "axial_force_i" => 0.0,
                "shear_force_y_i" => 1000.0,
                "shear_force_z_i" => 0.0,
                "torsion_i" => 0.0,
                "moment_y_i" => 0.0,
                "moment_z_i" => 2000.0,
                "axial_force_j" => 0.0,
                "shear_force_y_j" => -1000.0,
                "shear_force_z_j" => 0.0,
                "torsion_j" => 0.0,
                "moment_y_j" => 0.0,
                "moment_z_j" => 0.0,
                "axial_stress" => 0.0,
                "max_bending_stress" => 1.25e7,
                "max_combined_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/frame-3d/jobs",
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
              "fix_rx" => true,
              "fix_ry" => true,
              "fix_rz" => true,
              "load_x" => 0.0,
              "load_y" => 0.0,
              "load_z" => 0.0,
              "moment_x" => 0.0,
              "moment_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "y" => 0.0,
              "z" => 0.0,
              "fix_x" => false,
              "fix_y" => false,
              "fix_z" => false,
              "fix_rx" => false,
              "fix_ry" => false,
              "fix_rz" => false,
              "load_x" => 0.0,
              "load_y" => -1000.0,
              "load_z" => 0.0,
              "moment_x" => 0.0,
              "moment_y" => 0.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "f0",
              "node_i" => 0,
              "node_j" => 1,
              "area" => 0.02,
              "youngs_modulus" => 2.1e11,
              "shear_modulus" => 8.0e10,
              "torsion_constant" => 5.0e-6,
              "moment_of_inertia_y" => 8.0e-6,
              "moment_of_inertia_z" => 8.0e-6,
              "section_modulus_y" => 1.6e-4,
              "section_modulus_z" => 1.6e-4
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
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_rotation"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a one dimensional beam job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.55,
            "iteration" => 3,
            "message" => "assembling beam stiffness"
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
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "uy" => 0.0015873015873015873,
                "rz" => 0.0011904761904761906,
                "displacement_magnitude" => 0.0015873015873015873
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "b0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "shear_force_i" => 1000.0,
                "moment_i" => 2000.0,
                "shear_force_j" => -1000.0,
                "moment_j" => 0.0,
                "max_bending_stress" => 1.25e7
              }
            ],
            "max_displacement" => 0.0015873015873015873,
            "max_rotation" => 0.0011904761904761906,
            "max_moment" => 2000.0,
            "max_stress" => 1.25e7,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "fix_y" => false,
              "fix_rz" => false,
              "load_y" => -1000.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "b0",
              "node_i" => 0,
              "node_j" => 1,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4
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
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a thermal beam 1d job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.5,
            "iteration" => 1,
            "message" => "solving thermal beam"
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
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 2.0,
                "uy" => 0.0,
                "rz" => 0.0,
                "displacement_magnitude" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "tb0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "temperature_gradient_y" => 40.0,
                "thermal_curvature" => 0.0024,
                "shear_force_i" => 0.0,
                "moment_i" => 4032.0,
                "shear_force_j" => 0.0,
                "moment_j" => -4032.0,
                "max_bending_stress" => 2.52e7
              }
            ],
            "max_displacement" => 0.0,
            "max_rotation" => 0.0,
            "max_moment" => 4032.0,
            "max_stress" => 2.52e7,
            "max_temperature_gradient" => 40.0,
            "input" => %{"nodes" => [], "elements" => []}
          }
        }
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{
              "id" => "n0",
              "x" => 0.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            },
            %{
              "id" => "n1",
              "x" => 2.0,
              "fix_y" => true,
              "fix_rz" => true,
              "load_y" => 0.0,
              "moment_z" => 0.0
            }
          ],
          "elements" => [
            %{
              "id" => "tb0",
              "node_i" => 0,
              "node_j" => 1,
              "youngs_modulus" => 2.1e11,
              "moment_of_inertia" => 8.0e-6,
              "section_modulus" => 1.6e-4,
              "thermal_expansion" => 1.2e-5,
              "section_depth" => 0.2,
              "distributed_load_y" => 0.0,
              "temperature_gradient_y" => 40.0
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
    assert result_payload["result"]["max_moment"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert result_payload["result"]["max_temperature_gradient"] == 40.0
    assert length(result_payload["result"]["elements"]) == 1
  end
end
