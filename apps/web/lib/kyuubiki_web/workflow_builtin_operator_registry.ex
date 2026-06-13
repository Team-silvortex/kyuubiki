defmodule KyuubikiWeb.WorkflowBuiltinOperatorRegistry do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  @transform_specs [
    {"transform.first_available", "first_available",
     "Merge two branch payloads by forwarding the first available incoming artifact.",
     ["transform", "merge", "branch", "headless_safe"]},
    {"transform.merge_summary_pair", "merge_summary_pair",
     "Merge two summary payloads into one namespaced summary artifact.",
     ["transform", "summary", "merge", "headless_safe"]},
    {"transform.compare_summary_pair", "compare_summary_pair",
     "Compare two summary payloads and emit delta, ratio, and percent-change metrics for shared numeric fields.",
     ["transform", "summary", "compare", "benchmark", "headless_safe"]},
    {"transform.aggregate_summary_collection", "aggregate_summary_collection",
     "Aggregate multiple summary payloads into min/max/mean/span benchmark metrics for shared numeric fields.",
     ["transform", "summary", "aggregate", "benchmark", "headless_safe"]},
    {"transform.normalize_summary_fields", "normalize_summary_fields",
     "Rename and normalize summary fields with scale, offset, and clamp rules before downstream comparison or aggregation.",
     ["transform", "summary", "normalize", "mapping", "headless_safe"]},
    {"transform.select_best_summary", "select_best_summary",
     "Score multiple summary payloads against weighted min/max criteria and emit the best candidate summary with selection metadata.",
     ["transform", "summary", "select", "ranking", "benchmark", "headless_safe"]}
  ]

  @extract_specs [
    {"extract.result_summary", "result_summary",
     "Extract a compact summary from a solver result artifact.",
     ["extract", "summary", "headless_safe"]},
    {"extract.field_statistics", "field_statistics",
     "Extract min/max/mean/sum/count statistics from a numeric field on result nodes or elements.",
     ["extract", "statistics", "field", "headless_safe"]},
    {"extract.field_hotspots", "field_hotspots",
     "Extract hotspot candidates from a numeric result field using an absolute or percentile threshold.",
     ["extract", "hotspot", "threshold", "field", "headless_safe"]}
  ]

  @export_specs [
    {"export.summary_json", "summary_json",
     "Export a compact summary artifact as structured JSON content.",
     ["export", "json", "summary", "headless_safe"]},
    {"export.summary_csv", "summary_csv",
     "Export a compact summary artifact as CSV text for downstream delivery.",
     ["export", "csv", "summary", "headless_safe"]},
    {"export.alert_markdown", "alert_markdown",
     "Export a summary payload as a readable markdown alert document.",
     ["export", "markdown", "alert", "headless_safe"]}
  ]

  def list do
    heat_to_thermo_bridge_specs() ++
      electrostatic_to_heat_bridge_specs() ++
      Enum.map(@transform_specs, &transform_spec/1) ++
      Enum.map(@extract_specs, &extract_spec/1) ++
      Enum.map(@export_specs, &export_spec/1)
  end

  defp heat_to_thermo_bridge_specs do
    [
      %{
        "id" => "bridge.temperature_field_to_thermo_quad_2d",
        "domain" => "thermo_mechanical",
        "family" => "thermal_plane_quad_2d",
        "kind" => "workflow_bridge",
        "summary" => "Bridge a heat quad temperature field into a thermal quad structural model.",
        "capability_tags" => ["workflow_bridge", "temperature_field", "quad", "2d"],
        "config_schema" => %{
          "schema" => "kyuubiki.bridge-contract.heat_to_thermo.v1",
          "version" => "1"
        },
        "config_example" => %{
          "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
          "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
        }
      },
      %{
        "id" => "bridge.temperature_field_to_thermo_triangle_2d",
        "domain" => "thermo_mechanical",
        "family" => "thermal_plane_triangle_2d",
        "kind" => "workflow_bridge",
        "summary" =>
          "Bridge a heat triangle temperature field into a thermal triangle structural model.",
        "capability_tags" => ["workflow_bridge", "temperature_field", "triangle", "2d"],
        "config_schema" => %{
          "schema" => "kyuubiki.bridge-contract.heat_to_thermo.v1",
          "version" => "1"
        },
        "config_example" => %{
          "seed_model" => WorkflowCatalogSupport.thermo_triangle_seed_model_example(),
          "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
        }
      }
    ]
  end

  defp electrostatic_to_heat_bridge_specs do
    [
      electrostatic_to_heat_spec(
        "bridge.electrostatic_field_to_heat_quad_2d",
        "quad",
        "result/electrostatic_plane_quad_2d",
        "study_model/heat_plane_quad_2d",
        "Electrostatic quad result payload to bridge from",
        "Heat quad model with bridged nodal heat loads",
        ["node_i", "node_j", "node_k", "node_l"]
      ),
      electrostatic_to_heat_spec(
        "bridge.electrostatic_field_to_heat_triangle_2d",
        "triangle",
        "result/electrostatic_plane_triangle_2d",
        "study_model/heat_plane_triangle_2d",
        "Electrostatic triangle result payload to bridge from",
        "Heat triangle model with bridged nodal heat loads",
        ["node_i", "node_j", "node_k"]
      )
    ]
  end

  defp electrostatic_to_heat_spec(
         id,
         shape,
         source_artifact_type,
         target_artifact_type,
         input_description,
         output_description,
         node_index_fields
       ) do
    %{
      "id" => id,
      "domain" => "electromagnetic",
      "family" => "electrostatic_to_heat_#{shape}_2d",
      "kind" => "workflow_bridge",
      "bridge_type" => "electrostatic_to_heat",
      "shape" => shape,
      "summary" =>
        "Bridge electrostatic #{shape} field magnitudes into nodal heat loads for a downstream heat #{shape} model.",
      "capability_tags" => ["workflow_bridge", "electrostatic", "heat", shape, "2d"],
      "config_schema" => %{
        "schema" => "kyuubiki.bridge-contract.electrostatic_to_heat.v1",
        "version" => "1"
      },
      "config_example" => %{
        "contract" =>
          WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(
            50.0,
            node_index_fields
          )
      },
      "source_artifact_type" => source_artifact_type,
      "target_artifact_type" => target_artifact_type,
      "input_description" => input_description,
      "output_description" => output_description
    }
  end

  defp transform_spec({id, family, summary, capability_tags}),
    do: simple_spec(id, "multi_domain", family, "transform", summary, capability_tags)

  defp extract_spec({id, family, summary, capability_tags}),
    do: simple_spec(id, "multi_domain", family, "extract", summary, capability_tags)

  defp export_spec({id, family, summary, capability_tags}),
    do: simple_spec(id, "multi_domain", family, "export", summary, capability_tags)

  defp simple_spec(id, domain, family, kind, summary, capability_tags) do
    %{
      "id" => id,
      "domain" => domain,
      "family" => family,
      "kind" => kind,
      "summary" => summary,
      "capability_tags" => capability_tags
    }
  end
end
