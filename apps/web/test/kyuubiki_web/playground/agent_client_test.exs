defmodule KyuubikiWeb.Playground.AgentClientTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  setup do
    original_config = Application.get_env(:kyuubiki_web, AgentClient, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentClient, original_config)
    end)

    :ok
  end

  test "sends a solve request to the rust agent and decodes the result" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link(%{
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
      })

    port = await_fake_agent_port()
    Application.put_env(:kyuubiki_web, AgentClient, host: "127.0.0.1", port: port)

    assert {:ok, result} =
             AgentClient.solve(%{
               length: 1.0,
               area: 0.01,
               youngs_modulus: 210.0e9,
               elements: 1,
               tip_force: 1000.0
             })

    assert_in_delta result["tip_displacement"], 4.761904761904762e-7, 1.0e-12
    assert_in_delta result["max_stress"], 100_000.0, 1.0e-6
  end

  test "returns an rpc error when the agent reports a failure" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link(%{
        "ok" => false,
        "error" => %{
          "code" => "solve_failed",
          "message" => "length must be a positive finite number"
        }
      })

    port = await_fake_agent_port()
    Application.put_env(:kyuubiki_web, AgentClient, host: "127.0.0.1", port: port)

    assert {:error, {:rpc_error, "solve_failed", message}} =
             AgentClient.solve(%{
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
      FakePlaygroundAgent.start_link(%{
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
      })

    port = await_fake_agent_port()
    Application.put_env(:kyuubiki_web, AgentClient, host: "127.0.0.1", port: port)

    assert {:ok, result} =
             AgentClient.solve(%{
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

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end
end
