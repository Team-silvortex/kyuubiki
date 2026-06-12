defmodule KyuubikiWeb.WorkflowOperatorCatalog do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowCatalogQuery

  def list(filters \\ %{}) do
    normalized_filters = WorkflowCatalogQuery.normalize_filters(filters)

    [
      built_in_solver_descriptor("solve.frame_3d", "mechanical", "frame_3d", "Solve a 3D frame model with six-DOF nodes and verified baseline coverage.", ["verified", "mechanical", "frame", "3d"]),
      built_in_solver_descriptor("solve.thermal_frame_3d", "thermo_mechanical", "thermal_frame_3d", "Solve a thermal 3D frame model with restrained expansion and temperature gradients.", ["verified", "thermo_mechanical", "frame", "3d"]),
      built_in_solver_descriptor("solve.electrostatic_bar_1d", "electromagnetic", "electrostatic_bar_1d", "Solve a 1D electrostatic bar model and expose potential, field, and flux results.", ["verified", "electromagnetic", "electrostatic", "bar", "1d"]),
      built_in_solver_descriptor("solve.electrostatic_plane_triangle_2d", "electromagnetic", "electrostatic_plane_triangle_2d", "Solve a 2D electrostatic triangle model and expose potential, field, and flux results.", ["verified", "electromagnetic", "electrostatic", "plane", "triangle", "2d"]),
      built_in_solver_descriptor("solve.electrostatic_plane_quad_2d", "electromagnetic", "electrostatic_plane_quad_2d", "Solve a 2D electrostatic quad model and expose potential, field, and flux results.", ["verified", "electromagnetic", "electrostatic", "plane", "quad", "2d"]),
      built_in_solver_descriptor("solve.heat_plane_quad_2d", "thermal", "heat_plane_quad_2d", "Solve a 2D heat-conduction quad model and expose verified temperature/flux fields.", ["verified", "thermal", "heat", "plane", "2d"]),
      built_in_solver_descriptor("solve.thermal_truss_3d", "thermo_mechanical", "thermal_truss_3d", "Solve a thermal 3D truss model with expansion-driven axial response.", ["verified", "thermo_mechanical", "truss", "3d"]),
      built_in_bridge_descriptor("bridge.temperature_field_to_thermo_quad_2d", "thermo_mechanical", "thermal_plane_quad_2d", "Bridge a heat quad temperature field into a thermal quad structural model.", ["workflow_bridge", "temperature_field", "quad", "2d"], %{"schema" => "kyuubiki.bridge-contract.heat_to_thermo.v1", "version" => "1"}, %{"seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(), "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()}),
      %{
        "id" => "bridge.electrostatic_field_to_heat_quad_2d",
        "version" => "1.0.0",
        "domain" => "electromagnetic",
        "family" => "electrostatic_to_heat_quad_2d",
        "kind" => "workflow_bridge",
        "summary" => "Bridge electrostatic quad field magnitudes into nodal heat loads for a downstream heat quad model.",
        "capability_tags" => ["workflow_bridge", "electrostatic", "heat", "quad", "2d"],
        "origin" => "built_in",
        "input_schema" => %{"schema" => "kyuubiki.operator.electrostatic_to_heat_quad_2d.bridge_input", "version" => "1"},
        "output_schema" => %{"schema" => "kyuubiki.operator.electrostatic_to_heat_quad_2d.bridge_output", "version" => "1"},
        "config_schema" => %{"schema" => "kyuubiki.bridge-contract.electrostatic_to_heat.v1", "version" => "1"},
        "config_example" => %{"contract" => WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)},
        "inputs" => [
          %{"id" => "electrostatic_result", "artifact_type" => "result/electrostatic_plane_quad_2d", "description" => "Electrostatic quad result payload to bridge from", "dataset_value" => "electrostatic_result", "schema_ref" => %{"schema" => "kyuubiki.operator.electrostatic_plane_quad_2d.output", "version" => "1"}}
        ],
        "outputs" => [
          %{"id" => "heat_model", "artifact_type" => "study_model/heat_plane_quad_2d", "description" => "Heat quad model with bridged nodal heat loads", "dataset_value" => "heat_model", "schema_ref" => %{"schema" => "kyuubiki.operator.heat_plane_quad_2d.input", "version" => "1"}}
        ],
        "validation" => verified_operator_validation_profile("electrostatic_to_heat_quad_2d", ["workflow_graph", "catalog_job"])
      },
      built_in_extract_descriptor("extract.result_summary", "multi_domain", "result_summary", "Extract a compact summary from a solver result artifact.", ["extract", "summary", "headless_safe"]),
      built_in_export_descriptor("export.summary_json", "multi_domain", "summary_json", "Export a compact summary artifact as structured JSON content.", ["export", "json", "summary", "headless_safe"]),
      built_in_export_descriptor("export.summary_csv", "multi_domain", "summary_csv", "Export a compact summary artifact as CSV text for downstream delivery.", ["export", "csv", "summary", "headless_safe"])
    ]
    |> Enum.filter(&WorkflowCatalogQuery.matches_operator?(&1, normalized_filters))
  end

  def fetch(operator_id) when is_binary(operator_id) do
    case Enum.find(list(), &(&1["id"] == operator_id)) do
      nil -> {:error, {:operator_not_found, operator_id}}
      operator -> {:ok, %{"operator" => operator}}
    end
  end

  defp built_in_solver_descriptor(id, domain, family, summary, capability_tags) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "domain" => domain,
      "family" => family,
      "kind" => "solver",
      "summary" => summary,
      "capability_tags" => capability_tags,
      "origin" => "built_in",
      "input_schema" => %{"schema" => "kyuubiki.operator.#{family}.input", "version" => "1"},
      "output_schema" => %{"schema" => "kyuubiki.operator.#{family}.output", "version" => "1"},
      "inputs" => [operator_port_descriptor("model", "model/#{family}", "Primary operator model input", "model", "kyuubiki.operator.#{family}.input")],
      "outputs" => [operator_port_descriptor("result", "result/#{family}", "Primary operator result output", "result", "kyuubiki.operator.#{family}.output")],
      "validation" => verified_operator_validation_profile(family, ["workflow_graph", "orchestrated_api"])
    }
  end

  defp built_in_bridge_descriptor(id, domain, family, summary, capability_tags, config_schema, config_example) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "domain" => domain,
      "family" => family,
      "kind" => "workflow_bridge",
      "summary" => summary,
      "capability_tags" => capability_tags,
      "origin" => "built_in",
      "input_schema" => %{"schema" => "kyuubiki.operator.#{family}.bridge_input", "version" => "1"},
      "output_schema" => %{"schema" => "kyuubiki.operator.#{family}.bridge_output", "version" => "1"},
      "config_schema" => config_schema,
      "config_example" => config_example,
      "inputs" => [operator_port_descriptor("source", "result/#{family}_bridge_source", "Upstream workflow bridge payload", "upstream_result", "kyuubiki.operator.#{family}.bridge_input")],
      "outputs" => [operator_port_descriptor("bridged_model", "model/#{family}", "Downstream bridged model payload", "bridged_model", "kyuubiki.operator.#{family}.bridge_output")],
      "validation" => verified_operator_validation_profile(family, ["workflow_graph", "catalog_job"])
    }
  end

  defp built_in_extract_descriptor(id, domain, family, summary, capability_tags) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "domain" => domain,
      "family" => family,
      "kind" => "extract",
      "summary" => summary,
      "capability_tags" => capability_tags,
      "origin" => "built_in",
      "input_schema" => %{"schema" => "kyuubiki.operator.#{family}.extract_input", "version" => "1"},
      "output_schema" => %{"schema" => "kyuubiki.operator.#{family}.extract_output", "version" => "1"},
      "inputs" => [operator_port_descriptor("result", "result/any", "Result payload to extract from", "result", "kyuubiki.operator.#{family}.extract_input")],
      "outputs" => [operator_port_descriptor("summary", "extract/#{family}", "Extracted summary payload", "summary", "kyuubiki.operator.#{family}.extract_output")],
      "validation" => verified_operator_validation_profile(family, ["workflow_graph", "draft_builder"])
    }
  end

  defp built_in_export_descriptor(id, domain, family, summary, capability_tags) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "domain" => domain,
      "family" => family,
      "kind" => "export",
      "summary" => summary,
      "capability_tags" => capability_tags,
      "origin" => "built_in",
      "input_schema" => %{"schema" => "kyuubiki.operator.#{family}.export_input", "version" => "1"},
      "output_schema" => %{"schema" => "kyuubiki.operator.#{family}.export_output", "version" => "1"},
      "inputs" => [operator_port_descriptor("summary", "extract/result_summary", "Summary payload to export", "summary", "kyuubiki.operator.#{family}.export_input")],
      "outputs" => [operator_port_descriptor("export_artifact", "export/#{family}", "Exported delivery artifact", "export_artifact", "kyuubiki.operator.#{family}.export_output")],
      "validation" => verified_operator_validation_profile(family, ["workflow_graph", "draft_builder"])
    }
  end

  defp operator_port_descriptor(id, artifact_type, description, dataset_value, schema_ref) do
    %{"id" => id, "artifact_type" => artifact_type, "description" => description, "dataset_value" => dataset_value, "schema_ref" => %{"schema" => schema_ref, "version" => "1"}}
  end

  defp verified_operator_validation_profile(family, smoke_paths) do
    %{"baseline_status" => "verified", "baseline_cases" => ["#{family}_baseline"], "smoke_paths" => smoke_paths}
  end
end
