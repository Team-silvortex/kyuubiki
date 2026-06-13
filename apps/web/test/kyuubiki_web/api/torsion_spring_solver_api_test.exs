defmodule KyuubikiWeb.Api.TorsionSpringSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "runs a one dimensional torsion job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "iteration" => 2,
            "message" => "solving torsion shaft"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "rz" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 2.0, "rz" => 0.01}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "t0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 2.0,
                "twist" => 0.01,
                "torque" => 1200.0,
                "shear_stress" => 6.0e6
              }
            ],
            "max_rotation" => 0.01,
            "max_torque" => 1200.0,
            "max_stress" => 6.0e6,
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
        "/api/v1/fem/torsion-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "fix_rz" => true, "torque_z" => 0.0},
            %{"id" => "n1", "x" => 2.0, "fix_rz" => false, "torque_z" => 1200.0}
          ],
          "elements" => [
            %{
              "id" => "t0",
              "node_i" => 0,
              "node_j" => 1,
              "shear_modulus" => 8.0e10,
              "polar_moment" => 3.0e-6,
              "section_modulus" => 2.0e-4
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
    assert result_payload["result"]["max_torque"] > 0
    assert result_payload["result"]["max_stress"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end

  test "runs a one dimensional spring job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.75,
            "iteration" => 2,
            "message" => "solving spring chain"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "n0", "x" => 0.0, "ux" => 0.0},
              %{"index" => 1, "id" => "n1", "x" => 1.0, "ux" => 0.04}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "s0",
                "node_i" => 0,
                "node_j" => 1,
                "length" => 1.0,
                "extension" => 0.04,
                "force" => 1000.0
              }
            ],
            "max_displacement" => 0.04,
            "max_force" => 1000.0
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
        "/api/v1/fem/spring-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "fix_x" => true, "load_x" => 0.0},
            %{"id" => "n1", "x" => 1.0, "fix_x" => false, "load_x" => 1000.0}
          ],
          "elements" => [
            %{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 25_000.0}
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
    assert result_payload["result"]["max_force"] > 0
    assert length(result_payload["result"]["elements"]) == 1
  end
end
