defmodule KyuubikiWeb.WorkflowOperatorModules do
  @moduledoc false

  @domain_labels %{
    "electromagnetic" => "Electromagnetic",
    "mechanical" => "Structural",
    "thermal" => "Thermal",
    "thermo_mechanical" => "Thermo-Mechanical",
    "multi_domain" => "Multi-Domain"
  }

  @kind_labels %{
    "solver" => "Solvers",
    "workflow_bridge" => "Workflow Bridges",
    "transform" => "Transforms",
    "extract" => "Extractors",
    "export" => "Exporters"
  }

  def assign(%{"domain" => domain, "kind" => kind, "id" => operator_id} = descriptor) do
    module_id = module_id(domain, kind)

    Map.put_new(descriptor, "module", %{
      "id" => module_id,
      "label" => module_label(domain, kind),
      "domain" => domain,
      "kind" => kind,
      "lane" => lane_for(domain, kind),
      "operator_scope" => operator_scope(operator_id),
      "management" => %{
        "library_authority" => "central_operator_library",
        "agent_replication" => "forbidden",
        "agent_cache_policy" => "job_fetch",
        "ui_group" => ui_group(domain, kind)
      }
    })
  end

  def assign(descriptor), do: descriptor

  def summarize(operators) when is_list(operators) do
    operators
    |> Enum.group_by(&get_in(&1, ["module", "id"]))
    |> Enum.reject(fn {module_id, _operators} -> is_nil(module_id) end)
    |> Enum.map(fn {_module_id, grouped_operators} -> summarize_module(grouped_operators) end)
    |> Enum.sort_by(&{&1["lane"], &1["label"]})
  end

  defp module_id(domain, kind), do: "#{domain}.#{kind}"

  defp summarize_module([first | rest]) do
    module = first["module"]
    operators = [first | rest]
    validation_counts = validation_counts(operators)

    module
    |> Map.take(["id", "label", "domain", "kind", "lane", "operator_scope", "management"])
    |> Map.merge(%{
      "operator_count" => length(operators),
      "verified_count" => Map.get(validation_counts, "verified", 0),
      "partial_count" => Map.get(validation_counts, "partial", 0),
      "unverified_count" => Map.get(validation_counts, "unverified", 0),
      "capability_tags" => unique_capability_tags(operators),
      "operator_ids" => Enum.map(operators, & &1["id"]) |> Enum.sort()
    })
  end

  defp validation_counts(operators) do
    Enum.frequencies_by(operators, &get_in(&1, ["validation", "baseline_status"]))
  end

  defp unique_capability_tags(operators) do
    operators
    |> Enum.flat_map(&Map.get(&1, "capability_tags", []))
    |> Enum.uniq()
    |> Enum.sort()
  end

  defp module_label(domain, kind) do
    "#{Map.get(@domain_labels, domain, titleize(domain))} #{Map.get(@kind_labels, kind, titleize(kind))}"
  end

  defp lane_for(_domain, "workflow_bridge"), do: "coupling"
  defp lane_for(_domain, "transform"), do: "dataflow"
  defp lane_for(_domain, "extract"), do: "dataflow"
  defp lane_for(_domain, "export"), do: "delivery"

  defp lane_for(domain, "solver") when domain in ["mechanical", "thermal", "electromagnetic"],
    do: "physics"

  defp lane_for(_domain, _kind), do: "workflow"

  defp operator_scope("bridge." <> _), do: "coupling"
  defp operator_scope("export." <> _), do: "delivery"
  defp operator_scope("extract." <> _), do: "inspection"
  defp operator_scope("transform." <> _), do: "dataflow"
  defp operator_scope("solve." <> _), do: "physics"
  defp operator_scope(_), do: "workflow"

  defp ui_group(_domain, "workflow_bridge"), do: "Coupling"
  defp ui_group(_domain, "export"), do: "Delivery"
  defp ui_group(_domain, kind) when kind in ["transform", "extract"], do: "Dataflow"
  defp ui_group(domain, "solver"), do: Map.get(@domain_labels, domain, "Other Physics")
  defp ui_group(_domain, _kind), do: "Workflow"

  defp titleize(value) when is_binary(value) do
    value
    |> String.replace("_", " ")
    |> String.split(" ", trim: true)
    |> Enum.map_join(" ", &String.capitalize/1)
  end
end
