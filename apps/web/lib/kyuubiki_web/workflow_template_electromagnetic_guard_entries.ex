defmodule KyuubikiWeb.WorkflowTemplateElectromagneticGuardEntries do
  @moduledoc false

  def list do
    [
      electrostatic_preheat_guard_entry(),
      electrostatic_triangle_preheat_guard_entry(),
      electrostatic_preheat_guard_heat_entry(),
      electrostatic_triangle_preheat_guard_heat_entry()
    ]
  end

  defp electrostatic_preheat_guard_entry do
    guard_entry(
      "workflow.electrostatic-preheat-guard-markdown",
      "Electrostatic pre-heat guard markdown",
      "Evaluate electrostatic quad field hotspots before thermal coupling and export a readiness or hold markdown report.",
      "electrostatic_model",
      "study_model/electrostatic_plane_quad_2d",
      "solve.electrostatic_plane_quad_2d",
      "result/electrostatic_plane_quad_2d",
      "Electrostatic Pre-Heat"
    )
  end

  defp electrostatic_triangle_preheat_guard_entry do
    guard_entry(
      "workflow.electrostatic-triangle-preheat-guard-markdown",
      "Electrostatic triangle pre-heat guard markdown",
      "Evaluate electrostatic triangle field hotspots before thermal coupling and export a readiness or hold markdown report.",
      "electrostatic_plane_triangle_model",
      "study_model/electrostatic_plane_triangle_2d",
      "solve.electrostatic_plane_triangle_2d",
      "result/electrostatic_plane_triangle_2d",
      "Electrostatic Triangle Pre-Heat"
    )
  end

  defp electrostatic_preheat_guard_heat_entry do
    guard_heat_entry(
      "workflow.electrostatic-preheat-guard-heat-json",
      "Electrostatic pre-heat guard -> heat JSON",
      "Evaluate electrostatic quad hotspots, hold when risk is present, or automatically bridge into a heat quad solve and export JSON when clear.",
      "electrostatic_model",
      "study_model/electrostatic_plane_quad_2d",
      "solve.electrostatic_plane_quad_2d",
      "result/electrostatic_plane_quad_2d",
      "bridge.electrostatic_field_to_heat_quad_2d",
      "study_model/heat_plane_quad_2d",
      "solve.heat_plane_quad_2d",
      "result/heat_plane_quad_2d",
      heat_quad_seed_model(),
      "Electrostatic Pre-Heat"
    )
  end

  defp electrostatic_triangle_preheat_guard_heat_entry do
    guard_heat_entry(
      "workflow.electrostatic-triangle-preheat-guard-heat-json",
      "Electrostatic triangle pre-heat guard -> heat JSON",
      "Evaluate electrostatic triangle hotspots, hold when risk is present, or automatically bridge into a heat triangle solve and export JSON when clear.",
      "electrostatic_plane_triangle_model",
      "study_model/electrostatic_plane_triangle_2d",
      "solve.electrostatic_plane_triangle_2d",
      "result/electrostatic_plane_triangle_2d",
      "bridge.electrostatic_field_to_heat_triangle_2d",
      "study_model/heat_plane_triangle_2d",
      "solve.heat_plane_triangle_2d",
      "result/heat_plane_triangle_2d",
      heat_triangle_seed_model(),
      "Electrostatic Triangle Pre-Heat"
    )
  end

  defp guard_entry(
         id,
         name,
         summary,
         entry_node_id,
         entry_artifact_type,
         solve_operator_id,
         solve_result_artifact_type,
         title_prefix
       ) do
    custom_entry(
      id,
      name,
      summary,
      ["electromagnetic"],
      ["electrostatic", "guard", "readiness", "hotspot", "condition", "2d"],
      build_guard_graph(
        id,
        name,
        entry_node_id,
        entry_artifact_type,
        solve_operator_id,
        solve_result_artifact_type,
        title_prefix
      ),
      [
        %{
          "node_id" => entry_node_id,
          "artifact_type" => entry_artifact_type,
          "description" => "Electrostatic study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "final_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown readiness or hold report based on hotspot threshold evaluation."
        }
      ]
    )
  end

  defp build_guard_graph(
         id,
         name,
         entry_node_id,
         entry_artifact_type,
         solve_operator_id,
         solve_result_artifact_type,
         title_prefix
       ) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["final_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(entry_node_id, "model", entry_artifact_type),
        solve_node("solve_electrostatic", solve_operator_id, entry_artifact_type, solve_result_artifact_type),
        %{
          "id" => "field_hotspots",
          "kind" => "extract",
          "operator_id" => "extract.field_hotspots",
          "config" => %{
            "source" => "elements",
            "field" => "electric_field_magnitude",
            "output_prefix" => "field",
            "percentile" => 90,
            "sample_limit" => 4,
            "sample_sort" => "value_desc"
          },
          "inputs" => [%{"id" => "result", "artifact_type" => solve_result_artifact_type}],
          "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
        },
        %{
          "id" => "gate",
          "kind" => "condition",
          "config" => %{
            "predicate" => %{
              "path" => "field_hotspot_count",
              "operator" => "gt",
              "value" => 0
            }
          },
          "inputs" => [%{"id" => "value", "artifact_type" => "report/summary"}],
          "outputs" => [
            %{"id" => "if_true", "artifact_type" => "report/summary"},
            %{"id" => "if_false", "artifact_type" => "report/summary"}
          ]
        },
        export_alert_node(
          "alert_export",
          "#{title_prefix} Hotspot Hold",
          "warning",
          "Hotspot candidates were detected. Review the electrostatic field before heat coupling."
        ),
        export_alert_node(
          "clear_export",
          "#{title_prefix} Ready",
          "info",
          "No hotspot hold conditions were detected. The electrostatic field is ready for downstream heat coupling."
        ),
        %{
          "id" => "merge_output",
          "kind" => "transform",
          "operator_id" => "transform.first_available",
          "inputs" => [
            %{"id" => "left", "artifact_type" => "export/markdown"},
            %{"id" => "right", "artifact_type" => "export/markdown"}
          ],
          "outputs" => [%{"id" => "result", "artifact_type" => "export/markdown"}]
        },
        %{
          "id" => "final_output",
          "kind" => "output",
          "inputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown"}],
          "outputs" => []
        }
      ],
      "edges" => [
        edge("e0", entry_node_id, "model", "solve_electrostatic", "model", entry_artifact_type),
        edge(
          "e1",
          "solve_electrostatic",
          "result",
          "field_hotspots",
          "result",
          solve_result_artifact_type
        ),
        edge("e2", "field_hotspots", "summary", "gate", "value", "report/summary"),
        edge("e3", "gate", "if_true", "alert_export", "summary", "report/summary"),
        edge("e4", "gate", "if_false", "clear_export", "summary", "report/summary"),
        edge("e5", "alert_export", "markdown", "merge_output", "left", "export/markdown"),
        edge("e6", "clear_export", "markdown", "merge_output", "right", "export/markdown"),
        edge("e7", "merge_output", "result", "final_output", "markdown", "export/markdown")
      ]
    }
  end

  defp guard_heat_entry(
         id,
         name,
         summary,
         entry_node_id,
         entry_artifact_type,
         solve_operator_id,
         solve_result_artifact_type,
         bridge_operator_id,
         heat_model_artifact_type,
         heat_solve_operator_id,
         heat_result_artifact_type,
         seed_model,
         title_prefix
       ) do
    custom_entry(
      id,
      name,
      summary,
      ["electromagnetic", "thermal"],
      ["electrostatic", "heat", "guard", "workflow_bridge", "condition", "2d"],
      build_guard_heat_graph(
        id,
        name,
        entry_node_id,
        entry_artifact_type,
        solve_operator_id,
        solve_result_artifact_type,
        bridge_operator_id,
        heat_model_artifact_type,
        heat_solve_operator_id,
        heat_result_artifact_type,
        seed_model
      ),
      [
        %{
          "node_id" => entry_node_id,
          "artifact_type" => entry_artifact_type,
          "description" => "Electrostatic study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "#{title_prefix} gate result as JSON: hotspot hold summary when blocked, heat summary when cleared."
        }
      ]
    )
  end

  defp build_guard_heat_graph(
         id,
         name,
         entry_node_id,
         entry_artifact_type,
         solve_operator_id,
         solve_result_artifact_type,
         bridge_operator_id,
         heat_model_artifact_type,
         heat_solve_operator_id,
         heat_result_artifact_type,
         seed_model
       ) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(entry_node_id, "model", entry_artifact_type),
        solve_node("solve_electrostatic", solve_operator_id, entry_artifact_type, solve_result_artifact_type),
        %{
          "id" => "gate",
          "kind" => "condition",
          "config" => %{
            "predicate" => %{
              "path" => "max_electric_field",
              "operator" => "gt",
              "value" => 8.0
            }
          },
          "inputs" => [%{"id" => "value", "artifact_type" => solve_result_artifact_type}],
          "outputs" => [
            %{"id" => "if_true", "artifact_type" => solve_result_artifact_type},
            %{"id" => "if_false", "artifact_type" => solve_result_artifact_type}
          ]
        },
        %{
          "id" => "field_hotspots",
          "kind" => "extract",
          "operator_id" => "extract.field_hotspots",
          "config" => %{
            "source" => "elements",
            "field" => "electric_field_magnitude",
            "output_prefix" => "field",
            "percentile" => 90,
            "sample_limit" => 4,
            "sample_sort" => "value_desc"
          },
          "inputs" => [%{"id" => "result", "artifact_type" => solve_result_artifact_type}],
          "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
        },
        %{
          "id" => "bridge_field_to_heat",
          "kind" => "transform",
          "operator_id" => bridge_operator_id,
          "config" => %{
            "seed_model" => seed_model,
            "contract" => bridge_contract_for_operator(bridge_operator_id)
          },
          "inputs" => [%{"id" => "electrostatic_result", "artifact_type" => solve_result_artifact_type}],
          "outputs" => [%{"id" => "heat_model", "artifact_type" => heat_model_artifact_type}]
        },
        solve_node("solve_heat", heat_solve_operator_id, heat_model_artifact_type, heat_result_artifact_type),
        %{
          "id" => "extract_heat_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "config" => %{"fields" => ["max_temperature", "max_heat_flux"]},
          "inputs" => [%{"id" => "result", "artifact_type" => heat_result_artifact_type}],
          "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
        },
        %{
          "id" => "merge_summary",
          "kind" => "transform",
          "operator_id" => "transform.first_available",
          "inputs" => [
            %{"id" => "left", "artifact_type" => "report/summary"},
            %{"id" => "right", "artifact_type" => "report/summary"}
          ],
          "outputs" => [%{"id" => "result", "artifact_type" => "report/summary"}]
        },
        %{
          "id" => "export_json",
          "kind" => "export",
          "operator_id" => "export.summary_json",
          "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
          "outputs" => [%{"id" => "json", "artifact_type" => "export/json"}]
        },
        %{
          "id" => "json_output",
          "kind" => "output",
          "inputs" => [%{"id" => "json", "artifact_type" => "export/json"}],
          "outputs" => []
        }
      ],
      "edges" => [
        edge("e0", entry_node_id, "model", "solve_electrostatic", "model", entry_artifact_type),
        edge("e1", "solve_electrostatic", "result", "gate", "value", solve_result_artifact_type),
        edge("e2", "gate", "if_true", "field_hotspots", "result", solve_result_artifact_type),
        edge("e3", "gate", "if_false", "bridge_field_to_heat", "electrostatic_result", solve_result_artifact_type),
        edge("e4", "bridge_field_to_heat", "heat_model", "solve_heat", "model", heat_model_artifact_type),
        edge("e5", "solve_heat", "result", "extract_heat_summary", "result", heat_result_artifact_type),
        edge("e6", "field_hotspots", "summary", "merge_summary", "left", "report/summary"),
        edge("e7", "extract_heat_summary", "summary", "merge_summary", "right", "report/summary"),
        edge("e8", "merge_summary", "result", "export_json", "summary", "report/summary"),
        edge("e9", "export_json", "json", "json_output", "json", "export/json")
      ]
    }
  end

  defp input_node(id, port_id, artifact_type) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [%{"id" => port_id, "artifact_type" => artifact_type}]
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

  defp export_alert_node(id, title, severity, summary) do
    %{
      "id" => id,
      "kind" => "export",
      "operator_id" => "export.alert_markdown",
      "config" => %{
        "title" => title,
        "severity" => severity,
        "summary" => summary,
        "fields" => ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"],
        "sample_count" => 4
      },
      "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
      "outputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown"}]
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end

  defp custom_entry(id, name, summary, domains, capability_tags, graph, entry_inputs, output_artifacts) do
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

  defp bridge_contract_for_operator("bridge.electrostatic_field_to_heat_quad_2d") do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => ["node_i", "node_j", "node_k", "node_l"]
      },
      "transform" => %{"scale" => 50.0, "reduction" => "mean", "default_value" => 0.0},
      "target" => %{"field" => "heat_load"}
    }
  end

  defp bridge_contract_for_operator("bridge.electrostatic_field_to_heat_triangle_2d") do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => ["node_i", "node_j", "node_k"]
      },
      "transform" => %{"scale" => 50.0, "reduction" => "mean", "default_value" => 0.0},
      "target" => %{"field" => "heat_load"}
    }
  end

  defp heat_quad_seed_model do
    %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "h2", "x" => 1.0, "y" => 1.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "h3", "x" => 0.0, "y" => 1.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "hq0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "node_l" => 3, "thickness" => 0.02, "conductivity" => 45.0}
      ]
    }
  end

  defp heat_triangle_seed_model do
    %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h2", "x" => 0.0, "y" => 1.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "thickness" => 0.02, "conductivity" => 45.0}
      ]
    }
  end
end
