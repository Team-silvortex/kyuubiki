defmodule KyuubikiWeb.WorkflowBundleRuntime do
  @moduledoc false

  def compose_diagnostics_report_payload(payload, config)
      when is_map(payload) and is_map(config) do
    with %{} = bundle <- Map.get(payload, "bundle"),
         %{} = guard <- Map.get(payload, "guard") do
      include_guard = Map.get(config, "include_guard", true)
      include_bundle_items = Map.get(config, "include_bundle_items", true)
      focus_metrics = report_focus_metrics(bundle)
      highlights = report_highlights(focus_metrics, guard)

      {:ok,
       bundle
       |> maybe_drop("bundle_items", include_bundle_items)
       |> maybe_put(include_guard, "guard_payload", guard)
       |> Map.put("report_contract", "kyuubiki.workflow_report_payload/v1")
       |> Map.put("report_kind", "diagnostics_bundle_report_payload")
       |> Map.put("report_sources", Map.get(bundle, "bundle_sources", []))
       |> Map.put("report_focus_metrics", focus_metrics)
       |> Map.put("report_highlights", highlights)
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

  defp report_focus_metrics(%{"bundle_payloads" => payloads}) when is_map(payloads) do
    %{}
    |> maybe_insert_focus_metric(payloads, "electrostatic", ["electrostatic_potential_max"], "electrostatic.potential_max")
    |> maybe_insert_focus_metric(payloads, "electrostatic", ["electrostatic_peak_field", "electrostatic_field_peak_magnitude"], "electrostatic.field_peak")
    |> maybe_insert_focus_metric(payloads, "thermal", ["thermal_temperature_max"], "thermal.temperature_max")
    |> maybe_insert_focus_metric(payloads, "thermal", ["thermal_peak_flux", "thermal_flux_peak_magnitude"], "thermal.flux_peak")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_temperature_delta_max"], "thermo.temperature_delta_max")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_peak_displacement", "thermo_displacement_peak_magnitude"], "thermo.displacement_peak")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_peak_stress", "thermo_stress_peak"], "thermo.stress_peak")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"], "thermo.thermal_strain_peak")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_peak_mechanical_strain", "thermo_mechanical_strain_peak"], "thermo.mechanical_strain_peak")
    |> maybe_insert_focus_metric(payloads, "thermo", ["thermo_peak_total_strain", "thermo_total_strain_peak"], "thermo.total_strain_peak")
  end

  defp report_focus_metrics(_bundle), do: %{}

  defp maybe_insert_focus_metric(map, payloads, source, fields, key) do
    case Map.get(payloads, source) do
      payload when is_map(payload) ->
        case Enum.find_value(fields, &Map.get(payload, &1)) do
          nil -> map
          value -> Map.put(map, key, value)
        end

      _ ->
        map
    end
  end

  defp report_highlights(focus_metrics, guard) when is_map(focus_metrics) and is_map(guard) do
    triggered_fields =
      guard
      |> Map.get("guard_triggers", [])
      |> Enum.map(&Map.get(&1, "field"))

    []
    |> maybe_push_highlight(focus_metrics, "electrostatic.potential_max", "Electrostatic potential peak", triggered_fields, ["electrostatic_potential_max"])
    |> maybe_push_highlight(focus_metrics, "electrostatic.field_peak", "Electrostatic field peak", triggered_fields, ["electrostatic_peak_field", "electrostatic_field_peak_magnitude"])
    |> maybe_push_highlight(focus_metrics, "thermal.temperature_max", "Thermal temperature peak", triggered_fields, ["thermal_temperature_max"])
    |> maybe_push_highlight(focus_metrics, "thermo.temperature_delta_max", "Thermo temperature delta peak", triggered_fields, ["thermo_temperature_delta_max"])
    |> maybe_push_highlight(focus_metrics, "thermo.stress_peak", "Thermo stress peak", triggered_fields, ["thermo_peak_stress", "thermo_stress_peak"])
    |> maybe_push_highlight(focus_metrics, "thermo.thermal_strain_peak", "Thermo thermal strain peak", triggered_fields, ["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"])
  end

  defp report_highlights(_focus_metrics, _guard), do: []

  defp maybe_push_highlight(highlights, focus_metrics, metric_key, label, triggered_fields, source_fields) do
    case Map.fetch(focus_metrics, metric_key) do
      {:ok, value} ->
        attention = Enum.any?(source_fields, &(&1 in triggered_fields))
        highlights ++ [%{"id" => metric_key, "label" => label, "value" => value, "attention" => attention}]

      :error ->
        highlights
    end
  end
end
