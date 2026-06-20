defmodule KyuubikiWeb.TestSupport.WorkflowCatalogBenchmark do
  @moduledoc false

  alias KyuubikiWeb.TestSupport.WorkflowApi
  alias KyuubikiWeb.TestSupport.WorkflowApiFixtures

  @default_case_ids [
    "workflow.heat-thermo-quad-benchmark-json",
    "workflow.electrostatic-heat-thermo-summary-json",
    "workflow.electrostatic-preheat-guard-heat-thermo-json",
    "workflow.electrostatic-preheat-guard-heat-thermo-blocked-json",
    "workflow.electrostatic-heat-thermo-triangle-summary-json",
    "workflow.electrostatic-quad-triangle-compare-json",
    "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
    "workflow.electrostatic-triangle-preheat-guard-heat-thermo-blocked-json"
  ]

  def default_case_ids, do: @default_case_ids

  def benchmark_report(router_opts, case_ids \\ @default_case_ids, repeat \\ 3)
      when is_list(router_opts) and is_list(case_ids) and is_integer(repeat) and repeat > 0 do
    cases =
      Enum.map(case_ids, fn case_id ->
        definition = benchmark_case!(case_id)
        runs = Enum.map(1..repeat, &run_case(router_opts, definition, &1))

        %{
          "case_id" => case_id,
          "workflow_id" => definition.workflow_id,
          "repeat" => repeat,
          "runs" => runs,
          "summary" => summarize_runs(runs)
        }
      end)

    %{
      "generated_at" => DateTime.utc_now() |> DateTime.truncate(:second) |> DateTime.to_iso8601(),
      "repeat" => repeat,
      "cases" => cases,
      "summary" => summarize_cases(cases)
    }
  end

  defp run_case(router_opts, definition, run_index) do
    {:ok, _pid} = WorkflowApi.start_fake_agent_sessions(definition.frame_sessions.())

    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    started_at = System.monotonic_time(:millisecond)

    result_payload =
      WorkflowApi.submit_catalog_workflow_job(
        router_opts,
        definition.workflow_id,
        definition.input_artifacts.()
      )

    elapsed_ms = System.monotonic_time(:millisecond) - started_at
    result = result_payload["result"]
    performance = Map.get(result, "performance", %{})
    exported = get_in(result, ["artifacts", "json_output.json", "content"]) || "{}"
    summary = Jason.decode!(exported)

    verify_case!(definition, result_payload, summary)

    %{
      "run_index" => run_index,
      "elapsed_ms" => elapsed_ms,
      "completed_node_count" => length(result["completed_nodes"]),
      "skipped_node_count" => length(result["skipped_nodes"]),
      "artifact_count" => map_size(Map.get(result, "artifacts", %{})),
      "progress_event_count" => length(Map.get(result, "progress_events", [])),
      "performance_available" => map_size(performance) > 0,
      "performance" => performance,
      "slowest_nodes" => Map.get(performance, "slowest_nodes", []),
      "summary_excerpt" => definition.summary_excerpt.(summary)
    }
  end

  defp summarize_runs(runs) do
    elapsed_ms = Enum.map(runs, & &1["elapsed_ms"])
    completed_node_counts = Enum.map(runs, & &1["completed_node_count"])

    %{
      "run_count" => length(runs),
      "min_elapsed_ms" => Enum.min(elapsed_ms),
      "max_elapsed_ms" => Enum.max(elapsed_ms),
      "avg_elapsed_ms" => Enum.sum(elapsed_ms) / max(length(elapsed_ms), 1),
      "median_elapsed_ms" => median(elapsed_ms),
      "completed_node_count_range" => [
        Enum.min(completed_node_counts),
        Enum.max(completed_node_counts)
      ]
    }
  end

  defp summarize_cases(cases) do
    medians = Enum.map(cases, & &1["summary"]["median_elapsed_ms"])

    %{
      "case_count" => length(cases),
      "fastest_case_id" => Enum.min_by(cases, & &1["summary"]["median_elapsed_ms"])["case_id"],
      "slowest_case_id" => Enum.max_by(cases, & &1["summary"]["median_elapsed_ms"])["case_id"],
      "median_elapsed_ms_range" => [Enum.min(medians), Enum.max(medians)]
    }
  end

  defp benchmark_case!("workflow.heat-thermo-quad-benchmark-json") do
    %{
      workflow_id: "workflow.heat-thermo-quad-benchmark-json",
      input_artifacts: &WorkflowApi.heat_to_thermo_quad_input_artifacts/0,
      frame_sessions: &heat_thermo_benchmark_frames/0,
      summary_excerpt: fn summary ->
        %{
          "benchmark_winner" => summary["benchmark_winner"],
          "benchmark_margin" => summary["benchmark_margin"],
          "thermal_score" => summary["thermal_score"],
          "thermo_score" => summary["thermo_score"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "heat thermo benchmark job did not complete"
        end

        unless length(result["completed_nodes"]) == 9 do
          raise "heat thermo benchmark completed node count drifted"
        end

        unless summary["benchmark_winner"] == "thermo" and summary["benchmark_margin"] == 3.0 do
          raise "heat thermo benchmark summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-preheat-guard-heat-thermo-json") do
    %{
      workflow_id: "workflow.electrostatic-preheat-guard-heat-thermo-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_quad_input_artifacts/0,
      frame_sessions: fn -> WorkflowApiFixtures.guarded_quad_frames(:continued) end,
      summary_excerpt: fn summary ->
        %{
          "max_displacement" => summary["max_displacement"],
          "max_stress" => summary["max_stress"],
          "max_temperature_delta" => summary["max_temperature_delta"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "guarded coupled job did not complete"
        end

        unless length(result["completed_nodes"]) == 11 do
          raise "guarded coupled completed node count drifted"
        end

        unless summary["max_displacement"] == 0.0015 and
                 summary["max_stress"] == 18_000_000.0 and
                 summary["max_temperature_delta"] == 70.0 do
          raise "guarded coupled summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-heat-thermo-summary-json") do
    %{
      workflow_id: "workflow.electrostatic-heat-thermo-summary-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_quad_input_artifacts/0,
      frame_sessions: fn -> WorkflowApiFixtures.guarded_quad_frames(:continued) end,
      summary_excerpt: fn summary ->
        %{
          "max_displacement" => summary["max_displacement"],
          "max_stress" => summary["max_stress"],
          "max_temperature_delta" => summary["max_temperature_delta"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "quad coupled job did not complete"
        end

        unless length(result["completed_nodes"]) == 9 do
          raise "quad coupled completed node count drifted"
        end

        unless summary["max_displacement"] == 0.0015 and
                 summary["max_stress"] == 18_000_000.0 and
                 summary["max_temperature_delta"] == 70.0 do
          raise "quad coupled summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-preheat-guard-heat-thermo-blocked-json") do
    %{
      workflow_id: "workflow.electrostatic-preheat-guard-heat-thermo-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_quad_input_artifacts/0,
      frame_sessions: fn -> WorkflowApiFixtures.guarded_quad_frames(:blocked) end,
      summary_excerpt: fn summary ->
        %{
          "field_hotspot_count" => summary["field_hotspot_count"],
          "field_hotspot_max" => summary["field_hotspot_max"],
          "field_threshold" => summary["field_threshold"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "guarded blocked job did not complete"
        end

        unless length(result["completed_nodes"]) == 7 do
          raise "guarded blocked completed node count drifted"
        end

        unless length(result["skipped_nodes"]) >= 5 do
          raise "guarded blocked skipped node count drifted"
        end

        unless summary["field_hotspot_count"] == 1 and
                 summary["field_hotspot_max"] == 10.0 and
                 summary["field_threshold"] == 10.0 do
          raise "guarded blocked summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-heat-thermo-triangle-summary-json") do
    %{
      workflow_id: "workflow.electrostatic-heat-thermo-triangle-summary-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_triangle_input_artifacts/0,
      frame_sessions: &electrostatic_heat_thermo_triangle_frames/0,
      summary_excerpt: fn summary ->
        %{
          "max_displacement" => summary["max_displacement"],
          "max_stress" => summary["max_stress"],
          "max_temperature_delta" => summary["max_temperature_delta"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "triangle coupled job did not complete"
        end

        unless length(result["completed_nodes"]) == 9 do
          raise "triangle coupled completed node count drifted"
        end

        unless summary["max_displacement"] == 0.0025 and
                 summary["max_stress"] == 22_500_000.0 and
                 summary["max_temperature_delta"] == 75.0 do
          raise "triangle coupled summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-quad-triangle-compare-json") do
    %{
      workflow_id: "workflow.electrostatic-quad-triangle-compare-json",
      input_artifacts: &electrostatic_quad_triangle_compare_input_artifacts/0,
      frame_sessions: &electrostatic_quad_triangle_compare_frames/0,
      summary_excerpt: fn summary ->
        %{
          "quad_potential_peak" => summary["quad_potential_peak"],
          "triangle_potential_peak" => summary["triangle_potential_peak"],
          "delta_potential_peak" => summary["delta_potential_peak"],
          "summary_shared_numeric_field_count" => summary["summary_shared_numeric_field_count"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "electrostatic compare job did not complete"
        end

        unless length(result["completed_nodes"]) == 11 do
          raise "electrostatic compare completed node count drifted"
        end

        unless summary["quad_potential_peak"] == 10.0 and
                 summary["triangle_potential_peak"] == 12.0 and
                 summary["delta_potential_peak"] == 2.0 and
                 summary["ratio_potential_peak"] == 1.2 and
                 summary["percent_change_potential_peak"] == 20.0 and
                 summary["delta_electric_field_peak"] == 0.0 and
                 summary["delta_flux_density_peak"] == 0.0 and
                 summary["summary_shared_numeric_field_count"] == 6 do
          raise "electrostatic compare summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-triangle-preheat-guard-heat-thermo-json") do
    %{
      workflow_id: "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_triangle_input_artifacts/0,
      frame_sessions: fn -> WorkflowApiFixtures.guarded_triangle_frames(:continued) end,
      summary_excerpt: fn summary ->
        %{
          "max_displacement" => summary["max_displacement"],
          "max_stress" => summary["max_stress"],
          "max_temperature_delta" => summary["max_temperature_delta"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "guarded triangle coupled job did not complete"
        end

        unless length(result["completed_nodes"]) == 11 do
          raise "guarded triangle completed node count drifted"
        end

        unless length(result["skipped_nodes"]) == 1 do
          raise "guarded triangle skipped node count drifted"
        end

        unless summary["max_displacement"] == 0.0025 and
                 summary["max_stress"] == 22_500_000.0 and
                 summary["max_temperature_delta"] == 75.0 do
          raise "guarded triangle summary drifted"
        end
      end
    }
  end

  defp benchmark_case!("workflow.electrostatic-triangle-preheat-guard-heat-thermo-blocked-json") do
    %{
      workflow_id: "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
      input_artifacts: &WorkflowApi.electrostatic_plane_triangle_input_artifacts/0,
      frame_sessions: fn -> WorkflowApiFixtures.guarded_triangle_frames(:blocked) end,
      summary_excerpt: fn summary ->
        %{
          "field_hotspot_count" => summary["field_hotspot_count"],
          "field_hotspot_max" => summary["field_hotspot_max"],
          "field_threshold" => summary["field_threshold"]
        }
      end,
      verify: fn result_payload, summary ->
        result = result_payload["result"]

        unless result_payload["job"]["status"] == "completed" do
          raise "guarded triangle blocked job did not complete"
        end

        unless length(result["completed_nodes"]) == 7 do
          raise "guarded triangle blocked completed node count drifted"
        end

        unless length(result["skipped_nodes"]) >= 5 do
          raise "guarded triangle blocked skipped node count drifted"
        end

        unless summary["field_hotspot_count"] == 1 and
                 summary["field_hotspot_max"] == 10.0 and
                 summary["field_threshold"] == 10.0 do
          raise "guarded triangle blocked summary drifted"
        end
      end
    }
  end

  defp benchmark_case!(case_id) do
    raise "unknown workflow catalog benchmark case: #{case_id}"
  end

  defp verify_case!(definition, result_payload, summary) do
    definition.verify.(result_payload, summary)

    performance = get_in(result_payload, ["result", "performance"]) || %{}

    if map_size(performance) > 0 do
      unless Map.get(performance, "completed_node_count", 0) > 0 do
        raise "workflow performance metrics missing completed node count"
      end

      unless Map.get(performance, "total_elapsed_ms", -1.0) >= 0.0 do
        raise "workflow performance metrics missing total elapsed time"
      end
    end
  end

  defp median(values) when is_list(values) do
    sorted = Enum.sort(values)
    count = length(sorted)
    midpoint = div(count, 2)

    if rem(count, 2) == 1 do
      Enum.at(sorted, midpoint)
    else
      (Enum.at(sorted, midpoint - 1) + Enum.at(sorted, midpoint)) / 2
    end
  end

  defp heat_thermo_benchmark_frames do
    [
      [
        %{
          "ok" => true,
          "result" => %{
            "max_temperature" => 70.0,
            "max_heat_flux" => 3000.0,
            "nodes" => [
              %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 70.0, "heat_load" => 300.0},
              %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 45.0, "heat_load" => 300.0},
              %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0},
              %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0}
            ],
            "elements" => [
              %{
                "id" => "hq0",
                "temperature_gradient_x" => -25.0,
                "temperature_gradient_y" => -50.0,
                "heat_flux_x" => -1800.0,
                "heat_flux_y" => 2400.0
              }
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
      ],
      [
        %{
          "ok" => true,
          "result" => %{
            "max_displacement" => 0.0015,
            "max_stress" => 1200.0,
            "max_temperature_delta" => 70.0,
            "nodes" => [
              %{"id" => "h0", "temperature_delta" => 70.0, "displacement_x" => 0.0, "displacement_y" => 0.0},
              %{"id" => "h1", "temperature_delta" => 45.0, "displacement_x" => 0.001, "displacement_y" => 0.0},
              %{"id" => "h2", "temperature_delta" => 20.0, "displacement_x" => 0.0015, "displacement_y" => 0.0},
              %{"id" => "h3", "temperature_delta" => 20.0, "displacement_x" => 0.0012, "displacement_y" => 0.0}
            ],
            "elements" => [
              %{
                "id" => "tq0",
                "von_mises_stress" => 1200.0,
                "stress_x" => -1200.0,
                "stress_y" => -1200.0
              }
            ]
          }
        }
      ]
    ]
  end

  defp electrostatic_quad_triangle_compare_input_artifacts do
    WorkflowApi.electrostatic_plane_quad_input_artifacts()
    |> Map.merge(WorkflowApi.electrostatic_plane_triangle_input_artifacts())
  end

  defp electrostatic_quad_triangle_compare_frames do
    WorkflowApiFixtures.electrostatic_quad_summary_frames() ++
      [List.first(electrostatic_heat_thermo_triangle_frames())]
  end

  defp electrostatic_heat_thermo_triangle_frames do
    [
      [
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
      ],
      [
        %{
          "ok" => true,
          "result" => %{
            "max_temperature" => 75.0,
            "max_heat_flux" => 900.0,
            "nodes" => [
              %{"id" => "t0", "x" => 0.0, "y" => 0.0, "temperature" => 75.0, "heat_load" => 500.0},
              %{"id" => "t1", "x" => 1.0, "y" => 0.0, "temperature" => 55.0, "heat_load" => 500.0},
              %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature" => 35.0, "heat_load" => 500.0}
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
      ],
      [
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
      ]
    ]
  end
end
