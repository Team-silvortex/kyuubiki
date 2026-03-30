defmodule KyuubikiWeb.Playground.RouterTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  @opts Router.init([])

  setup do
    Store.reset()

    original_config = Application.get_env(:kyuubiki_web, AgentClient, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentClient, original_config)
    end)

    :ok
  end

  test "runs an axial bar job through the orchestration API" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link(%{
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
      })

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

    assert conn.status == 200

    payload = Jason.decode!(conn.resp_body)

    assert payload["job"]["status"] == "completed"
    assert payload["job"]["worker_id"] == "rust-agent-rpc"
    assert payload["result"]["max_displacement"] > 0
    assert length(payload["result"]["elements"]) == 1
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

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end
end
