defmodule KyuubikiWeb.WorkflowTemplateCustomEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowTemplateElectromagneticEntries
  alias KyuubikiWeb.WorkflowTemplateElectromagneticGuardEntries
  alias KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoEntries

  def list do
    WorkflowTemplateElectromagneticEntries.list() ++
      WorkflowTemplateElectromagneticGuardEntries.list() ++
      WorkflowTemplateElectromagneticGuardThermoEntries.list() ++
      [
        electrostatic_to_heat_quad_entry(),
        electrostatic_plane_quad_entry(),
        electrostatic_plane_quad_hotspot_alert_entry(),
        heat_to_thermo_quad_entry(),
        heat_to_thermo_quad_comparison_entry(),
        electrostatic_to_heat_triangle_entry(),
        electrostatic_heat_thermo_triangle_entry()
      ]
  end

  defp electrostatic_to_heat_quad_entry do
    custom_entry(
      "workflow.electrostatic-to-heat-quad-2d",
      "Electrostatic to heat quad",
      "Solves an electrostatic quad model, bridges field magnitude into a heat quad study, then extracts and exports a heat summary.",
      ["electromagnetic", "thermal"],
      ["electrostatic", "heat", "workflow_bridge", "quad", "2d"],
      graph_from_chain(
        "workflow.electrostatic-to-heat-quad-2d",
        "Electrostatic to heat quad",
        "electrostatic_model",
        "study_model/electrostatic_plane_quad_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_quad_2d",
            "study_model/electrostatic_plane_quad_2d",
            "result/electrostatic_plane_quad_2d"
          ),
          transform_node(
            "bridge_field_to_heat",
            "bridge.electrostatic_field_to_heat_quad_2d",
            %{
              "seed_model" => heat_quad_seed_model_example(),
              "contract" =>
                WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)
            },
            "electrostatic_result",
            "result/electrostatic_plane_quad_2d",
            "heat_model",
            "study_model/heat_plane_quad_2d"
          ),
          solve_node(
            "solve_heat",
            "solve.heat_plane_quad_2d",
            "study_model/heat_plane_quad_2d",
            "result/heat_plane_quad_2d"
          ),
          extract_node("extract_summary", "result/heat_plane_quad_2d", [
            "max_temperature",
            "max_heat_flux"
          ]),
          export_node(),
          output_node()
        ]
      ),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded heat summary after bridging the electrostatic field."
        }
      ]
    )
  end

  defp electrostatic_plane_quad_entry do
    custom_entry(
      "workflow.electrostatic-plane-quad-2d",
      "Electrostatic plane quad",
      "Solves a 2D electrostatic quad model, extracts the peak field metrics, and exports a JSON summary artifact.",
      ["electromagnetic"],
      ["electrostatic", "extract", "export", "quad", "2d"],
      graph_from_chain(
        "workflow.electrostatic-plane-quad-2d",
        "Electrostatic plane quad",
        "electrostatic_model",
        "study_model/electrostatic_plane_quad_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_quad_2d",
            "study_model/electrostatic_plane_quad_2d",
            "result/electrostatic_plane_quad_2d"
          ),
          extract_node("extract_summary", "result/electrostatic_plane_quad_2d", [
            "max_potential",
            "max_electric_field",
            "max_flux_density"
          ]),
          export_node(),
          output_node()
        ]
      ),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded summary for the electrostatic quad solve."
        }
      ]
    )
  end

  defp electrostatic_plane_quad_hotspot_alert_entry do
    custom_entry(
      "workflow.electrostatic-plane-quad-hotspot-alert",
      "Electrostatic plane quad hotspot alert",
      "Solves a 2D electrostatic quad model, extracts electric-field hotspots, and exports a markdown alert artifact.",
      ["electromagnetic"],
      ["electrostatic", "hotspot", "alert", "field", "quad", "2d"],
      graph_from_chain(
        "workflow.electrostatic-plane-quad-hotspot-alert",
        "Electrostatic plane quad hotspot alert",
        "electrostatic_model",
        "study_model/electrostatic_plane_quad_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_quad_2d",
            "study_model/electrostatic_plane_quad_2d",
            "result/electrostatic_plane_quad_2d"
          ),
          extract_field_hotspots_node(
            "extract_hotspots",
            "result/electrostatic_plane_quad_2d",
            "elements",
            "electric_field_magnitude",
            %{"percentile" => 90, "sample_limit" => 4}
          ),
          export_alert_node(
            "export_alert",
            %{
              "title" => "Electrostatic Quad Hotspot Alert",
              "severity" => "warning",
              "summary" =>
                "The electrostatic quad solve produced high electric-field hotspot regions.",
              "sample_field" => "electric_field_magnitude_hotspot_samples",
              "sample_value_key" => "electric_field_magnitude"
            },
            "report/summary",
            "export/markdown"
          ),
          markdown_output_node()
        ]
      ),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown alert describing electrostatic hotspot regions."
        }
      ]
    )
  end

  defp heat_to_thermo_quad_entry do
    custom_entry(
      "workflow.heat-to-thermo-quad-2d",
      "Heat to thermo quad",
      "Solves a heat plane quad model, bridges the temperature field into a thermo-mechanical quad model, then extracts and exports a summary.",
      ["thermal", "thermo_mechanical"],
      ["thermal", "thermo_mechanical", "workflow_bridge", "quad", "2d"],
      graph_from_chain(
        "workflow.heat-to-thermo-quad-2d",
        "Heat to thermo quad",
        "heat_model",
        "study_model/heat_plane_quad_2d",
        [
          solve_node(
            "solve_heat",
            "solve.heat_plane_quad_2d",
            "study_model/heat_plane_quad_2d",
            "result/heat_plane_quad_2d"
          ),
          transform_node(
            "bridge_temperature",
            "bridge.temperature_field_to_thermo_quad_2d",
            %{
              "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
              "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
            },
            "heat_result",
            "result/heat_plane_quad_2d",
            "thermo_model",
            "study_model/thermal_plane_quad_2d"
          ),
          solve_node(
            "solve_thermo",
            "solve.thermal_plane_quad_2d",
            "study_model/thermal_plane_quad_2d",
            "result/thermal_plane_quad_2d"
          ),
          extract_node("extract_summary", "result/thermal_plane_quad_2d", nil),
          export_node(),
          output_node()
        ]
      ),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded result summary for the final thermo-mechanical solve."
        }
      ]
    )
  end

  defp heat_to_thermo_quad_comparison_entry do
    custom_entry(
      "workflow.heat-to-thermo-quad-comparison-json",
      "Heat to thermo quad comparison JSON",
      "Solves a heat quad model, bridges into a thermo-mechanical quad model, then compares heat and thermo summaries in one JSON artifact.",
      ["thermal", "thermo_mechanical"],
      ["thermal", "thermo_mechanical", "summary", "compare", "benchmark", "quad", "2d"],
      heat_to_thermo_quad_comparison_graph(),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded comparison summary across heat and thermo-mechanical quad solves."
        }
      ]
    )
  end

  defp electrostatic_to_heat_triangle_entry do
    custom_entry(
      "workflow.electrostatic-to-heat-triangle-2d",
      "Electrostatic to heat triangle",
      "Solves an electrostatic triangle model, bridges field magnitude into a heat triangle study, then extracts and exports a heat summary.",
      ["electromagnetic", "thermal"],
      ["electrostatic", "heat", "workflow_bridge", "triangle", "2d"],
      graph_from_chain(
        "workflow.electrostatic-to-heat-triangle-2d",
        "Electrostatic to heat triangle",
        "electrostatic_plane_triangle_model",
        "study_model/electrostatic_plane_triangle_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_triangle_2d",
            "study_model/electrostatic_plane_triangle_2d",
            "result/electrostatic_plane_triangle_2d"
          ),
          transform_node(
            "bridge_field_to_heat",
            "bridge.electrostatic_field_to_heat_triangle_2d",
            %{
              "seed_model" => heat_triangle_seed_model_example(),
              "contract" =>
                WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(
                  50.0,
                  ["node_i", "node_j", "node_k"]
                )
            },
            "electrostatic_result",
            "result/electrostatic_plane_triangle_2d",
            "heat_model",
            "study_model/heat_plane_triangle_2d"
          ),
          solve_node(
            "solve_heat",
            "solve.heat_plane_triangle_2d",
            "study_model/heat_plane_triangle_2d",
            "result/heat_plane_triangle_2d"
          ),
          extract_node("extract_summary", "result/heat_plane_triangle_2d", [
            "max_temperature",
            "max_heat_flux"
          ]),
          export_node(),
          output_node()
        ]
      ),
      [
        %{
          "node_id" => "electrostatic_plane_triangle_model",
          "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
          "description" =>
            "Electrostatic plane triangle study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded heat triangle summary after bridging the electrostatic field."
        }
      ]
    )
  end

  defp electrostatic_heat_thermo_triangle_entry do
    custom_entry(
      "workflow.electrostatic-heat-thermo-triangle-summary-json",
      "Electrostatic heat thermo triangle summary JSON",
      "Runs the full electrostatic triangle to heat triangle to thermo-mechanical triangle chain and exports a JSON summary.",
      ["electromagnetic", "thermal", "thermo_mechanical"],
      [
        "electrostatic",
        "heat",
        "thermal",
        "thermo_mechanical",
        "workflow_bridge",
        "triangle",
        "summary",
        "2d"
      ],
      graph_from_chain(
        "workflow.electrostatic-heat-thermo-triangle-summary-json",
        "Electrostatic heat thermo triangle summary JSON",
        "electrostatic_plane_triangle_model",
        "study_model/electrostatic_plane_triangle_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_triangle_2d",
            "study_model/electrostatic_plane_triangle_2d",
            "result/electrostatic_plane_triangle_2d"
          ),
          transform_node(
            "bridge_field_to_heat",
            "bridge.electrostatic_field_to_heat_triangle_2d",
            %{
              "seed_model" => heat_triangle_seed_model_example(),
              "contract" =>
                WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(
                  50.0,
                  ["node_i", "node_j", "node_k"]
                )
            },
            "electrostatic_result",
            "result/electrostatic_plane_triangle_2d",
            "heat_model",
            "study_model/heat_plane_triangle_2d"
          ),
          solve_node(
            "solve_heat",
            "solve.heat_plane_triangle_2d",
            "study_model/heat_plane_triangle_2d",
            "result/heat_plane_triangle_2d"
          ),
          transform_node(
            "bridge_temperature",
            "bridge.temperature_field_to_thermo_triangle_2d",
            %{
              "seed_model" => WorkflowCatalogSupport.thermo_triangle_seed_model_example(),
              "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
            },
            "heat_result",
            "result/heat_plane_triangle_2d",
            "thermo_model",
            "study_model/thermal_plane_triangle_2d"
          ),
          solve_node(
            "solve_thermo",
            "solve.thermal_plane_triangle_2d",
            "study_model/thermal_plane_triangle_2d",
            "result/thermal_plane_triangle_2d"
          ),
          extract_node("extract_summary", "result/thermal_plane_triangle_2d", [
            "max_displacement",
            "max_stress",
            "max_temperature_delta"
          ]),
          export_node(),
          output_node()
        ]
      ),
      [
        %{
          "node_id" => "electrostatic_plane_triangle_model",
          "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
          "description" =>
            "Electrostatic plane triangle study model used as the coupled workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded summary for the final thermo-mechanical triangle solve."
        }
      ]
    )
  end

  defp graph_from_chain(id, name, entry_node_id, entry_artifact_type, chain_nodes) do
    nodes = [
      %{
        "id" => entry_node_id,
        "kind" => "input",
        "outputs" => [%{"id" => "model", "artifact_type" => entry_artifact_type}]
      }
      | chain_nodes
    ]

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => nodes,
      "edges" => build_edges(nodes)
    }
  end

  defp heat_to_thermo_quad_comparison_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.heat-to-thermo-quad-comparison-json",
      "name" => "Heat to thermo quad comparison JSON",
      "version" => "1.0.0",
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        %{
          "id" => "heat_model",
          "kind" => "input",
          "outputs" => [%{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}]
        },
        solve_node(
          "solve_heat",
          "solve.heat_plane_quad_2d",
          "study_model/heat_plane_quad_2d",
          "result/heat_plane_quad_2d"
        ),
        extract_node(
          "extract_heat_summary",
          "result/heat_plane_quad_2d",
          ["max_temperature", "max_heat_flux"]
        ),
        transform_node(
          "bridge_temperature",
          "bridge.temperature_field_to_thermo_quad_2d",
          %{
            "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "heat_result",
          "result/heat_plane_quad_2d",
          "thermo_model",
          "study_model/thermal_plane_quad_2d"
        ),
        solve_node(
          "solve_thermo",
          "solve.thermal_plane_quad_2d",
          "study_model/thermal_plane_quad_2d",
          "result/thermal_plane_quad_2d"
        ),
        extract_node(
          "extract_thermo_summary",
          "result/thermal_plane_quad_2d",
          ["max_displacement", "max_stress", "max_temperature_delta"]
        ),
        %{
          "id" => "compare_summaries",
          "kind" => "transform",
          "operator_id" => "transform.compare_summary_pair",
          "config" => %{
            "left_prefix" => "heat",
            "right_prefix" => "thermo",
            "include_ratio" => false,
            "include_percent_change" => false
          },
          "inputs" => [
            %{"id" => "left", "artifact_type" => "report/summary"},
            %{"id" => "right", "artifact_type" => "report/summary"}
          ],
          "outputs" => [%{"id" => "result", "artifact_type" => "report/summary"}]
        },
        export_node(),
        output_node()
      ],
      "edges" => [
        edge(
          "e0",
          "heat_model",
          "model",
          "solve_heat",
          "model",
          "study_model/heat_plane_quad_2d"
        ),
        edge(
          "e1",
          "solve_heat",
          "result",
          "extract_heat_summary",
          "result",
          "result/heat_plane_quad_2d"
        ),
        edge(
          "e2",
          "solve_heat",
          "result",
          "bridge_temperature",
          "heat_result",
          "result/heat_plane_quad_2d"
        ),
        edge(
          "e3",
          "bridge_temperature",
          "thermo_model",
          "solve_thermo",
          "model",
          "study_model/thermal_plane_quad_2d"
        ),
        edge(
          "e4",
          "solve_thermo",
          "result",
          "extract_thermo_summary",
          "result",
          "result/thermal_plane_quad_2d"
        ),
        edge(
          "e5",
          "extract_heat_summary",
          "summary",
          "compare_summaries",
          "left",
          "report/summary"
        ),
        edge(
          "e6",
          "extract_thermo_summary",
          "summary",
          "compare_summaries",
          "right",
          "report/summary"
        ),
        edge("e7", "compare_summaries", "result", "export_json", "summary", "report/summary"),
        edge("e8", "export_json", "json", "json_output", "json", "export/json")
      ]
    }
  end

  defp build_edges(nodes) do
    nodes
    |> Enum.chunk_every(2, 1, :discard)
    |> Enum.with_index()
    |> Enum.map(fn {[left, right], index} ->
      left_output = left |> Map.get("outputs", []) |> List.last(%{})
      right_input = right |> Map.get("inputs", []) |> List.first(%{})
      from_port = Map.fetch!(left_output, "id")
      artifact_type = Map.fetch!(left_output, "artifact_type")
      to_port = Map.fetch!(right_input, "id")

      %{
        "id" => "e#{index}",
        "from" => %{"node" => left["id"], "port" => from_port},
        "to" => %{"node" => right["id"], "port" => to_port},
        "artifact_type" => artifact_type
      }
    end)
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end

  defp solve_node(id, operator_id, input_artifact_type, output_artifact_type) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [%{"id" => "model", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "result", "artifact_type" => output_artifact_type}]
    }
  end

  defp transform_node(
         id,
         operator_id,
         config,
         input_id,
         input_artifact_type,
         output_id,
         output_artifact_type
       ) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => config,
      "inputs" => [%{"id" => input_id, "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => output_id, "artifact_type" => output_artifact_type}]
    }
  end

  defp extract_node(id, input_artifact_type, fields) do
    node = %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.result_summary",
      "inputs" => [%{"id" => "result", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
    }

    if is_list(fields), do: Map.put(node, "config", %{"fields" => fields}), else: node
  end

  defp extract_field_hotspots_node(id, input_artifact_type, source, field, config) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.field_hotspots",
      "config" => Map.merge(%{"source" => source, "field" => field}, config),
      "inputs" => [%{"id" => "result", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
    }
  end

  defp export_node do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
      "outputs" => [%{"id" => "json", "artifact_type" => "export/json"}]
    }
  end

  defp export_alert_node(id, config, input_artifact_type, output_artifact_type) do
    %{
      "id" => id,
      "kind" => "export",
      "operator_id" => "export.alert_markdown",
      "config" => config,
      "inputs" => [%{"id" => "summary", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "markdown", "artifact_type" => output_artifact_type}]
    }
  end

  defp output_node do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [%{"id" => "json", "artifact_type" => "export/json"}],
      "outputs" => []
    }
  end

  defp markdown_output_node do
    %{
      "id" => "markdown_output",
      "kind" => "output",
      "inputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown"}],
      "outputs" => []
    }
  end

  defp custom_entry(
         id,
         name,
         summary,
         domains,
         capability_tags,
         graph,
         entry_inputs,
         output_artifacts
       ) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => domains,
      "capability_tags" => capability_tags,
      "graph" => graph,
      "entry_inputs" => entry_inputs,
      "output_artifacts" => output_artifacts
    }
  end

  defp heat_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "n0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n3",
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
  end

  defp heat_triangle_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "t0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        }
      ],
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
  end
end
