defmodule KyuubikiWeb.TestSupport.WorkflowApi do
  @moduledoc false

  require ExUnit.Assertions

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Router

  def await_fake_agent_port(timeout \\ 1_000) do
    receive do
      {:fake_agent_ready, port} -> port
    after
      timeout -> ExUnit.Assertions.flunk("timed out waiting for fake agent port")
    end
  end

  def start_fake_agent_sessions(frame_sessions) when is_list(frame_sessions) do
    parent = self()

    Task.start_link(fn ->
      {:ok, listen_socket} =
        :gen_tcp.listen(0, [
          :binary,
          packet: 4,
          active: false,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, port} = :inet.port(listen_socket)
      send(parent, {:fake_agent_ready, port})

      Enum.each(frame_sessions, fn frames ->
        {:ok, socket} = :gen_tcp.accept(listen_socket)
        {:ok, request_payload} = :gen_tcp.recv(socket, 0, 1_000)
        request = Jason.decode!(request_payload)

        Enum.each(frames, fn frame ->
          response_payload =
            frame
            |> Map.put_new("rpc_version", request["rpc_version"])
            |> Map.put_new("id", request["id"])
            |> Jason.encode!()

          :ok = :gen_tcp.send(socket, response_payload)
        end)

        :gen_tcp.close(socket)
      end)

      :gen_tcp.close(listen_socket)
    end)
  end

  def wait_for_job(job_id, router_opts, attempts \\ 20)

  def wait_for_job(job_id, router_opts, attempts)
      when is_binary(job_id) and attempts > 0 do
    conn =
      :get
      |> conn("/api/v1/jobs/#{job_id}")
      |> Router.call(router_opts)

    payload = Jason.decode!(conn.resp_body)

    if payload["job"]["status"] in ["completed", "failed"] do
      payload
    else
      Process.sleep(10)
      wait_for_job(job_id, router_opts, attempts - 1)
    end
  end

  def wait_for_job(_job_id, _router_opts, 0),
    do: ExUnit.Assertions.flunk("timed out waiting for async job completion")

  def configure_fake_agent_pool(port) when is_integer(port) do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end

  def submit_catalog_workflow_job(router_opts, workflow_id, input_artifacts)
      when is_list(router_opts) and is_binary(workflow_id) and is_map(input_artifacts) do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/catalog/#{workflow_id}/jobs",
        Jason.encode!(%{"input_artifacts" => input_artifacts})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(router_opts)

    ExUnit.Assertions.assert(conn.status == 202)
    payload = Jason.decode!(conn.resp_body)
    wait_for_job(payload["job"]["job_id"], router_opts)
  end

  def start_guarded_quad_sessions(:blocked) do
    start_fake_agent_sessions([[quad_electrostatic_result(:blocked)]])
  end

  def start_guarded_quad_sessions(:continued) do
    start_fake_agent_sessions([
      [quad_electrostatic_result(:continued)],
      [quad_heat_result()],
      [quad_thermo_result()]
    ])
  end

  def start_electrostatic_quad_summary_session do
    start_fake_agent_sessions([[quad_catalog_summary_result()]])
  end

  def start_guarded_triangle_sessions(:blocked) do
    start_fake_agent_sessions([[triangle_electrostatic_result(:blocked)]])
  end

  def start_guarded_triangle_sessions(:continued) do
    start_fake_agent_sessions([
      [triangle_electrostatic_result(:continued)],
      [triangle_heat_result()],
      [triangle_thermo_result()]
    ])
  end

  def assert_guard_blocked(result_payload, workflow_id, skipped_nodes, hotspot_max) do
    ExUnit.Assertions.assert(result_payload["job"]["status"] == "completed")
    ExUnit.Assertions.assert(result_payload["result"]["workflow_id"] == workflow_id)
    ExUnit.Assertions.assert(length(result_payload["result"]["completed_nodes"]) == 7)
    ExUnit.Assertions.assert(length(result_payload["result"]["progress_events"]) == 7)

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "field_hotspots")
    )

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_heat")
    )

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_thermo")
    )

    ExUnit.Assertions.assert(
      MapSet.subset?(
        MapSet.new(skipped_nodes),
        MapSet.new(result_payload["result"]["skipped_nodes"])
      )
    )

    ExUnit.Assertions.assert(
      result_payload["result"]["branch_decisions"] == [
        %{"node_id" => "gate", "chosen_output" => "if_true", "predicate_result" => true}
      ]
    )

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    ExUnit.Assertions.assert(exported["format"] == "json")

    summary = Jason.decode!(exported["content"])
    ExUnit.Assertions.assert(summary["field_hotspot_count"] == 1)
    ExUnit.Assertions.assert(summary["field_hotspot_max"] == hotspot_max)
    ExUnit.Assertions.assert(summary["field_threshold"] == hotspot_max)
    ExUnit.Assertions.assert(summary["source_field"] == "electric_field_magnitude")
  end

  def assert_guard_continued(
        result_payload,
        workflow_id,
        expected_temperature_deltas,
        expected_summary
      ) do
    ExUnit.Assertions.assert(result_payload["job"]["status"] == "completed")
    ExUnit.Assertions.assert(result_payload["result"]["workflow_id"] == workflow_id)
    ExUnit.Assertions.assert(length(result_payload["result"]["completed_nodes"]) == 11)
    ExUnit.Assertions.assert(length(result_payload["result"]["progress_events"]) == 11)

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "field_hotspots")
    )

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_heat")
    )

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_thermo")
    )

    ExUnit.Assertions.assert(
      MapSet.equal?(
        MapSet.new(result_payload["result"]["skipped_nodes"]),
        MapSet.new(["field_hotspots"])
      )
    )

    ExUnit.Assertions.assert(
      result_payload["result"]["branch_decisions"] == [
        %{"node_id" => "gate", "chosen_output" => "if_false", "predicate_result" => false}
      ]
    )

    bridged_heat_model = result_payload["result"]["artifacts"]["bridge_field_to_heat.heat_model"]

    ExUnit.Assertions.assert(
      Enum.all?(bridged_heat_model["nodes"], fn node -> node["heat_load"] == 300.0 end)
    )

    bridged_thermo_model =
      result_payload["result"]["artifacts"]["bridge_temperature.thermo_model"]

    ExUnit.Assertions.assert(
      Enum.map(bridged_thermo_model["nodes"], & &1["temperature_delta"]) ==
        expected_temperature_deltas
    )

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    ExUnit.Assertions.assert(exported["format"] == "json")

    summary = Jason.decode!(exported["content"])
    ExUnit.Assertions.assert(summary["max_displacement"] == expected_summary["max_displacement"])
    ExUnit.Assertions.assert(summary["max_stress"] == expected_summary["max_stress"])

    ExUnit.Assertions.assert(
      summary["max_temperature_delta"] == expected_summary["max_temperature_delta"]
    )
  end

  def heat_to_thermo_quad_input_artifacts do
    %{
      "heat_model" => %{
        "nodes" => [
          %{
            "id" => "h0",
            "x" => 0.0,
            "y" => 0.0,
            "fix_temperature" => true,
            "temperature" => 100.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h1",
            "x" => 1.0,
            "y" => 0.0,
            "fix_temperature" => false,
            "temperature" => 0.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h2",
            "x" => 1.0,
            "y" => 1.0,
            "fix_temperature" => true,
            "temperature" => 20.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h3",
            "x" => 0.0,
            "y" => 1.0,
            "fix_temperature" => true,
            "temperature" => 20.0,
            "heat_load" => 0.0
          }
        ],
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
  end

  def electrostatic_plane_quad_input_artifacts do
    %{
      "electrostatic_model" => %{
        "nodes" => [
          %{
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "fix_potential" => true,
            "potential" => 10.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n1",
            "x" => 1.0,
            "y" => 0.0,
            "fix_potential" => true,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n2",
            "x" => 1.0,
            "y" => 1.0,
            "fix_potential" => true,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n3",
            "x" => 0.0,
            "y" => 1.0,
            "fix_potential" => true,
            "potential" => 10.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "thickness" => 0.05,
            "permittivity" => 2.0
          }
        ]
      }
    }
  end

  def electrostatic_plane_triangle_input_artifacts do
    %{
      "electrostatic_plane_triangle_model" => %{
        "nodes" => [
          %{"id" => "e0", "x" => 0.0, "y" => 0.0, "fix_potential" => true, "potential" => 12.0},
          %{"id" => "e1", "x" => 1.0, "y" => 0.0, "fix_potential" => true, "potential" => 0.0},
          %{
            "id" => "e2",
            "x" => 0.0,
            "y" => 1.0,
            "fix_potential" => false,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "id" => "et0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "thickness" => 0.05,
            "permittivity" => 2.0
          }
        ]
      }
    }
  end

  defp quad_electrostatic_result(:blocked) do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{
            "index" => 0,
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "potential" => 10.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "area" => 1.0,
            "average_potential" => 5.0,
            "potential_gradient_x" => -10.0,
            "potential_gradient_y" => 0.0,
            "electric_field_x" => 10.0,
            "electric_field_y" => 0.0,
            "electric_field_magnitude" => 10.0,
            "electric_flux_density_x" => 20.0,
            "electric_flux_density_y" => 0.0,
            "electric_flux_density_magnitude" => 20.0
          }
        ],
        "max_potential" => 10.0,
        "max_electric_field" => 10.0,
        "max_flux_density" => 20.0,
        "input" => %{
          "elements" => [
            %{
              "id" => "epq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        }
      }
    }
  end

  defp quad_catalog_summary_result do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{
            "index" => 0,
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "potential" => 10.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "area" => 1.0,
            "average_potential" => 5.0,
            "potential_gradient_x" => -10.0,
            "potential_gradient_y" => 0.0,
            "electric_field_x" => 10.0,
            "electric_field_y" => 0.0,
            "electric_field_magnitude" => 10.0,
            "electric_flux_density_x" => 20.0,
            "electric_flux_density_y" => 0.0,
            "electric_flux_density_magnitude" => 20.0
          }
        ],
        "max_potential" => 10.0,
        "max_electric_field" => 10.0,
        "max_flux_density" => 20.0,
        "input" => %{"nodes" => [], "elements" => []}
      }
    }
  end

  defp quad_electrostatic_result(:continued) do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{
            "index" => 0,
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "potential" => 6.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "area" => 1.0,
            "average_potential" => 3.0,
            "potential_gradient_x" => -6.0,
            "potential_gradient_y" => 0.0,
            "electric_field_x" => 6.0,
            "electric_field_y" => 0.0,
            "electric_field_magnitude" => 6.0,
            "electric_flux_density_x" => 12.0,
            "electric_flux_density_y" => 0.0,
            "electric_flux_density_magnitude" => 12.0
          }
        ],
        "max_potential" => 6.0,
        "max_electric_field" => 6.0,
        "max_flux_density" => 12.0,
        "input" => %{
          "elements" => [
            %{
              "id" => "epq0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "node_l" => 3,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        }
      }
    }
  end

  defp quad_heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 70.0,
        "max_heat_flux" => 1500.0,
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 70.0, "heat_load" => 300.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 45.0, "heat_load" => 300.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0}
        ],
        "elements" => [
          %{"id" => "hq0", "temperature_gradient_x" => -25.0, "temperature_gradient_y" => -50.0}
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

  defp quad_thermo_result do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0015,
        "max_stress" => 18_000_000.0,
        "max_temperature_delta" => 70.0,
        "nodes" => [
          %{"id" => "h0", "temperature_delta" => 70.0},
          %{"id" => "h1", "temperature_delta" => 45.0},
          %{"id" => "h2", "temperature_delta" => 20.0},
          %{"id" => "h3", "temperature_delta" => 20.0}
        ],
        "elements" => [
          %{
            "id" => "tq0",
            "stress_x" => -18_000_000.0,
            "stress_y" => -18_000_000.0,
            "mechanical_strain_x" => -1.8e-4,
            "mechanical_strain_y" => -1.8e-4
          }
        ]
      }
    }
  end

  defp triangle_electrostatic_result(:blocked) do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{"index" => 0, "id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 12.0},
          %{"index" => 1, "id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0},
          %{"index" => 2, "id" => "e2", "x" => 0.0, "y" => 1.0, "potential" => 6.0}
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "et0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "area" => 0.5,
            "electric_field_x" => 6.0,
            "electric_field_y" => 8.0,
            "electric_field_magnitude" => 10.0,
            "electric_flux_density_x" => 12.0,
            "electric_flux_density_y" => 16.0,
            "electric_flux_density_magnitude" => 20.0
          }
        ],
        "max_potential" => 12.0,
        "max_electric_field" => 10.0,
        "max_flux_density" => 20.0,
        "input" => %{
          "elements" => [
            %{
              "id" => "et0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        }
      }
    }
  end

  defp triangle_electrostatic_result(:continued) do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{"index" => 0, "id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 6.0},
          %{"index" => 1, "id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0},
          %{"index" => 2, "id" => "e2", "x" => 0.0, "y" => 1.0, "potential" => 3.0}
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "et0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "area" => 0.5,
            "electric_field_x" => 4.8,
            "electric_field_y" => 3.6,
            "electric_field_magnitude" => 6.0,
            "electric_flux_density_x" => 9.6,
            "electric_flux_density_y" => 7.2,
            "electric_flux_density_magnitude" => 12.0
          }
        ],
        "max_potential" => 6.0,
        "max_electric_field" => 6.0,
        "max_flux_density" => 12.0,
        "input" => %{
          "elements" => [
            %{
              "id" => "et0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.05,
              "permittivity" => 2.0
            }
          ]
        }
      }
    }
  end

  defp triangle_heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 75.0,
        "max_heat_flux" => 900.0,
        "nodes" => [
          %{"id" => "t0", "x" => 0.0, "y" => 0.0, "temperature" => 75.0, "heat_load" => 300.0},
          %{"id" => "t1", "x" => 1.0, "y" => 0.0, "temperature" => 55.0, "heat_load" => 300.0},
          %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature" => 35.0, "heat_load" => 300.0}
        ],
        "elements" => [
          %{
            "id" => "ht0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "temperature_gradient_x" => 20.0,
            "temperature_gradient_y" => -40.0,
            "heat_flux_x" => -450.0,
            "heat_flux_y" => 900.0
          }
        ],
        "input" => %{
          "elements" => [
            %{
              "id" => "ht0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "conductivity" => 45.0
            }
          ]
        }
      }
    }
  end

  defp triangle_thermo_result do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0025,
        "max_stress" => 22_500_000.0,
        "max_temperature_delta" => 75.0,
        "nodes" => [
          %{"id" => "t0", "temperature_delta" => 75.0},
          %{"id" => "t1", "temperature_delta" => 55.0},
          %{"id" => "t2", "temperature_delta" => 35.0}
        ],
        "elements" => [
          %{
            "id" => "tt0",
            "stress_x" => -22_500_000.0,
            "stress_y" => -18_000_000.0,
            "mechanical_strain_x" => -2.2e-4,
            "mechanical_strain_y" => -1.8e-4
          }
        ]
      }
    }
  end
end
