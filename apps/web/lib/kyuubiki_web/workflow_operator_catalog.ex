defmodule KyuubikiWeb.WorkflowOperatorCatalog do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowBuiltinOperatorRegistry
  alias KyuubikiWeb.WorkflowCatalogQuery
  alias KyuubikiWeb.WorkflowOperatorModules
  alias KyuubikiWeb.WorkflowSolverRegistry

  def catalog(filters \\ %{}) do
    operators = list(filters)
    %{"operators" => operators, "modules" => WorkflowOperatorModules.summarize(operators)}
  end

  def list(filters \\ %{}) do
    normalized_filters = WorkflowCatalogQuery.normalize_filters(filters)

    (Enum.map(WorkflowSolverRegistry.list(), &WorkflowSolverRegistry.descriptor/1) ++
       Enum.map(WorkflowBuiltinOperatorRegistry.list(), &built_in_descriptor/1))
    |> Enum.map(&WorkflowCatalogSupport.enrich_operator_descriptor/1)
    |> Enum.filter(&WorkflowCatalogQuery.matches_operator?(&1, normalized_filters))
  end

  def fetch(operator_id) when is_binary(operator_id) do
    case Enum.find(list(), &(&1["id"] == operator_id)) do
      nil -> {:error, {:operator_not_found, operator_id}}
      operator -> {:ok, %{"operator" => operator}}
    end
  end

  defp built_in_descriptor(
         %{"kind" => "workflow_bridge", "bridge_type" => "electrostatic_to_heat"} = spec
       ) do
    family = spec["family"]
    shape = spec["shape"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "workflow_bridge",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_output",
        "version" => "1"
      },
      "config_schema" => spec["config_schema"],
      "config_example" => spec["config_example"],
      "contract_support" => electrostatic_to_heat_contract_support(spec),
      "inputs" => [
        %{
          "id" => "electrostatic_result",
          "artifact_type" => spec["source_artifact_type"],
          "description" => spec["input_description"],
          "dataset_value" => "electrostatic_result",
          "schema_ref" => %{
            "schema" => "kyuubiki.operator.electrostatic_plane_#{shape}_2d.output",
            "version" => "1"
          }
        }
      ],
      "outputs" => [
        %{
          "id" => "heat_model",
          "artifact_type" => spec["target_artifact_type"],
          "description" => spec["output_description"],
          "dataset_value" => "heat_model",
          "schema_ref" => %{
            "schema" => "kyuubiki.operator.heat_plane_#{shape}_2d.input",
            "version" => "1"
          }
        }
      ],
      "validation" => validation_profile(family, ["workflow_graph", "catalog_job"])
    }
  end

  defp built_in_descriptor(
         %{"kind" => "workflow_bridge", "bridge_type" => "heat_to_thermo"} = spec
       ) do
    family = spec["family"]
    shape = spec["shape"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "workflow_bridge",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_output",
        "version" => "1"
      },
      "config_schema" => spec["config_schema"],
      "config_example" => spec["config_example"],
      "contract_support" => heat_to_thermo_contract_support(spec),
      "inputs" => [
        %{
          "id" => "heat_result",
          "artifact_type" => spec["source_artifact_type"],
          "description" => spec["input_description"],
          "dataset_value" => "heat_result",
          "schema_ref" => %{
            "schema" => "kyuubiki.operator.heat_plane_#{shape}_2d.output",
            "version" => "1"
          }
        }
      ],
      "outputs" => [
        %{
          "id" => "thermo_model",
          "artifact_type" => spec["target_artifact_type"],
          "description" => spec["output_description"],
          "dataset_value" => "thermo_model",
          "schema_ref" => %{
            "schema" => "kyuubiki.operator.thermal_plane_#{shape}_2d.input",
            "version" => "1"
          }
        }
      ],
      "validation" => validation_profile(family, ["workflow_graph", "catalog_job"])
    }
  end

  defp built_in_descriptor(%{"kind" => "workflow_bridge"} = spec) do
    family = spec["family"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "workflow_bridge",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.bridge_output",
        "version" => "1"
      },
      "config_schema" => spec["config_schema"],
      "config_example" => spec["config_example"],
      "inputs" => [
        port_descriptor(
          "source",
          "result/#{family}_bridge_source",
          "Upstream workflow bridge payload",
          "upstream_result",
          "kyuubiki.operator.#{family}.bridge_input"
        )
      ],
      "outputs" => [
        port_descriptor(
          "bridged_model",
          "model/#{family}",
          "Downstream bridged model payload",
          "bridged_model",
          "kyuubiki.operator.#{family}.bridge_output"
        )
      ],
      "validation" => validation_profile(family, ["workflow_graph", "catalog_job"])
    }
  end

  defp built_in_descriptor(%{"kind" => "transform"} = spec) do
    family = spec["family"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "transform",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.transform_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.transform_output",
        "version" => "1"
      },
      "inputs" => [
        port_descriptor(
          "value",
          "artifact/json",
          "Transform payload to process",
          "value",
          "kyuubiki.operator.#{family}.transform_input"
        )
      ],
      "outputs" => [
        port_descriptor(
          "result",
          "artifact/json",
          "Transformed payload output",
          "result",
          "kyuubiki.operator.#{family}.transform_output"
        )
      ],
      "validation" => validation_profile(family, ["workflow_graph", "draft_builder"])
    }
  end

  defp built_in_descriptor(%{"kind" => "extract"} = spec) do
    family = spec["family"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "extract",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.extract_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.extract_output",
        "version" => "1"
      },
      "inputs" => [
        port_descriptor(
          "result",
          "result/any",
          "Result payload to extract from",
          "result",
          "kyuubiki.operator.#{family}.extract_input"
        )
      ],
      "outputs" => [
        port_descriptor(
          "summary",
          "extract/#{family}",
          "Extracted summary payload",
          "summary",
          "kyuubiki.operator.#{family}.extract_output"
        )
      ],
      "validation" => validation_profile(family, ["workflow_graph", "draft_builder"])
    }
  end

  defp built_in_descriptor(%{"kind" => "export"} = spec) do
    family = spec["family"]

    %{
      "id" => spec["id"],
      "version" => "1.0.0",
      "domain" => spec["domain"],
      "family" => family,
      "kind" => "export",
      "summary" => spec["summary"],
      "capability_tags" => spec["capability_tags"],
      "origin" => "built_in",
      "input_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.export_input",
        "version" => "1"
      },
      "output_schema" => %{
        "schema" => "kyuubiki.operator.#{family}.export_output",
        "version" => "1"
      },
      "inputs" => [
        port_descriptor(
          "summary",
          "extract/result_summary",
          "Summary payload to export",
          "summary",
          "kyuubiki.operator.#{family}.export_input"
        )
      ],
      "outputs" => [
        port_descriptor(
          "export_artifact",
          "export/#{family}",
          "Exported delivery artifact",
          "export_artifact",
          "kyuubiki.operator.#{family}.export_output"
        )
      ],
      "validation" => validation_profile(family, ["workflow_graph", "draft_builder"])
    }
  end

  defp port_descriptor(id, artifact_type, description, dataset_value, schema_ref) do
    %{
      "id" => id,
      "artifact_type" => artifact_type,
      "description" => description,
      "dataset_value" => dataset_value,
      "schema_ref" => %{"schema" => schema_ref, "version" => "1"}
    }
  end

  defp validation_profile(family, smoke_paths) do
    %{
      "baseline_status" => "verified",
      "baseline_cases" => ["#{family}_baseline"],
      "smoke_paths" => smoke_paths
    }
  end

  defp electrostatic_to_heat_contract_support(%{"shape" => "triangle"}) do
    %{
      "source" => %{
        "fields" => [
          "potential",
          "charge_density",
          "electric_field_magnitude",
          "electric_field_x",
          "electric_field_y",
          "average_potential",
          "flux_magnitude",
          "electric_flux_density_magnitude",
          "electric_flux_density_x",
          "electric_flux_density_y"
        ],
        "distributions" => %{
          "node_to_node" => ["potential", "charge_density"],
          "element_to_nodes" => [
            "electric_field_magnitude",
            "electric_field_x",
            "electric_field_y",
            "average_potential",
            "flux_magnitude",
            "electric_flux_density_magnitude",
            "electric_flux_density_x",
            "electric_flux_density_y"
          ]
        },
        "node_index_fields" => ["node_i", "node_j", "node_k"]
      },
      "transform" => %{
        "reductions" => ["mean", "sum", "area_weighted_mean", "min", "max"],
        "default_reduction_by_distribution" => %{
          "node_to_node" => "mean",
          "element_to_nodes" => "mean"
        },
        "default_scale" => 1.0,
        "default_value" => 0.0
      },
      "target" => %{"fields" => ["heat_load", "temperature"], "default_field" => "heat_load"}
    }
  end

  defp electrostatic_to_heat_contract_support(%{"shape" => "quad"}) do
    %{
      "source" => %{
        "fields" => [
          "potential",
          "charge_density",
          "electric_field_magnitude",
          "electric_field_x",
          "electric_field_y",
          "average_potential",
          "flux_magnitude",
          "electric_flux_density_magnitude",
          "electric_flux_density_x",
          "electric_flux_density_y"
        ],
        "distributions" => %{
          "node_to_node" => ["potential", "charge_density"],
          "element_to_nodes" => [
            "electric_field_magnitude",
            "electric_field_x",
            "electric_field_y",
            "average_potential",
            "flux_magnitude",
            "electric_flux_density_magnitude",
            "electric_flux_density_x",
            "electric_flux_density_y"
          ]
        },
        "node_index_fields" => ["node_i", "node_j", "node_k", "node_l"]
      },
      "transform" => %{
        "reductions" => ["mean", "sum", "area_weighted_mean", "min", "max"],
        "default_reduction_by_distribution" => %{
          "node_to_node" => "mean",
          "element_to_nodes" => "mean"
        },
        "default_scale" => 1.0,
        "default_value" => 0.0
      },
      "target" => %{"fields" => ["heat_load", "temperature"], "default_field" => "heat_load"}
    }
  end

  defp heat_to_thermo_contract_support(%{"shape" => "triangle"}) do
    %{
      "source" => %{
        "fields" => [
          "temperature",
          "heat_load",
          "average_temperature",
          "heat_flux_x",
          "heat_flux_y",
          "heat_flux",
          "heat_flux_magnitude"
        ],
        "distributions" => %{
          "node_to_node" => ["temperature", "heat_load"],
          "element_to_nodes" => [
            "average_temperature",
            "heat_flux_x",
            "heat_flux_y",
            "heat_flux",
            "heat_flux_magnitude"
          ]
        },
        "node_index_fields" => ["node_i", "node_j", "node_k"]
      },
      "transform" => %{
        "reductions" => ["copy", "mean", "sum", "area_weighted_mean", "min", "max"],
        "default_reduction_by_distribution" => %{
          "node_to_node" => "copy",
          "element_to_nodes" => "mean"
        },
        "default_scale" => 1.0,
        "default_value" => 0.0
      },
      "target" => %{"fields" => ["temperature_delta"], "default_field" => "temperature_delta"}
    }
  end

  defp heat_to_thermo_contract_support(%{"shape" => "quad"}) do
    %{
      "source" => %{
        "fields" => [
          "temperature",
          "heat_load",
          "average_temperature",
          "heat_flux_x",
          "heat_flux_y",
          "heat_flux",
          "heat_flux_magnitude"
        ],
        "distributions" => %{
          "node_to_node" => ["temperature", "heat_load"],
          "element_to_nodes" => [
            "average_temperature",
            "heat_flux_x",
            "heat_flux_y",
            "heat_flux",
            "heat_flux_magnitude"
          ]
        },
        "node_index_fields" => ["node_i", "node_j", "node_k", "node_l"]
      },
      "transform" => %{
        "reductions" => ["copy", "mean", "sum", "area_weighted_mean", "min", "max"],
        "default_reduction_by_distribution" => %{
          "node_to_node" => "copy",
          "element_to_nodes" => "mean"
        },
        "default_scale" => 1.0,
        "default_value" => 0.0
      },
      "target" => %{"fields" => ["temperature_delta"], "default_field" => "temperature_delta"}
    }
  end
end
