defmodule KyuubikiWeb.WorkflowTemplateElectromagneticEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateElectromagneticContractGraphs

  def list do
    [
      electrostatic_heat_thermo_quad_entry(),
      electrostatic_plane_quad_field_statistics_entry(),
      electrostatic_quad_triangle_compare_entry()
    ]
  end

  defp electrostatic_heat_thermo_quad_entry do
    custom_entry(
      "workflow.electrostatic-heat-thermo-summary-json",
      "Electrostatic heat thermo quad summary JSON",
      "Runs the full electrostatic quad to heat quad to thermo-mechanical quad chain and exports a JSON summary.",
      ["electromagnetic", "thermal", "thermo_mechanical"],
      [
        "electrostatic",
        "heat",
        "thermal",
        "thermo_mechanical",
        "workflow_bridge",
        "quad",
        "summary",
        "2d"
      ],
      WorkflowTemplateElectromagneticContractGraphs.electrostatic_heat_thermo_quad_graph(),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the coupled workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded summary for the final thermo-mechanical quad solve."
        }
      ]
    )
  end

  defp electrostatic_plane_quad_field_statistics_entry do
    custom_entry(
      "workflow.electrostatic-plane-quad-field-statistics-json",
      "Electrostatic plane quad field statistics JSON",
      "Solves an electrostatic quad model, extracts electric-field statistics, and exports a JSON summary.",
      ["electromagnetic"],
      ["electrostatic", "statistics", "field", "quad", "2d", "benchmark"],
      graph_from_chain(
        "workflow.electrostatic-plane-quad-field-statistics-json",
        "Electrostatic plane quad field statistics JSON",
        "electrostatic_model",
        "study_model/electrostatic_plane_quad_2d",
        [
          solve_node(
            "solve_electrostatic",
            "solve.electrostatic_plane_quad_2d",
            "study_model/electrostatic_plane_quad_2d",
            "result/electrostatic_plane_quad_2d"
          ),
          extract_statistics_node(
            "field_stats",
            "result/electrostatic_plane_quad_2d",
            "elements",
            "electric_field_magnitude",
            "field",
            [50, 90, 99]
          ),
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
          "description" => "JSON-encoded electric-field statistics summary."
        }
      ]
    )
  end

  defp electrostatic_quad_triangle_compare_entry do
    custom_entry(
      "workflow.electrostatic-quad-triangle-compare-json",
      "Electrostatic quad triangle compare JSON",
      "Runs electrostatic quad and triangle solves, normalizes shared fields, compares them, and exports a JSON benchmark delta.",
      ["electromagnetic"],
      ["electrostatic", "compare", "benchmark", "quad", "triangle", "2d"],
      electrostatic_quad_triangle_compare_graph(),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" => "Electrostatic plane quad study model used as the left entry artifact."
        },
        %{
          "node_id" => "electrostatic_triangle_model",
          "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
          "description" => "Electrostatic plane triangle study model used as the right entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded benchmark delta across quad and triangle electrostatic solves."
        }
      ]
    )
  end

  defp electrostatic_quad_triangle_compare_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-quad-triangle-compare-json",
      "name" => "Electrostatic quad triangle compare JSON",
      "version" => "1.0.0",
      "entry_nodes" => ["electrostatic_model", "electrostatic_plane_triangle_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(
          "electrostatic_model",
          "model",
          "study_model/electrostatic_plane_quad_2d"
        ),
        input_node(
          "electrostatic_plane_triangle_model",
          "model",
          "study_model/electrostatic_plane_triangle_2d"
        ),
        solve_node(
          "solve_quad",
          "solve.electrostatic_plane_quad_2d",
          "study_model/electrostatic_plane_quad_2d",
          "result/electrostatic_plane_quad_2d"
        ),
        extract_node("extract_quad_summary", "result/electrostatic_plane_quad_2d", [
          "max_potential",
          "max_electric_field",
          "max_flux_density"
        ]),
        solve_node(
          "solve_triangle",
          "solve.electrostatic_plane_triangle_2d",
          "study_model/electrostatic_plane_triangle_2d",
          "result/electrostatic_plane_triangle_2d"
        ),
        extract_node("extract_triangle_summary", "result/electrostatic_plane_triangle_2d", [
          "max_potential",
          "max_electric_field",
          "max_flux_density"
        ]),
        normalize_summary_node("normalize_quad_summary"),
        normalize_summary_node("normalize_triangle_summary"),
        compare_summary_node(),
        export_node(),
        output_node()
      ],
      "edges" => [
        edge(
          "e0",
          "electrostatic_model",
          "model",
          "solve_quad",
          "model",
          "study_model/electrostatic_plane_quad_2d"
        ),
        edge(
          "e1",
          "solve_quad",
          "result",
          "extract_quad_summary",
          "result",
          "result/electrostatic_plane_quad_2d"
        ),
        edge(
          "e2",
          "electrostatic_plane_triangle_model",
          "model",
          "solve_triangle",
          "model",
          "study_model/electrostatic_plane_triangle_2d"
        ),
        edge(
          "e3",
          "solve_triangle",
          "result",
          "extract_triangle_summary",
          "result",
          "result/electrostatic_plane_triangle_2d"
        ),
        edge(
          "e4",
          "extract_quad_summary",
          "summary",
          "normalize_quad_summary",
          "summary",
          "report/summary"
        ),
        edge(
          "e5",
          "extract_triangle_summary",
          "summary",
          "normalize_triangle_summary",
          "summary",
          "report/summary"
        ),
        edge("e6", "normalize_quad_summary", "result", "compare_summaries", "left", "report/summary"),
        edge("e7", "normalize_triangle_summary", "result", "compare_summaries", "right", "report/summary"),
        edge("e8", "compare_summaries", "result", "export_json", "summary", "report/summary"),
        edge("e9", "export_json", "json", "json_output", "json", "export/json")
      ]
    }
  end

  defp input_node(id, port_id, artifact_type, dataset_value \\ nil) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port(port_id, artifact_type, dataset_value)]
    }
  end

  defp solve_node(
         id,
         operator_id,
         input_artifact_type,
         output_artifact_type,
         input_dataset_value \\ nil,
         output_dataset_value \\ nil
       ) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [port("model", input_artifact_type, input_dataset_value)],
      "outputs" => [port("result", output_artifact_type, output_dataset_value)]
    }
  end

  defp extract_node(id, input_artifact_type, fields, input_dataset_value \\ nil, output_dataset_value \\ nil) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.result_summary",
      "config" => %{"fields" => fields},
      "inputs" => [port("result", input_artifact_type, input_dataset_value)],
      "outputs" => [port("summary", "report/summary", output_dataset_value)]
    }
  end

  defp extract_statistics_node(id, input_artifact_type, source, field, output_prefix, percentiles) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.field_statistics",
      "config" => %{
        "source" => source,
        "field" => field,
        "output_prefix" => output_prefix,
        "percentiles" => percentiles
      },
      "inputs" => [%{"id" => "result", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
    }
  end

  defp normalize_summary_node(id) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => "transform.normalize_summary_fields",
      "config" => %{
        "copy_unmapped" => true,
        "rules" => [
          %{"source" => "max_potential", "target" => "potential_peak"},
          %{"source" => "max_electric_field", "target" => "electric_field_peak"},
          %{"source" => "max_flux_density", "target" => "flux_density_peak"}
        ]
      },
      "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
      "outputs" => [%{"id" => "result", "artifact_type" => "report/summary"}]
    }
  end

  defp compare_summary_node do
    %{
      "id" => "compare_summaries",
      "kind" => "transform",
      "operator_id" => "transform.compare_summary_pair",
      "config" => %{
        "left_prefix" => "quad",
        "right_prefix" => "triangle",
        "delta_prefix" => "delta",
        "ratio_prefix" => "ratio",
        "percent_prefix" => "percent_change",
        "include_originals" => true,
        "include_delta" => true,
        "include_ratio" => true,
        "include_percent_change" => true,
        "include_shared_field_count" => true
      },
      "inputs" => [
        %{"id" => "left", "artifact_type" => "report/summary"},
        %{"id" => "right", "artifact_type" => "report/summary"}
      ],
      "outputs" => [%{"id" => "result", "artifact_type" => "report/summary"}]
    }
  end

  defp export_node(input_dataset_value \\ nil, output_dataset_value \\ nil) do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", input_dataset_value)],
      "outputs" => [port("json", "export/json", output_dataset_value)]
    }
  end

  defp output_node(input_dataset_value \\ nil) do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", input_dataset_value)],
      "outputs" => []
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value \\ nil) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
    |> maybe_put_dataset_value(dataset_value)
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

  defp build_edges(nodes) do
    nodes
    |> Enum.chunk_every(2, 1, :discard)
    |> Enum.with_index()
    |> Enum.map(fn {[left, right], index} ->
      left_output = left |> Map.get("outputs", []) |> List.last(%{})
      right_input = right |> Map.get("inputs", []) |> List.first(%{})

      %{
        "id" => "e#{index}",
        "from" => %{"node" => left["id"], "port" => Map.fetch!(left_output, "id")},
        "to" => %{"node" => right["id"], "port" => Map.fetch!(right_input, "id")},
        "artifact_type" => Map.fetch!(left_output, "artifact_type")
      }
      |> maybe_put_dataset_value(left_output["dataset_value"] || right_input["dataset_value"])
    end)
  end

  defp port(id, artifact_type, dataset_value) do
    %{"id" => id, "artifact_type" => artifact_type}
    |> maybe_put_dataset_value(dataset_value)
  end

  defp maybe_put_dataset_value(map, nil), do: map
  defp maybe_put_dataset_value(map, dataset_value), do: Map.put(map, "dataset_value", dataset_value)

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

end
