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
             endpoint_count: 2,
             cooling_down_count: 0,
             ready_endpoint_count: 2
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

  test "prefers method-capable and healthier endpoints before tag-only fallbacks" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "tag-only",
          host: "127.0.0.1",
          port: 5101,
          tags: ["truss"],
          capacity: 9,
          health_score: 100
        },
        %{
          id: "method-low-health",
          host: "127.0.0.1",
          port: 5102,
          methods: ["solve_truss_2d"],
          capacity: 2,
          health_score: 55
        },
        %{
          id: "capability-high-health",
          host: "127.0.0.1",
          port: 5103,
          capacity: 1,
          health_score: 95,
          capabilities: [
            %{
              id: "truss-2d",
              role: "solver",
              methods: ["solve_truss_2d"],
              tags: ["truss", "2d"]
            }
          ]
        }
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints("solve_truss_2d"), & &1.id) == [
             "capability-high-health",
             "method-low-health",
             "tag-only"
           ]
  end

  test "prefers endpoints whose declared capabilities and placement tags match workflow constraints" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "legacy-solver",
          host: "127.0.0.1",
          port: 5101
        },
        %{
          id: "thermal-solver",
          host: "127.0.0.1",
          port: 5102,
          role: "solver",
          tags: ["thermal", "mesh"],
          methods: ["solve_heat_plane_triangle_2d"],
          capabilities: [
            %{
              id: "solver_rpc",
              role: "solver",
              methods: ["solve_heat_plane_triangle_2d"],
              tags: ["thermal", "mesh"]
            }
          ]
        },
        %{
          id: "mechanical-solver",
          host: "127.0.0.1",
          port: 5103,
          role: "solver",
          tags: ["mechanical"],
          methods: ["solve_heat_plane_triangle_2d"]
        }
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(
             AgentPool.checkout_endpoints(
               "solve_heat_plane_triangle_2d",
               required_capabilities: ["solver_rpc"],
               placement_tags: ["thermal", "mesh"]
             ),
             & &1.id
           ) == ["thermal-solver", "legacy-solver"]
  end

  test "returns no endpoints when every typed agent explicitly mismatches workflow constraints" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "reporter",
          host: "127.0.0.1",
          port: 5101,
          role: "reporting",
          tags: ["reporting"],
          methods: ["export_summary_json"]
        }
      ]
    )

    assert :ok = AgentPool.reload()

    assert AgentPool.checkout_endpoints(
             "solve_heat_plane_triangle_2d",
             required_capabilities: ["solver_rpc"],
             placement_tags: ["thermal"]
           ) == []
  end

  test "deprioritizes cooling endpoints after reported failures and exposes cooldown state" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "agent-a", host: "127.0.0.1", port: 5101},
        %{id: "agent-b", host: "127.0.0.1", port: 5102},
        %{id: "agent-c", host: "127.0.0.1", port: 5103}
      ],
      failure_cooldown_ms: 60_000
    )

    assert :ok = AgentPool.reload()

    assert :ok =
             AgentPool.report_failure(%{id: "agent-a", host: "127.0.0.1", port: 5101}, :timeout)

    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-b", "agent-c", "agent-a"]

    cooled =
      AgentPool.endpoints()
      |> Enum.find(&(&1.id == "agent-a"))

    assert cooled.consecutive_failures == 1
    assert cooled.cooldown_remaining_ms > 0
    assert cooled.last_failure_reason == ":timeout"
    assert is_binary(cooled.cooldown_until)

    assert AgentPool.deployment_info().cooling_down_count == 1
    assert AgentPool.deployment_info().ready_endpoint_count == 2
  end

  test "clears cooldown state after a reported success" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "agent-a", host: "127.0.0.1", port: 5101}
      ],
      failure_cooldown_ms: 60_000
    )

    assert :ok = AgentPool.reload()

    assert :ok =
             AgentPool.report_failure(%{id: "agent-a", host: "127.0.0.1", port: 5101}, :timeout)

    assert :ok = AgentPool.report_success(%{id: "agent-a", host: "127.0.0.1", port: 5101})

    endpoint = hd(AgentPool.endpoints())

    assert endpoint.consecutive_failures == 0
    assert endpoint.cooldown_remaining_ms == 0
    assert endpoint.last_failure_reason == nil
    assert AgentPool.deployment_info().cooling_down_count == 0
  end
end
