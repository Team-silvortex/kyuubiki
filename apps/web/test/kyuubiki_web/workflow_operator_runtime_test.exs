defmodule KyuubikiWeb.WorkflowOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule StubSolveClient do
    def request(method, payload, _on_progress, opts) do
      send(self(), {:solve_request, method, payload, opts})

      case method do
        "solve_acoustic_bar_1d" ->
          {:ok, %{"solver" => "acoustic_bar_1d", "payload" => payload}}

        "solve_electrostatic_bar_1d" ->
          {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => payload}}

        "solve_heat_plane_triangle_2d" ->
          {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => payload}}

        "solve_frame_3d" ->
          {:ok, %{"solver" => "frame_3d", "payload" => payload}}
      end
    end

    def solve_electrostatic_bar_1d(payload),
      do: {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => payload}}

    def solve_acoustic_bar_1d(payload),
      do: {:ok, %{"solver" => "acoustic_bar_1d", "payload" => payload}}

    def solve_heat_plane_triangle_2d(payload),
      do: {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => payload}}

    def solve_frame_3d(payload), do: {:ok, %{"solver" => "frame_3d", "payload" => payload}}
  end

  setup do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(original, :solve_runtime_client, StubSolveClient)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
    end)

    :ok
  end

  test "dispatches supported solve operators through the configured runtime client" do
    assert {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => %{"model" => 1}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.electrostatic_bar_1d",
               %{"model" => 1}
             )

    assert {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => %{"model" => 2}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.heat_plane_triangle_2d",
               %{"model" => 2}
             )

    assert {:ok, %{"solver" => "frame_3d", "payload" => %{"model" => 3}}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.frame_3d", %{"model" => 3})

    assert {:ok, %{"solver" => "acoustic_bar_1d", "payload" => %{"model" => 4}}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.acoustic_bar_1d", %{"model" => 4})
  end

  test "rejects unsupported solve operators" do
    assert {:error, {:unsupported_workflow_solve_operator, "solve.unknown"}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.unknown", %{})
  end

  test "forwards workflow node routing constraints into the solve runtime client" do
    assert {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => %{"model" => 2}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.heat_plane_triangle_2d",
               %{"model" => 2},
               %{
                 "required_capabilities" => ["solver_rpc"],
                 "placement_tags" => ["thermal", "mesh", "triangle"]
               }
             )

    assert_receive {:solve_request, "solve_heat_plane_triangle_2d", %{"model" => 2},
                    [
                      required_capabilities: ["solver_rpc"],
                      placement_tags: ["thermal", "mesh", "triangle"],
                      orchestration: %{}
                    ]}
  end

  test "forwards orchestration context into the solve runtime client" do
    assert {:ok, %{"solver" => "frame_3d", "payload" => %{"model" => 3}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.frame_3d",
               %{"model" => 3},
               %{
                 "orchestration_context" => %{
                   "control_mode" => "orch_managed",
                   "orch_id" => "orch-alpha",
                   "orch_session_id" => "session-a"
                 }
               }
             )

    assert_receive {:solve_request, "solve_frame_3d", %{"model" => 3},
                    [
                      required_capabilities: [],
                      placement_tags: [],
                      orchestration: %{
                        control_mode: "orch_managed",
                        orch_id: "orch-alpha",
                        orch_session_id: "session-a"
                      }
                    ]}
  end

  test "catalog exposes newly wired workflow solve operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "solve.electrostatic_bar_1d")
    assert MapSet.member?(operators, "solve.acoustic_bar_1d")
    assert MapSet.member?(operators, "solve.heat_plane_triangle_2d")
    assert MapSet.member?(operators, "solve.frame_3d")
  end

  test "catalog exposes triangle bridge operators that local chains depend on" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "bridge.electrostatic_field_to_heat_triangle_2d")
    assert MapSet.member?(operators, "bridge.temperature_field_to_thermo_triangle_2d")
    assert MapSet.member?(operators, "transform.compare_summary_pair")
    assert MapSet.member?(operators, "transform.evaluate_thermal_guard")
    assert MapSet.member?(operators, "transform.benchmark_coupled_heat_pair")
    assert MapSet.member?(operators, "extract.field_statistics")
    assert MapSet.member?(operators, "extract.thermal_result_diagnostics")
    assert MapSet.member?(operators, "extract.thermo_result_diagnostics")
    assert MapSet.member?(operators, "export.alert_markdown")
  end

  test "catalog exposes real artifact contracts for heat to thermo bridge operators" do
    {:ok, %{"operator" => quad_bridge}} =
      WorkflowOperatorCatalog.fetch("bridge.temperature_field_to_thermo_quad_2d")

    {:ok, %{"operator" => triangle_bridge}} =
      WorkflowOperatorCatalog.fetch("bridge.temperature_field_to_thermo_triangle_2d")

    assert quad_bridge["inputs"] |> hd() |> Map.take(["id", "artifact_type", "dataset_value"]) ==
             %{
               "id" => "heat_result",
               "artifact_type" => "result/heat_plane_quad_2d",
               "dataset_value" => "heat_result"
             }

    assert quad_bridge["outputs"] |> hd() |> Map.take(["id", "artifact_type", "dataset_value"]) ==
             %{
               "id" => "thermo_model",
               "artifact_type" => "study_model/thermal_plane_quad_2d",
               "dataset_value" => "thermo_model"
             }

    assert triangle_bridge["inputs"] |> hd() |> Map.fetch!("artifact_type") ==
             "result/heat_plane_triangle_2d"

    assert triangle_bridge["outputs"] |> hd() |> Map.fetch!("artifact_type") ==
             "study_model/thermal_plane_triangle_2d"
  end

  test "bridges electrostatic triangle results into a heat triangle seed model" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "electric_field_magnitude" => 4.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      },
      "contract" =>
        WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(10.0, [
          "node_i",
          "node_j",
          "node_k"
        ])
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [40.0, 40.0, 40.0]
  end

  test "bridges electrostatic triangle results with max reduction" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0},
        %{"id" => "e3", "x" => 1.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "area" => 1.0,
          "electric_field_magnitude" => 2.0
        },
        %{
          "id" => "et1",
          "node_i" => 1,
          "node_j" => 3,
          "node_k" => 2,
          "area" => 2.0,
          "electric_field_magnitude" => 8.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0},
          %{"id" => "h3", "x" => 1.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [
          %{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2},
          %{"id" => "ht1", "node_i" => 1, "node_j" => 3, "node_k" => 2}
        ]
      },
      "contract" => %{
        "source" => %{
          "field" => "electric_field_magnitude",
          "distribution" => "element_to_nodes",
          "node_index_fields" => ["node_i", "node_j", "node_k"]
        },
        "transform" => %{"scale" => 1.0, "reduction" => "max", "default_value" => 0.0},
        "target" => %{"field" => "heat_load"}
      }
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [2.0, 8.0, 8.0, 8.0]
  end

  test "bridges electrostatic triangle flux alias into heat model" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "electric_flux_density_magnitude" => 5.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      },
      "contract" => %{
        "source" => %{
          "field" => "flux_magnitude",
          "distribution" => "element_to_nodes",
          "node_index_fields" => ["node_i", "node_j", "node_k"]
        },
        "transform" => %{"scale" => 10.0, "reduction" => "mean", "default_value" => 0.0},
        "target" => %{"field" => "heat_load"}
      }
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [50.0, 50.0, 50.0]
  end

  test "bridges heat triangle element fields into thermo nodes with max reduction" do
    heat_result = %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 10.0, "heat_load" => 0.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h2", "x" => 0.0, "y" => 1.0, "temperature" => 30.0, "heat_load" => 0.0},
        %{"id" => "h3", "x" => 1.0, "y" => 1.0, "temperature" => 40.0, "heat_load" => 0.0}
      ],
      "input" => %{
        "elements" => [
          %{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2},
          %{"id" => "ht1", "node_i" => 1, "node_j" => 3, "node_k" => 2}
        ]
      },
      "elements" => [
        %{
          "id" => "ht0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "area" => 1.0,
          "average_temperature" => 30.0
        },
        %{
          "id" => "ht1",
          "node_i" => 1,
          "node_j" => 3,
          "node_k" => 2,
          "area" => 2.0,
          "average_temperature" => 90.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "t0", "x" => 0.0, "y" => 0.0, "temperature_delta" => 0.0},
          %{"id" => "t1", "x" => 1.0, "y" => 0.0, "temperature_delta" => 0.0},
          %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature_delta" => 0.0},
          %{"id" => "t3", "x" => 1.0, "y" => 1.0, "temperature_delta" => 0.0}
        ],
        "elements" => [
          %{"id" => "tt0", "node_i" => 0, "node_j" => 1, "node_k" => 2},
          %{"id" => "tt1", "node_i" => 1, "node_j" => 3, "node_k" => 2}
        ]
      },
      "contract" => %{
        "source" => %{
          "field" => "average_temperature",
          "distribution" => "element_to_nodes",
          "node_index_fields" => ["node_i", "node_j", "node_k"]
        },
        "transform" => %{"scale" => 1.0, "reduction" => "max", "default_value" => 0.0},
        "target" => %{"field" => "temperature_delta"}
      }
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.temperature_field_to_thermo_triangle_2d",
               heat_result,
               config
             )

    assert Enum.map(nodes, & &1["temperature_delta"]) == [30.0, 90.0, 90.0, 90.0]
  end
end
