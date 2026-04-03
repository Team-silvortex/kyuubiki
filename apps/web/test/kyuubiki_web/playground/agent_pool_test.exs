defmodule KyuubikiWeb.Playground.AgentPoolTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentPool

  setup do
    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      AgentPool.reload()
    end)

    :ok
  end

  test "rotates endpoints in round-robin order" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "agent-a", host: "127.0.0.1", port: 5101},
        %{id: "agent-b", host: "127.0.0.1", port: 5102},
        %{id: "agent-c", host: "127.0.0.1", port: 5103}
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-a", "agent-b", "agent-c"]
    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-b", "agent-c", "agent-a"]
    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-c", "agent-a", "agent-b"]
  end

  test "loads remote agents from a manifest file in distributed mode" do
    manifest_path = Path.join(System.tmp_dir!(), "kyuubiki-agent-manifest-test.json")

    File.write!(
      manifest_path,
      Jason.encode!(%{
        "schema_version" => "kyuubiki.agent-manifest/v1",
        "deployment_mode" => "distributed",
        "agents" => [
          %{
            "id" => "solver-a",
            "host" => "10.0.0.11",
            "port" => 6101,
            "region" => "ap-east",
            "role" => "solver"
          },
          %{
            "id" => "solver-b",
            "host" => "10.0.0.12",
            "port" => 6102,
            "zone" => "rack-b"
          }
        ]
      })
    )

    on_exit(fn -> File.rm(manifest_path) end)

    Application.put_env(:kyuubiki_web, AgentPool,
      deployment_mode: :distributed,
      discovery: :manifest,
      manifest_path: manifest_path
    )

    assert :ok = AgentPool.reload()

    endpoints = AgentPool.endpoints()
    assert Enum.map(endpoints, & &1.id) == ["solver-a", "solver-b"]
    assert Enum.at(endpoints, 0).host == "10.0.0.11"
    assert Enum.at(endpoints, 1).port == 6102

    assert AgentPool.deployment_info() == %{
             mode: :distributed,
             discovery: :manifest,
             manifest_path: manifest_path,
             endpoint_count: 2
           }
  end

  test "prefers tagged endpoints for matching solver methods" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "general-a", host: "127.0.0.1", port: 5101, tags: ["general"], capacity: 2},
        %{id: "plane-a", host: "127.0.0.1", port: 5102, tags: ["plane", "2d"], capacity: 1},
        %{id: "truss-a", host: "127.0.0.1", port: 5103, tags: ["truss", "2d"], capacity: 4}
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints("solve_truss_2d"), & &1.id) == [
             "truss-a",
             "general-a",
             "plane-a"
           ]

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints("solve_plane_triangle_2d"), & &1.id) == [
             "plane-a",
             "general-a",
             "truss-a"
           ]
  end

  test "sorts preferred agents by capacity before falling back" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "truss-low", host: "127.0.0.1", port: 5101, tags: ["truss", "2d"], capacity: 2},
        %{id: "truss-high", host: "127.0.0.1", port: 5102, tags: ["truss", "2d"], capacity: 8},
        %{id: "general", host: "127.0.0.1", port: 5103, tags: ["general"], capacity: 6}
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints("solve_truss_2d"), & &1.id) == [
             "truss-high",
             "truss-low",
             "general"
           ]
  end
end
