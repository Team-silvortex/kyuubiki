defmodule KyuubikiWeb.WorkflowCatalogSupport do
  @moduledoc false

  alias KyuubikiWeb.WorkflowOperatorModules
  alias KyuubikiWeb.WorkflowOperatorCategoryTaxonomy

  def workflow_dataset_contract(id, values, metadata \\ %{}) when is_list(values) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "values" => values,
      "metadata" => metadata
    }
  end

  def workflow_dataset_value_info(id, data_class, semantic_type, element_type \\ "json_object") do
    %{
      "id" => id,
      "data_class" => data_class,
      "element_type" => element_type,
      "shape" => %{"axes" => []},
      "semantic_type" => semantic_type,
      "unit" => nil,
      "encoding" => nil,
      "schema_ref" => nil
    }
  end

  def enrich_operator_descriptor(%{"id" => operator_id} = descriptor)
      when is_binary(operator_id) do
    descriptor
    |> WorkflowOperatorCategoryTaxonomy.assign()
    |> WorkflowOperatorModules.assign()
    |> Map.put_new(
      "execution",
      %{
        "authority_mode" => "central_operator_library",
        "execution_mode" => "orchestra_fetch",
        "source_ref" => "orchestra://operator/#{operator_id}",
        "package_ref" => "orchestra://operator-package/#{operator_id}",
        "package_version" => "library-managed",
        "integrity" => nil,
        "placement_tags" => derive_operator_placement_tags(operator_id),
        "required_capabilities" => derive_operator_required_capabilities(operator_id),
        "cache_scope" => "job",
        "agent_fetchable" => true
      }
    )
  end

  def enrich_operator_descriptor(descriptor), do: descriptor

  def enrich_workflow_descriptor(%{"graph" => graph} = workflow) when is_map(graph) do
    enriched_graph = enrich_workflow_graph(graph)

    workflow
    |> Map.put("graph", enriched_graph)
    |> Map.put("runtime_manifest", build_workflow_runtime_manifest(enriched_graph))
  end

  def enrich_workflow_descriptor(workflow), do: workflow

  def enrich_workflow_graph(graph) when is_map(graph) do
    required_operator_ids = required_operator_ids(graph)
    graph_placement_tags = derive_graph_placement_tags(required_operator_ids)
    graph_required_capabilities = derive_graph_required_capabilities(required_operator_ids)
    simple_fetch_plan = build_simple_operator_fetch_plan(graph)

    graph
    |> update_in(
      ["defaults"],
      &enrich_graph_defaults(&1, graph_placement_tags, graph_required_capabilities)
    )
    |> Map.put_new("dispatch_policy", "central_fetch")
    |> Map.put_new("operator_fetch_plan", simple_fetch_plan)
    |> Map.put_new("placement_tags", graph_placement_tags)
    |> Map.put_new("required_capabilities", graph_required_capabilities)
    |> update_in(["nodes"], &enrich_graph_nodes/1)
  end

  def enrich_workflow_graph(graph), do: graph

  def build_workflow_runtime_manifest(graph) when is_map(graph) do
    required_operator_ids = required_operator_ids(graph)

    %{
      "required_operator_ids" => required_operator_ids,
      "sample_input_node_ids" => Map.get(graph, "entry_nodes", []),
      "included_input_text_node_ids" => [],
      "bridge_seed_summaries" => build_bridge_seed_summaries(graph),
      "dispatch_policy" => %{
        "authority_mode" => "central_operator_library",
        "agent_cache_policy" => "ephemeral_fetch",
        "missing_operator_behavior" => "fetch_from_orchestra",
        "agent_library_replication" => "forbidden"
      },
      "operator_fetch_plan" => build_runtime_operator_fetch_plan(graph)
    }
  end

  def build_workflow_runtime_manifest(_graph), do: %{}

  def derive_dataset_lineage(graph, artifact_lineage)
      when is_map(graph) and is_list(artifact_lineage) do
    dataset_contract =
      case Map.get(graph, "dataset_contract") do
        value when is_map(value) -> value
        _ -> %{}
      end

    dataset_info_by_value = dataset_info_by_value(dataset_contract)
    port_info = build_port_info(graph)
    artifact_dataset_map = build_artifact_dataset_map(artifact_lineage, port_info)

    Enum.reduce(artifact_lineage, [], fn entry, acc ->
      artifact_key = Map.get(entry, "artifact_key")

      case Map.get(artifact_dataset_map, artifact_key) do
        dataset_value when is_binary(dataset_value) ->
          info = Map.get(dataset_info_by_value, dataset_value, %{})

          acc ++
            [
              %{
                "artifact_key" => artifact_key,
                "node_id" => Map.get(entry, "node_id"),
                "port_id" => Map.get(entry, "port_id"),
                "artifact_type" => port_artifact_type(port_info, entry),
                "dataset_value" => dataset_value,
                "contract_id" => Map.get(dataset_contract, "id"),
                "dataset_role" => Map.get(info, "data_class"),
                "dataset_format" => Map.get(info, "element_type"),
                "semantic_type" => Map.get(info, "semantic_type"),
                "source_datasets" => source_datasets(entry, artifact_dataset_map)
              }
            ]

        _ ->
          acc
      end
    end)
  end

  def derive_dataset_lineage(_graph, _artifact_lineage), do: []

  def electrostatic_to_heat_bridge_contract_example(
        scale,
        node_index_fields \\ ["node_i", "node_j", "node_k", "node_l"]
      ) do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => node_index_fields
      },
      "transform" => %{
        "scale" => scale,
        "reduction" => "mean",
        "default_value" => 0.0
      },
      "target" => %{"field" => "heat_load"}
    }
  end

  def heat_to_thermo_bridge_contract_example do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{"field" => "temperature"},
      "transform" => %{"scale" => 1.0, "default_value" => 0.0},
      "target" => %{"field" => "temperature_delta"}
    }
  end

  def thermo_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "h0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "tq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.02,
          "youngs_modulus" => 210.0e9,
          "poisson_ratio" => 0.3,
          "thermal_expansion" => 11.0e-6
        }
      ]
    }
  end

  def thermo_triangle_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "t0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "t1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "t2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "tt0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.02,
          "youngs_modulus" => 210.0e9,
          "poisson_ratio" => 0.3,
          "thermal_expansion" => 11.0e-6
        }
      ]
    }
  end

  defp dataset_info_by_value(%{"values" => values}) when is_list(values) do
    Enum.reduce(values, %{}, fn value, acc ->
      case Map.get(value, "id") do
        dataset_value when is_binary(dataset_value) -> Map.put(acc, dataset_value, value)
        _ -> acc
      end
    end)
  end

  defp dataset_info_by_value(_dataset_contract), do: %{}

  defp build_port_info(graph) do
    Enum.reduce(Map.get(graph, "nodes", []), %{}, fn node, acc ->
      node_id = Map.get(node, "id")

      acc
      |> merge_ports(node_id, Map.get(node, "inputs", []))
      |> merge_ports(node_id, Map.get(node, "outputs", []))
    end)
  end

  defp merge_ports(acc, node_id, ports) when is_binary(node_id) and is_list(ports) do
    Enum.reduce(ports, acc, fn port, port_acc ->
      case Map.get(port, "id") do
        port_id when is_binary(port_id) ->
          Map.put(port_acc, {node_id, port_id}, %{
            "dataset_value" => Map.get(port, "dataset_value"),
            "artifact_type" => Map.get(port, "artifact_type")
          })

        _ ->
          port_acc
      end
    end)
  end

  defp merge_ports(acc, _node_id, _ports), do: acc

  defp build_artifact_dataset_map(artifact_lineage, port_info) do
    Enum.reduce(artifact_lineage, %{}, fn entry, acc ->
      dataset_value =
        port_info
        |> Map.get({Map.get(entry, "node_id"), Map.get(entry, "port_id")}, %{})
        |> Map.get("dataset_value")

      if is_binary(dataset_value) do
        Map.put(acc, Map.get(entry, "artifact_key"), dataset_value)
      else
        acc
      end
    end)
  end

  defp source_datasets(entry, artifact_dataset_map) do
    entry
    |> Map.get("source_artifacts", [])
    |> Enum.map(&Map.get(artifact_dataset_map, &1))
    |> Enum.filter(&is_binary/1)
    |> Enum.uniq()
  end

  defp port_artifact_type(port_info, entry) do
    port_info
    |> Map.get({Map.get(entry, "node_id"), Map.get(entry, "port_id")}, %{})
    |> Map.get("artifact_type")
  end

  defp enrich_graph_defaults(nil, placement_tags, required_capabilities) do
    %{
      "cache_policy" => "cached",
      "orchestrated" => true,
      "dispatch_policy" => "central_fetch",
      "placement_tags" => placement_tags,
      "required_capabilities" => required_capabilities
    }
  end

  defp enrich_graph_defaults(defaults, placement_tags, required_capabilities)
       when is_map(defaults) do
    defaults
    |> Map.put_new("dispatch_policy", "central_fetch")
    |> Map.put_new("placement_tags", placement_tags)
    |> Map.put_new("required_capabilities", required_capabilities)
  end

  defp enrich_graph_defaults(defaults, _placement_tags, _required_capabilities), do: defaults

  defp enrich_graph_nodes(nodes) when is_list(nodes) do
    Enum.map(nodes, fn
      %{"operator_id" => operator_id} = node when is_binary(operator_id) ->
        node
        |> Map.put_new("placement_tags", derive_operator_placement_tags(operator_id))
        |> Map.put_new(
          "required_capabilities",
          derive_operator_required_capabilities(operator_id)
        )

      node ->
        node
    end)
  end

  defp enrich_graph_nodes(nodes), do: nodes

  defp required_operator_ids(graph) do
    graph
    |> Map.get("nodes", [])
    |> Enum.map(&Map.get(&1, "operator_id"))
    |> Enum.filter(&is_binary/1)
    |> unique_sorted()
  end

  defp build_simple_operator_fetch_plan(graph) do
    graph
    |> Map.get("nodes", [])
    |> Enum.flat_map(fn
      %{"id" => node_id, "operator_id" => operator_id}
      when is_binary(node_id) and is_binary(operator_id) ->
        [
          %{
            "node_id" => node_id,
            "operator_id" => operator_id,
            "package_ref" => "orchestra://operator-package/#{operator_id}",
            "version" => "library-managed",
            "integrity" => nil,
            "cache_scope" => "job"
          }
        ]

      _ ->
        []
    end)
  end

  defp build_runtime_operator_fetch_plan(graph) do
    graph
    |> Map.get("nodes", [])
    |> Enum.flat_map(fn
      %{"id" => node_id, "operator_id" => operator_id}
      when is_binary(node_id) and is_binary(operator_id) ->
        [
          %{
            "node_id" => node_id,
            "operator_id" => operator_id,
            "execution_mode" => "orchestra_fetch",
            "source_ref" => "orchestra://operator/#{operator_id}",
            "package_ref" => "orchestra://operator-package/#{operator_id}",
            "package_version" => "library-managed",
            "integrity" => nil,
            "placement_tags" => derive_operator_placement_tags(operator_id),
            "required_capabilities" => derive_operator_required_capabilities(operator_id),
            "cache_scope" => "job"
          }
        ]

      _ ->
        []
    end)
  end

  defp build_bridge_seed_summaries(graph) do
    graph
    |> Map.get("nodes", [])
    |> Enum.flat_map(fn
      %{"operator_id" => "bridge." <> _rest, "config" => config} = node when is_map(config) ->
        seed_model =
          case Map.get(config, "seed_model") do
            value when is_map(value) -> value
            _ -> config
          end

        nodes = if is_list(seed_model["nodes"]), do: seed_model["nodes"], else: []
        elements = if is_list(seed_model["elements"]), do: seed_model["elements"], else: []
        contract = if is_map(config["contract"]), do: config["contract"], else: %{}

        [
          %{
            "node_id" => Map.get(node, "id"),
            "operator_id" => Map.get(node, "operator_id"),
            "node_count" => length(nodes),
            "element_count" => length(elements),
            "contract_version" => Map.get(contract, "version")
          }
        ]

      _ ->
        []
    end)
  end

  defp derive_graph_placement_tags(operator_ids) do
    operator_ids
    |> Enum.flat_map(&derive_operator_placement_tags/1)
    |> unique_sorted()
  end

  defp derive_graph_required_capabilities(operator_ids) do
    operator_ids
    |> Enum.flat_map(&derive_operator_required_capabilities/1)
    |> unique_sorted()
  end

  defp derive_operator_placement_tags(operator_id) when is_binary(operator_id) do
    normalized = String.downcase(operator_id)

    [
      String.contains?(normalized, "electrostatic") && "electromagnetic",
      String.contains?(normalized, "thermal") && "thermo_mechanical",
      String.contains?(normalized, "heat") && "thermal",
      String.contains?(normalized, "frame") && "frame",
      String.contains?(normalized, "truss") && "truss",
      String.contains?(normalized, "plane") && "mesh",
      String.starts_with?(normalized, "bridge.") && "bridge",
      String.starts_with?(normalized, "extract.") && "postprocess",
      String.starts_with?(normalized, "transform.") && "transform",
      String.starts_with?(normalized, "export.") && "export"
    ]
    |> Enum.filter(&is_binary/1)
    |> unique_sorted()
  end

  defp derive_operator_required_capabilities(operator_id) when is_binary(operator_id) do
    normalized = String.downcase(operator_id)

    [
      String.starts_with?(normalized, "solve.") && "solver_rpc",
      String.starts_with?(normalized, "bridge.") && "workflow_bridge_runtime",
      String.starts_with?(normalized, "transform.") && "workflow_transform_runtime",
      String.starts_with?(normalized, "extract.") && "workflow_extract_runtime",
      String.starts_with?(normalized, "export.") && "workflow_export_runtime"
    ]
    |> Enum.filter(&is_binary/1)
    |> unique_sorted()
  end

  defp unique_sorted(values) do
    values
    |> Enum.filter(&(is_binary(&1) and String.trim(&1) != ""))
    |> Enum.uniq()
    |> Enum.sort()
  end
end
