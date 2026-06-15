defmodule KyuubikiWeb.WorkflowOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule StubSolveClient do
    def request(method, payload, _on_progress, opts) do
      send(self(), {:solve_request, method, payload, opts})

      case method do
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

  test "compares summary pairs through the transform runtime" do
    payload = %{
      "left" => %{"max_stress" => 10.0, "max_temperature" => 40.0},
      "right" => %{"max_stress" => 13.0, "max_temperature" => 44.0}
    }

    assert {:ok, compared} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compare_summary_pair",
               payload,
               %{"left_prefix" => "mechanical", "right_prefix" => "thermal"}
             )

    assert compared["mechanical_max_stress"] == 10.0
    assert compared["thermal_max_stress"] == 13.0
    assert compared["delta_max_stress"] == 3.0
    assert compared["ratio_max_stress"] == 1.3
    assert compared["summary_shared_numeric_field_count"] == 2
  end

  test "extracts field statistics and hotspots from numeric result collections" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0},
        %{"id" => "n1", "temperature" => 50.0},
        %{"id" => "n2", "temperature" => 80.0}
      ],
      "elements" => [
        %{"id" => "e0", "electric_field_magnitude" => 2.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0},
        %{"id" => "e2", "electric_field_magnitude" => 9.0}
      ]
    }

    assert {:ok, stats} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_statistics",
               payload,
               %{"source" => "nodes", "field" => "temperature", "percentiles" => [50]}
             )

    assert stats["temperature_min"] == 20.0
    assert stats["temperature_max"] == 80.0
    assert stats["temperature_p50"] == 50.0

    assert {:ok, hotspots} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_hotspots",
               payload,
               %{"field" => "electric_field_magnitude", "threshold" => 5.0}
             )

    assert hotspots["electric_field_magnitude_hotspot_count"] == 2
    assert hotspots["electric_field_magnitude_hotspot_ids"] == ["e2", "e1"]
  end

  test "extracts dedicated thermal diagnostics from a heat result" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "n1", "temperature" => 50.0, "heat_load" => 10.0},
        %{"id" => "n2", "temperature" => 80.0, "heat_load" => -5.0}
      ],
      "elements" => [
        %{
          "id" => "e0",
          "temperature_gradient_x" => 3.0,
          "temperature_gradient_y" => 4.0,
          "heat_flux_x" => -6.0,
          "heat_flux_y" => 8.0
        },
        %{
          "id" => "e1",
          "temperature_gradient_x" => 0.0,
          "temperature_gradient_y" => 12.0,
          "heat_flux_x" => 5.0,
          "heat_flux_y" => 12.0
        }
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.thermal_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["thermal_temperature_min"] == 20.0
    assert diagnostics["thermal_temperature_max"] == 80.0
    assert diagnostics["thermal_temperature_span"] == 60.0
    assert diagnostics["thermal_total_heat_load"] == 5.0
    assert diagnostics["thermal_loaded_node_count"] == 2
    assert diagnostics["thermal_peak_gradient_magnitude"] == 12.0
    assert diagnostics["thermal_peak_gradient_id"] == "e1"
    assert diagnostics["thermal_peak_flux_magnitude"] == 13.0
    assert diagnostics["thermal_peak_flux_id"] == "e1"
  end

  test "extracts dedicated thermo-mechanical diagnostics from a structural thermal result" do
    payload = %{
      "nodes" => [
        %{
          "id" => "n0",
          "temperature_delta" => 0.0,
          "displacement_x" => 0.0,
          "displacement_y" => 0.0
        },
        %{
          "id" => "n1",
          "temperature_delta" => 20.0,
          "displacement_x" => 3.0,
          "displacement_y" => 4.0
        },
        %{
          "id" => "n2",
          "temperature_delta" => 35.0,
          "displacement_x" => 6.0,
          "displacement_y" => 8.0
        }
      ],
      "elements" => [
        %{"id" => "e0", "von_mises_stress" => 120.0},
        %{"id" => "e1", "von_mises_stress" => 180.0}
      ]
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.thermo_result_diagnostics",
               payload,
               %{}
             )

    assert diagnostics["thermo_temperature_delta_max"] == 35.0
    assert diagnostics["thermo_temperature_delta_span"] == 35.0
    assert diagnostics["thermo_heated_node_count"] == 2
    assert diagnostics["thermo_peak_displacement"] == 10.0
    assert diagnostics["thermo_peak_displacement_id"] == "n2"
    assert diagnostics["thermo_peak_stress"] == 180.0
    assert diagnostics["thermo_peak_stress_id"] == "e1"
  end

  test "evaluates thermal guard thresholds into pass warn block states" do
    payload = %{
      "thermal_temperature_max" => 120.0,
      "thermal_peak_flux_magnitude" => 14.0,
      "thermo_peak_stress" => 210.0
    }

    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_thermal_guard",
               payload,
               %{
                 "rules" => [
                   %{
                     "field" => "thermal_temperature_max",
                     "threshold" => 100.0,
                     "severity" => "warn",
                     "label" => "temperature ceiling"
                   },
                   %{
                     "field" => "thermo_peak_stress",
                     "threshold" => 180.0,
                     "comparison" => "gt",
                     "severity" => "block",
                     "label" => "stress ceiling"
                   }
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_passed"] == false
    assert guard["guard_trigger_count"] == 2
    assert guard["guard_warn_count"] == 1
    assert guard["guard_block_count"] == 1
    assert guard["guard_recommendation"] == "hold_and_review"
    assert String.starts_with?(guard["guard_summary"], "BLOCK:")
    assert Enum.any?(guard["guard_triggers"], &(&1["field"] == "thermal_temperature_max"))
    assert Enum.any?(guard["guard_triggers"], &(&1["severity"] == "block"))
  end

  test "benchmarks coupled heat pairs with weighted criteria" do
    payload = %{
      "left" => %{
        "thermal_temperature_max" => 80.0,
        "thermal_peak_flux_magnitude" => 10.0,
        "thermal_loaded_node_count" => 3.0
      },
      "right" => %{
        "thermo_temperature_delta_max" => 75.0,
        "thermo_peak_stress" => 140.0,
        "thermo_heated_node_count" => 2.0
      }
    }

    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_coupled_heat_pair",
               payload,
               %{
                 "left_label" => "baseline",
                 "right_label" => "candidate",
                 "criteria" => [
                   %{
                     "field" => "temperature_vs_delta",
                     "left_field" => "thermal_temperature_max",
                     "right_field" => "thermo_temperature_delta_max",
                     "goal" => "min",
                     "weight" => 2.0
                   },
                   %{
                     "field" => "loaded_vs_heated_nodes",
                     "left_field" => "thermal_loaded_node_count",
                     "right_field" => "thermo_heated_node_count",
                     "goal" => "min",
                     "weight" => 1.0
                   },
                   %{
                     "field" => "flux_vs_stress",
                     "left_field" => "thermal_peak_flux_magnitude",
                     "right_field" => "thermo_peak_stress",
                     "goal" => "min",
                     "weight" => 3.0
                   }
                 ]
               }
             )

    assert benchmark["baseline_score"] == 3.0
    assert benchmark["candidate_score"] == 3.0
    assert benchmark["benchmark_winner"] == "tie"
    assert benchmark["benchmark_margin"] == 0.0
    assert benchmark["benchmark_criteria_count"] == 3
    assert benchmark["benchmark_left_win_count"] == 1
    assert benchmark["benchmark_right_win_count"] == 2
    assert benchmark["benchmark_tie_count"] == 0
    assert benchmark["benchmark_recommendation"] == "keep_both_under_review"
    assert String.contains?(benchmark["benchmark_summary"], "tie across 3 criteria")
    assert length(benchmark["benchmark_breakdown"]) == 3
  end

  test "exports hotspot summaries as markdown alerts" do
    payload = %{
      "field_hotspot_count" => 2,
      "field_hotspot_samples" => [
        %{"id" => "e2", "electric_field_magnitude" => 9.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0}
      ]
    }

    assert {:ok, exported} =
             WorkflowOperatorRuntime.run_export_operator(
               "export.alert_markdown",
               payload,
               %{"title" => "Electrostatic Hotspots", "severity" => "critical"}
             )

    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Electrostatic Hotspots")
    assert String.contains?(exported["content"], "- Severity: critical")
    assert String.contains?(exported["content"], "## Sample Context")
  end
end
