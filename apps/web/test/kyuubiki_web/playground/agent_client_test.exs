defmodule KyuubikiWeb.Playground.AgentClientTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.TestSupport.{FakePlaygroundAgent, FakeStallingAgent}

  setup do
    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_client_config = Application.get_env(:kyuubiki_web, AgentClient, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      Application.put_env(:kyuubiki_web, AgentClient, original_client_config)
      AgentPool.reload()
    end)

    :ok
  end

  test "sends a solve request to the rust agent and decodes the result" do
    test_process = self()

    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "preprocessing",
            "progress" => 0.1,
            "iteration" => 1,
            "message" => "normalizing study inputs"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "tip_displacement" => 4.761904761904762e-7,
            "reaction_force" => -1000.0,
            "max_displacement" => 4.761904761904762e-7,
            "max_stress" => 100_000.0,
            "nodes" => [%{"index" => 0, "x" => 0.0, "displacement" => 0.0}],
            "elements" => [],
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
      endpoints: [%{id: "test-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    assert {:ok, result} =
             AgentClient.solve_bar_1d(
               %{
                 length: 1.0,
                 area: 0.01,
                 youngs_modulus: 210.0e9,
                 elements: 1,
                 tip_force: 1000.0
               },
               fn progress -> send(test_process, {:progress, progress}) end
             )

    assert_receive {:progress, %{"stage" => "preprocessing", "progress" => 0.1}}
    assert_in_delta result["tip_displacement"], 4.761904761904762e-7, 1.0e-12
    assert_in_delta result["max_stress"], 100_000.0, 1.0e-6
  end

  test "returns an rpc error when the agent reports a failure" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => false,
          "error" => %{
            "code" => "solve_failed",
            "message" => "length must be a positive finite number"
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "test-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    assert {:error, {:rpc_error, "solve_failed", message}} =
             AgentClient.solve_bar_1d(%{
               length: 0.0,
               area: 0.01,
               youngs_modulus: 210.0e9,
               elements: 1,
               tip_force: 1000.0
             })

    assert message =~ "length"
  end

  test "decodes large solver payloads without truncation" do
    nodes =
      Enum.map(0..60, fn index ->
        %{
          "index" => index,
          "x" => index / 3,
          "displacement" => index * 8.57142857142857e-7
        }
      end)

    elements =
      Enum.map(0..59, fn index ->
        %{
          "index" => index,
          "x1" => index / 3,
          "x2" => (index + 1) / 3,
          "strain" => 8.57142857142857e-7,
          "stress" => 180_000.0,
          "axial_force" => 1800.0
        }
      end)

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
            "tip_displacement" => 5.142857142857142e-5,
            "reaction_force" => -1800.0,
            "max_displacement" => 5.142857142857142e-5,
            "max_stress" => 180_000.0,
            "nodes" => nodes,
            "elements" => elements,
            "input" => %{
              "length" => 20.0,
              "area" => 0.01,
              "youngs_modulus" => 70.0e9,
              "elements" => 60,
              "tip_force" => 1800.0
            }
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "test-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    assert {:ok, result} =
             AgentClient.solve_bar_1d(%{
               length: 20.0,
               area: 0.01,
               youngs_modulus: 70.0e9,
               elements: 60,
               tip_force: 1800.0
             })

    assert length(result["nodes"]) == 61
    assert length(result["elements"]) == 60
    assert_in_delta result["tip_displacement"], 5.142857142857142e-5, 1.0e-12
  end

  test "accepts heartbeat frames while waiting for the final response" do
    test_process = self()

    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "heartbeat",
          "progress" => %{
            "job_id" => "solver-session",
            "stage" => "solving",
            "progress" => 0.7,
            "message" => "agent heartbeat: solver still active"
          }
        },
        %{
          "ok" => true,
          "result" => %{
            "tip_displacement" => 1.0,
            "reaction_force" => -1.0,
            "max_displacement" => 1.0,
            "max_stress" => 1.0,
            "nodes" => [],
            "elements" => [],
            "input" => %{
              "length" => 1.0,
              "area" => 1.0,
              "youngs_modulus" => 1.0,
              "elements" => 1,
              "tip_force" => 1.0
            }
          }
        }
      ])

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "heartbeat-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    assert {:ok, _result} =
             AgentClient.solve_bar_1d(
               %{
                 length: 1.0,
                 area: 1.0,
                 youngs_modulus: 1.0,
                 elements: 1,
                 tip_force: 1.0
               },
               fn progress -> send(test_process, {:progress, progress}) end
             )

    assert_receive {:progress, %{"message" => "agent heartbeat: solver still active"}}
  end

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end

  test "fails over to the next configured agent when the first endpoint is unavailable" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "ok" => true,
          "result" => %{
            "tip_displacement" => 1.0,
            "reaction_force" => -1.0,
            "max_displacement" => 1.0,
            "max_stress" => 1.0,
            "nodes" => [],
            "elements" => [],
            "input" => %{
              "length" => 1.0,
              "area" => 1.0,
              "youngs_modulus" => 1.0,
              "elements" => 1,
              "tip_force" => 1.0
            }
          }
        }
      ])

    good_port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "offline", host: "127.0.0.1", port: 59_999},
        %{id: "online", host: "127.0.0.1", port: good_port}
      ]
    )

    AgentPool.reload()

    assert {:ok, result, endpoint} =
             AgentClient.request_with_agent("solve_bar_1d", %{
               length: 1.0,
               area: 1.0,
               youngs_modulus: 1.0,
               elements: 1,
               tip_force: 1.0
             })

    assert endpoint.id == "online"
    assert result["max_stress"] == 1.0
  end

  test "times out when the agent stops responding" do
    {:ok, _pid} = FakeStallingAgent.start_link(delay_ms: 250)
    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "stalling", host: "127.0.0.1", port: port}]
    )

    Application.put_env(:kyuubiki_web, AgentClient,
      connect_timeout_ms: 100,
      recv_timeout_ms: 100
    )

    AgentPool.reload()

    assert {:error, {:all_agents_failed, [%{agent: "rust-agent-rpc@stalling", reason: ":timeout"}]}} =
             AgentClient.solve_bar_1d(%{
               length: 1.0,
               area: 1.0,
               youngs_modulus: 1.0,
               elements: 1,
               tip_force: 1.0
             })
  end
end
