defmodule KyuubikiWeb.WorkflowBundleRuntime do
  @moduledoc false

  def compose_diagnostics_report_payload(payload, config)
      when is_map(payload) and is_map(config) do
    with %{} = bundle <- Map.get(payload, "bundle"),
         %{} = guard <- Map.get(payload, "guard") do
      include_guard = Map.get(config, "include_guard", true)
      include_bundle_items = Map.get(config, "include_bundle_items", true)

      {:ok,
       bundle
       |> maybe_drop("bundle_items", include_bundle_items)
       |> maybe_put(include_guard, "guard_payload", guard)
       |> Map.put("report_contract", "kyuubiki.workflow_report_payload/v1")
       |> Map.put("report_kind", "diagnostics_bundle_report_payload")
       |> Map.put("report_sources", Map.get(bundle, "bundle_sources", []))
       |> maybe_put(include_guard, "report_guard_status", Map.get(guard, "guard_status"))
       |> maybe_put(include_guard, "report_guard_recommendation", Map.get(guard, "guard_recommendation"))}
    else
      _ -> {:error, :invalid_diagnostics_report_payload}
    end
  end

  def compose_diagnostics_report_payload(_payload, _config),
    do: {:error, :invalid_diagnostics_report_payload}

  defp maybe_put(map, true, key, value), do: Map.put(map, key, value)
  defp maybe_put(map, _condition, _key, _value), do: map

  defp maybe_drop(map, _key, true), do: map
  defp maybe_drop(map, key, _condition), do: Map.delete(map, key)
end
