defmodule KyuubikiWeb.Playground.RouterTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  @opts Router.init([])

  setup do
    Store.reset()
    AnalysisResultStore.reset()

    original_config = Application.get_env(:kyuubiki_web, AgentClient, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentClient, original_config)
    end)

    :ok
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
    Application.put_env(:kyuubiki_web, AgentClient, host: "127.0.0.1", port: port)

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
    assert result_payload["job"]["worker_id"] == "rust-agent-rpc"
    assert result_payload["result"]["max_displacement"] > 0
    assert length(result_payload["result"]["elements"]) == 1
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
    Application.put_env(:kyuubiki_web, AgentClient, host: "127.0.0.1", port: port)

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

  test "exposes orchestrator health" do
    conn =
      :get
      |> conn("/api/health")
      |> Router.call(@opts)

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)

    assert payload["service"] == "kyuubiki-orchestrator"
    assert payload["status"] == "ok"
    assert payload["transport"]["http"] == 4000
    assert payload["transport"]["solver_agent_tcp"] == 5001
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

    if payload["job"]["status"] == "completed" do
      payload
    else
      Process.sleep(10)
      wait_for_job(job_id, attempts - 1)
    end
  end

  defp wait_for_job(_job_id, 0), do: flunk("timed out waiting for async job completion")
end
