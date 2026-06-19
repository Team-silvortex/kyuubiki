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
     ["transform", "summary", "select", "ranking", "benchmark", "headless_safe"]},
    {"transform.compose_diagnostics_bundle", "compose_diagnostics_bundle",
     "Compose multiple diagnostics payloads into a single workflow diagnostics bundle with domain, source, and metric-group metadata.",
     ["transform", "diagnostics", "bundle", "compose", "headless_safe"]},
    {"transform.compose_diagnostics_report_payload", "compose_diagnostics_report_payload",
     "Compose a diagnostics bundle and guard result into a standard report payload for downstream export operators.",
     ["transform", "diagnostics", "bundle", "report", "compose", "headless_safe"]},
    {"transform.select_focus_payload", "select_focus_payload",
     "Select one standard focus payload by metric id from a diagnostics report payload for downstream workflow chaining.",
     ["transform", "diagnostics", "focus", "select", "headless_safe"]},
    {"transform.compose_focus_chain_input", "compose_focus_chain_input",
     "Compose a selected focus payload into a standard downstream chain input with bindings and orchestration annotations.",
     ["transform", "diagnostics", "focus", "chain", "compose", "headless_safe"]},
    {"transform.compose_focus_bridge_request", "compose_focus_bridge_request",
     "Compose a focus chain input into a standard bridge request payload with explicit bridge operator and bridge config.",
     ["transform", "diagnostics", "focus", "bridge", "compose", "headless_safe"]},
    {"transform.resolve_focus_bridge_execution", "resolve_focus_bridge_execution",
     "Resolve a focus bridge request into a directly executable bridge payload with operator id, source payload, and bridge config.",
     ["transform", "diagnostics", "focus", "bridge", "execute", "headless_safe"]},
    {"transform.execute_focus_bridge_execution", "execute_focus_bridge_execution",
     "Execute a resolved focus bridge execution payload and emit the resulting bridge output with execution lineage.",
     ["transform", "diagnostics", "focus", "bridge", "run", "headless_safe"]},
    {"transform.evaluate_diagnostics_bundle_guard", "evaluate_diagnostics_bundle_guard",
     "Evaluate a workflow diagnostics bundle against visible warn/block rules and emit a unified guard decision.",
     ["transform", "diagnostics", "bundle", "guard", "headless_safe"]},
    {"transform.evaluate_thermal_guard", "evaluate_thermal_guard",
     "Evaluate a thermal or thermo-mechanical diagnostic payload against visible threshold rules and emit pass, warn, or block guard state.",
     ["transform", "thermal", "guard", "threshold", "headless_safe"]},
    {"transform.benchmark_coupled_heat_pair", "benchmark_coupled_heat_pair",
     "Benchmark paired heat or thermo-mechanical summaries against weighted min/max criteria and emit a side-by-side winner breakdown.",
     ["transform", "thermal", "benchmark", "compare", "headless_safe"]}
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
     ["extract", "hotspot", "threshold", "field", "headless_safe"]},
    {"extract.electrostatic_result_diagnostics", "electrostatic_result_diagnostics",
     "Extract electrostatic diagnostics such as potential span, charge-density totals, and peak electric-field magnitude from an electrostatic result.",
     ["extract", "electrostatic", "diagnostics", "field", "headless_safe"]},
    {"extract.electrostatic_peak_field", "electrostatic_peak_field",
     "Extract the peak electric-field element and flux-density extrema from an electrostatic quad result.",
     ["extract", "electrostatic", "peak_field", "postprocess", "headless_safe"]},
    {"extract.thermal_result_diagnostics", "thermal_result_diagnostics",
     "Extract thermal diagnostics such as temperature span, heat-load totals, and peak gradient or flux magnitudes from a heat result.",
     ["extract", "thermal", "diagnostics", "heat", "headless_safe"]},
    {"extract.heat_peak_flux", "heat_peak_flux",
     "Extract the peak heat-flux element and temperature-gradient extrema from a heat quad result.",
     ["extract", "thermal", "heat_flux", "postprocess", "headless_safe"]},
    {"extract.thermo_result_diagnostics", "thermo_result_diagnostics",
     "Extract thermo-mechanical diagnostics such as temperature-delta spread, peak displacement, and peak stress from a thermal structural result.",
     ["extract", "thermo_mechanical", "diagnostics", "headless_safe"]},
    {"extract.thermo_peak_response", "thermo_peak_response",
     "Extract the peak displacement node and peak von-Mises element from a thermo-mechanical quad result.",
     ["extract", "thermo_mechanical", "peak_response", "postprocess", "headless_safe"]}
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
     ["export", "markdown", "alert", "headless_safe"]},
    {"export.diagnostics_bundle_markdown", "diagnostics_bundle_markdown",
     "Export a workflow diagnostics bundle and optional guard result as a readable markdown report.",
     ["export", "markdown", "diagnostics", "bundle", "headless_safe"]}
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
      heat_to_thermo_spec(
        "bridge.temperature_field_to_thermo_quad_2d",
        "quad",
        "result/heat_plane_quad_2d",
        "study_model/thermal_plane_quad_2d",
        "Heat quad result payload to bridge from",
        "Thermo-mechanical quad model with bridged nodal temperature deltas",
        WorkflowCatalogSupport.thermo_quad_seed_model_example()
      ),
      heat_to_thermo_spec(
        "bridge.temperature_field_to_thermo_triangle_2d",
        "triangle",
        "result/heat_plane_triangle_2d",
        "study_model/thermal_plane_triangle_2d",
        "Heat triangle result payload to bridge from",
        "Thermo-mechanical triangle model with bridged nodal temperature deltas",
        WorkflowCatalogSupport.thermo_triangle_seed_model_example()
      )
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

  defp heat_to_thermo_spec(
         id,
         shape,
         source_artifact_type,
         target_artifact_type,
         input_description,
         output_description,
         seed_model
       ) do
    %{
      "id" => id,
      "domain" => "thermo_mechanical",
      "family" => "thermal_plane_#{shape}_2d",
      "kind" => "workflow_bridge",
      "bridge_type" => "heat_to_thermo",
      "shape" => shape,
      "summary" =>
        "Bridge heat #{shape} temperatures into nodal temperature deltas for a downstream thermo-mechanical #{shape} model.",
      "capability_tags" => ["workflow_bridge", "temperature_field", "thermo", shape, "2d"],
      "config_schema" => %{
        "schema" => "kyuubiki.bridge-contract.heat_to_thermo.v1",
        "version" => "1"
      },
      "config_example" => %{
        "seed_model" => seed_model,
        "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
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
