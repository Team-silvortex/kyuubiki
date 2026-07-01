defmodule KyuubikiWeb.WorkflowCatalogQuery do
  @moduledoc false

  def normalize_filters(filters) when is_map(filters) do
    filters
    |> Enum.reduce(%{}, fn
      {key, value}, acc when is_atom(key) -> put_filter(acc, Atom.to_string(key), value)
      {key, value}, acc when is_binary(key) -> put_filter(acc, key, value)
      _, acc -> acc
    end)
  end

  def matches_operator?(descriptor, filters) when is_map(descriptor) and is_map(filters) do
    matches_query?(operator_haystack(descriptor), Map.get(filters, "q", "")) and
      matches_value?(descriptor["domain"], Map.get(filters, "domain", "")) and
      matches_value?(descriptor["kind"], Map.get(filters, "kind", "")) and
      matches_value?(descriptor["operator_category_id"], Map.get(filters, "category", "")) and
      matches_value?(
        get_in(descriptor, ["validation", "baseline_status"]),
        Map.get(filters, "validation", "")
      ) and
      matches_value?(get_in(descriptor, ["module", "id"]), Map.get(filters, "module", "")) and
      matches_capability?(descriptor["capability_tags"], Map.get(filters, "capability", ""))
  end

  def matches_workflow?(workflow, filters) when is_map(workflow) and is_map(filters) do
    matches_query?(workflow_haystack(workflow), Map.get(filters, "q", "")) and
      matches_membership?(workflow["domains"], Map.get(filters, "domain", "")) and
      matches_capability?(workflow["capability_tags"], Map.get(filters, "capability", "")) and
      matches_membership?(workflow_operator_ids(workflow), Map.get(filters, "operator_id", "")) and
      matches_artifact?(workflow["entry_inputs"], Map.get(filters, "entry_artifact", "")) and
      matches_artifact?(workflow["output_artifacts"], Map.get(filters, "output_artifact", ""))
  end

  defp put_filter(acc, _key, value) when value in [nil, ""], do: acc

  defp put_filter(acc, key, value) when is_binary(value) do
    trimmed = String.trim(value)
    if trimmed == "", do: acc, else: Map.put(acc, key, trimmed)
  end

  defp put_filter(acc, key, value), do: Map.put(acc, key, to_string(value))

  defp matches_query?(_haystack, ""), do: true

  defp matches_query?(haystack, query) do
    String.contains?(String.downcase(haystack), String.downcase(query))
  end

  defp matches_value?(_value, ""), do: true
  defp matches_value?(value, expected) when is_binary(value), do: value == expected
  defp matches_value?(_, _), do: false

  defp matches_capability?(_values, ""), do: true
  defp matches_capability?(values, expected) when is_list(values), do: expected in values
  defp matches_capability?(_, _), do: false

  defp matches_membership?(_values, ""), do: true
  defp matches_membership?(values, expected) when is_list(values), do: expected in values
  defp matches_membership?(_, _), do: false

  defp matches_artifact?(_artifacts, ""), do: true

  defp matches_artifact?(artifacts, expected) when is_list(artifacts) do
    Enum.any?(artifacts, &(Map.get(&1, "artifact_type") == expected))
  end

  defp matches_artifact?(_, _), do: false

  defp operator_haystack(descriptor) do
    [
      descriptor["id"],
      descriptor["family"],
      descriptor["domain"],
      descriptor["kind"],
      descriptor["operator_category_id"],
      get_in(descriptor, ["operator_category", "label"]),
      get_in(descriptor, ["module", "id"]),
      get_in(descriptor, ["module", "label"]),
      get_in(descriptor, ["module", "lane"]),
      descriptor["summary"],
      Enum.join(descriptor["capability_tags"] || [], " ")
    ]
    |> Enum.filter(&is_binary/1)
    |> Enum.join(" ")
  end

  defp workflow_haystack(workflow) do
    [
      workflow["id"],
      workflow["name"],
      workflow["summary"],
      Enum.join(workflow["domains"] || [], " "),
      Enum.join(workflow["capability_tags"] || [], " "),
      Enum.join(workflow_operator_ids(workflow), " ")
    ]
    |> Enum.filter(&is_binary/1)
    |> Enum.join(" ")
  end

  defp workflow_operator_ids(workflow) do
    case get_in(workflow, ["graph", "nodes"]) do
      nodes when is_list(nodes) ->
        nodes
        |> Enum.map(&Map.get(&1, "operator_id"))
        |> Enum.filter(&is_binary/1)

      _ ->
        []
    end
  end
end
