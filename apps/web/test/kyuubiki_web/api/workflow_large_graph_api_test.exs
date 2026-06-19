defmodule KyuubikiWeb.Api.WorkflowLargeGraphApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  require Logger

  test "runs a large workflow graph at 96 intermediate nodes through the orchestration API" do
    run_large_graph_case(96)
  end

  test "runs a large workflow graph at 192 intermediate nodes through the orchestration API" do
    run_large_graph_case(192)
  end

  test "runs a large workflow graph at 256 intermediate nodes through the orchestration API" do
    run_large_graph_case(256)
  end

  test "runs a large workflow graph at 384 intermediate nodes through the orchestration API" do
    run_large_graph_case(384)
  end

  test "runs a large workflow graph at 512 intermediate nodes through the orchestration API" do
    run_large_graph_case(512)
  end

  defp run_large_graph_case(pass_through_count) do
    {:ok, _pid} =
      WorkflowApi.start_fake_agent_sessions([
        [heat_result_frame()],
        [thermo_result_frame()]
      ])

    port = WorkflowApi.await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()

    started_at = System.monotonic_time(:millisecond)

    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => build_graph(pass_through_count),
          "input_artifacts" => %{
            "heat_model" => WorkflowApi.heat_to_thermo_quad_input_artifacts()["heat_model"]
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    elapsed_ms = System.monotonic_time(:millisecond) - started_at

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    Logger.info(
      "workflow_large_graphs[web]: pass_through_count=#{pass_through_count} completed_nodes=#{length(payload["completed_nodes"])} elapsed_ms=#{elapsed_ms}"
    )

    assert length(payload["completed_nodes"]) == pass_through_count + 5
    assert payload["skipped_nodes"] == []
    assert hd(payload["completed_nodes"]) == "heat_model"
    assert List.last(payload["completed_nodes"]) == "thermo_output"

    tail_key =
      "pass_#{String.pad_leading(Integer.to_string(pass_through_count - 1), 3, "0")}.result"

    assert get_in(payload, ["artifacts", tail_key, "max_temperature"]) == 100.0
    assert get_in(payload, ["artifacts", "thermo_output.result", "max_stress"]) > 0.0
    assert length(payload["node_runs"]) == pass_through_count + 5
  end

  defp build_graph(pass_through_count) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.large-heat-to-thermo-chain-#{pass_through_count}",
      "name" => "Large heat to thermo chain",
      "version" => "1.0.0",
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["thermo_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => build_nodes(pass_through_count),
      "edges" => build_edges(pass_through_count)
    }
  end

  defp build_nodes(pass_through_count) do
    [
      %{
        "id" => "heat_model",
        "kind" => "input",
        "outputs" => [%{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}]
      },
      %{
        "id" => "solve_heat",
        "kind" => "solve",
        "operator_id" => "solve.heat_plane_quad_2d",
        "inputs" => [%{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}],
        "outputs" => [%{"id" => "result", "artifact_type" => "result/heat_plane_quad_2d"}]
      }
    ] ++
      Enum.map(0..(pass_through_count - 1), fn index ->
        %{
          "id" => pass_node_id(index),
          "kind" => "transform",
          "operator_id" => "transform.first_available",
          "inputs" => [%{"id" => "input", "artifact_type" => "result/heat_plane_quad_2d"}],
          "outputs" => [%{"id" => "result", "artifact_type" => "result/heat_plane_quad_2d"}]
        }
      end) ++
      [
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
          "config" => %{
            "nodes" => [
              %{
                "id" => "t0",
                "x" => 0.0,
                "y" => 0.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t1",
                "x" => 1.0,
                "y" => 0.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t2",
                "x" => 1.0,
                "y" => 1.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t3",
                "x" => 0.0,
                "y" => 1.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              }
            ],
            "elements" => [
              %{
                "id" => "tq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "thickness" => 0.02,
                "youngs_modulus" => 210.0e9,
                "poisson_ratio" => 0.3,
                "thermal_expansion" => 11.0e-6
              }
            ]
          },
          "inputs" => [%{"id" => "heat_result", "artifact_type" => "result/heat_plane_quad_2d"}],
          "outputs" => [
            %{"id" => "thermo_model", "artifact_type" => "study_model/thermal_plane_quad_2d"}
          ]
        },
        %{
          "id" => "solve_thermo",
          "kind" => "solve",
          "operator_id" => "solve.thermal_plane_quad_2d",
          "inputs" => [%{"id" => "model", "artifact_type" => "study_model/thermal_plane_quad_2d"}],
          "outputs" => [%{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}]
        },
        %{
          "id" => "thermo_output",
          "kind" => "output",
          "inputs" => [%{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}],
          "outputs" => []
        }
      ]
  end

  defp build_edges(pass_through_count) do
    [
      %{
        "id" => "edge-heat-input",
        "from" => %{"node" => "heat_model", "port" => "model"},
        "to" => %{"node" => "solve_heat", "port" => "model"},
        "artifact_type" => "study_model/heat_plane_quad_2d"
      }
    ] ++
      Enum.map(0..(pass_through_count - 1), fn index ->
        from_node = if index == 0, do: "solve_heat", else: pass_node_id(index - 1)

        %{
          "id" => "edge-pass-#{index}",
          "from" => %{"node" => from_node, "port" => "result"},
          "to" => %{"node" => pass_node_id(index), "port" => "input"},
          "artifact_type" => "result/heat_plane_quad_2d"
        }
      end) ++
      [
        %{
          "id" => "edge-tail-to-bridge",
          "from" => %{"node" => pass_node_id(pass_through_count - 1), "port" => "result"},
          "to" => %{"node" => "bridge_temperature", "port" => "heat_result"},
          "artifact_type" => "result/heat_plane_quad_2d"
        },
        %{
          "id" => "edge-bridge-to-thermo",
          "from" => %{"node" => "bridge_temperature", "port" => "thermo_model"},
          "to" => %{"node" => "solve_thermo", "port" => "model"},
          "artifact_type" => "study_model/thermal_plane_quad_2d"
        },
        %{
          "id" => "edge-thermo-output",
          "from" => %{"node" => "solve_thermo", "port" => "result"},
          "to" => %{"node" => "thermo_output", "port" => "result"},
          "artifact_type" => "result/thermal_plane_quad_2d"
        }
      ]
  end

  defp pass_node_id(index),
    do: "pass_#{String.pad_leading(Integer.to_string(index), 3, "0")}"

  defp heat_result_frame do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 100.0,
        "max_heat_flux" => 2846.0498941515416,
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 100.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 60.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0}
        ],
        "elements" => [
          %{"id" => "hq0", "temperature_gradient_x" => -20.0, "temperature_gradient_y" => -60.0}
        ],
        "input" => %{
          "elements" => [
            %{
              "id" => "hq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.02,
              "conductivity" => 45.0
            }
          ]
        }
      }
    }
  end

  defp thermo_result_frame do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0,
        "max_stress" => 34_477_611.940298505,
        "max_temperature_delta" => 30,
        "nodes" => [
          %{"id" => "h0", "temperature_delta" => 100.0},
          %{"id" => "h1", "temperature_delta" => 60.0},
          %{"id" => "h2", "temperature_delta" => 20.0},
          %{"id" => "h3", "temperature_delta" => 20.0}
        ],
        "elements" => [
          %{
            "id" => "tq0",
            "stress_x" => -34_477_611.940298505,
            "stress_y" => -34_477_611.940298505,
            "mechanical_strain_x" => -3.3e-4,
            "mechanical_strain_y" => -3.3e-4
          }
        ]
      }
    }
  end
end
