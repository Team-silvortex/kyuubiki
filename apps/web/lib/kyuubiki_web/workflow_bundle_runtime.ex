defmodule KyuubikiWeb.WorkflowBundleRuntime do
  @moduledoc false

  def compose_diagnostics_report_payload(payload, config)
      when is_map(payload) and is_map(config) do
    with %{} = bundle <- Map.get(payload, "bundle"),
         %{} = guard <- Map.get(payload, "guard") do
      include_guard = Map.get(config, "include_guard", true)
      include_bundle_items = Map.get(config, "include_bundle_items", true)
      focus_metrics = report_focus_metrics(bundle)
      focus_context = report_focus_context(bundle)
      focus_payloads = report_focus_payloads(bundle)
      highlights = report_highlights(focus_metrics, guard)

      {:ok,
       bundle
       |> maybe_drop("bundle_items", include_bundle_items)
       |> maybe_put(include_guard, "guard_payload", guard)
       |> Map.put("report_contract", "kyuubiki.workflow_report_payload/v1")
       |> Map.put("report_kind", "diagnostics_bundle_report_payload")
       |> Map.put("report_sources", Map.get(bundle, "bundle_sources", []))
       |> Map.put("report_focus_metrics", focus_metrics)
       |> Map.put("report_focus_context", focus_context)
       |> Map.put("report_focus_payloads", focus_payloads)
       |> Map.put("report_highlights", highlights)
       |> maybe_put(include_guard, "report_guard_status", Map.get(guard, "guard_status"))
       |> maybe_put(
         include_guard,
         "report_guard_recommendation",
         Map.get(guard, "guard_recommendation")
       )}
    else
      _ -> {:error, :invalid_diagnostics_report_payload}
    end
  end

  def compose_diagnostics_report_payload(_payload, _config),
    do: {:error, :invalid_diagnostics_report_payload}

  def select_focus_payload(payload, %{"metric_id" => metric_id})
      when is_map(payload) and is_binary(metric_id) do
    case resolve_focus_payload(payload, %{"metric_id" => metric_id}) do
      {:ok, focus_payload} -> {:ok, focus_payload}
      {:error, _reason} -> {:error, :focus_metric_not_found}
    end
  end

  def select_focus_payload(_payload, _config), do: {:error, :invalid_focus_payload_selection}

  def compose_focus_chain_input(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, %{} = focus_payload} <- resolve_focus_payload(payload, config),
         metric_id when is_binary(metric_id) <- Map.get(focus_payload, "metric_id") do
      {:ok,
       %{
         "chain_contract" => "kyuubiki.workflow_focus_chain_input/v1",
         "chain_kind" => "focus_chain_input",
         "metric_id" => metric_id,
         "source" => Map.get(focus_payload, "source"),
         "value" => Map.get(focus_payload, "value"),
         "value_field" => Map.get(focus_payload, "value_field"),
         "focus_payload" => focus_payload,
         "context" => Map.get(focus_payload, "context", %{}),
         "bindings" => Map.get(config, "bindings", %{}),
         "annotations" => Map.get(config, "annotations", %{})
       }
       |> maybe_put(
         Map.has_key?(config, "target_operator"),
         "target_operator",
         Map.get(config, "target_operator")
       )
       |> maybe_put(Map.has_key?(config, "stage"), "stage", Map.get(config, "stage"))}
    else
      _ -> {:error, :invalid_focus_chain_input}
    end
  end

  def compose_focus_chain_input(_payload, _config), do: {:error, :invalid_focus_chain_input}

  def compose_focus_bridge_request(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, %{} = chain_input} <- resolve_focus_chain_input(payload, config),
         bridge_operator when is_binary(bridge_operator) <-
           Map.get(config, "bridge_operator") || Map.get(chain_input, "target_operator"),
         true <- String.starts_with?(bridge_operator, "bridge.") do
      {:ok,
       %{
         "request_contract" => "kyuubiki.workflow_focus_bridge_request/v1",
         "request_kind" => "focus_bridge_request",
         "bridge_operator" => bridge_operator,
         "metric_id" => Map.get(chain_input, "metric_id"),
         "source" => Map.get(chain_input, "source"),
         "focus_value" => Map.get(chain_input, "value"),
         "focus_chain_input" => chain_input,
         "focus_payload" => Map.get(chain_input, "focus_payload"),
         "bridge_config" => %{
           "seed_model" => Map.get(config, "seed_model"),
           "contract" => Map.get(config, "contract")
         },
         "bindings" => Map.get(chain_input, "bindings", %{}),
         "annotations" => Map.get(chain_input, "annotations", %{})
       }
       |> maybe_put(Map.has_key?(chain_input, "stage"), "stage", Map.get(chain_input, "stage"))
       |> maybe_put(
         Map.has_key?(config, "bridge_payload_source"),
         "bridge_payload_source",
         Map.get(config, "bridge_payload_source")
       )}
    else
      _ -> {:error, :invalid_focus_bridge_request}
    end
  end

  def compose_focus_bridge_request(_payload, _config), do: {:error, :invalid_focus_bridge_request}

  def resolve_focus_bridge_execution(payload, config) when is_map(payload) and is_map(config) do
    with %{"execution_contract" => "kyuubiki.workflow_focus_bridge_execution/v1"} = execution <-
           payload do
      {:ok, execution}
    else
      _ ->
        with {:ok, %{} = bridge_request} <- resolve_named_focus_bridge_request(payload, config),
             bridge_operator when is_binary(bridge_operator) <-
               Map.get(bridge_request, "bridge_operator"),
             %{} = bridge_config <- Map.get(bridge_request, "bridge_config", %{}),
             {:ok, bridge_payload} <- resolve_named_bridge_payload(payload, config) do
          {:ok,
           %{
             "execution_contract" => "kyuubiki.workflow_focus_bridge_execution/v1",
             "execution_kind" => "focus_bridge_execution",
             "operator_id" => bridge_operator,
             "bridge_payload" => bridge_payload,
             "bridge_config" => bridge_config,
             "bridge_request" => bridge_request,
             "metric_id" => Map.get(bridge_request, "metric_id"),
             "focus_value" => Map.get(bridge_request, "focus_value"),
             "bindings" => Map.get(bridge_request, "bindings", %{}),
             "annotations" => Map.get(bridge_request, "annotations", %{}),
             "bridge_payload_source" =>
               Map.get(
                 config,
                 "bridge_payload_source",
                 Map.get(bridge_request, "bridge_payload_source")
               )
           }}
        else
          _ -> {:error, :invalid_focus_bridge_execution}
        end
    end
  end

  def resolve_focus_bridge_execution(_payload, _config),
    do: {:error, :invalid_focus_bridge_execution}

  defp maybe_put(map, true, key, value), do: Map.put(map, key, value)
  defp maybe_put(map, _condition, _key, _value), do: map

  defp maybe_drop(map, _key, true), do: map
  defp maybe_drop(map, key, _condition), do: Map.delete(map, key)

  defp report_focus_metrics(%{"bundle_payloads" => payloads}) when is_map(payloads) do
    %{}
    |> maybe_insert_focus_metric(
      payloads,
      "electrostatic",
      ["electrostatic_potential_max"],
      "electrostatic.potential_max"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "electrostatic",
      ["electrostatic_peak_field", "electrostatic_field_peak_magnitude"],
      "electrostatic.field_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermal",
      ["thermal_temperature_max"],
      "thermal.temperature_max"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermal",
      ["thermal_peak_flux", "thermal_flux_peak_magnitude"],
      "thermal.flux_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_temperature_delta_max"],
      "thermo.temperature_delta_max"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_peak_displacement", "thermo_displacement_peak_magnitude"],
      "thermo.displacement_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_peak_stress", "thermo_stress_peak"],
      "thermo.stress_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"],
      "thermo.thermal_strain_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_peak_mechanical_strain", "thermo_mechanical_strain_peak"],
      "thermo.mechanical_strain_peak"
    )
    |> maybe_insert_focus_metric(
      payloads,
      "thermo",
      ["thermo_peak_total_strain", "thermo_total_strain_peak"],
      "thermo.total_strain_peak"
    )
  end

  defp report_focus_metrics(_bundle), do: %{}

  defp report_focus_context(%{"bundle_payloads" => payloads}) when is_map(payloads) do
    [
      {"electrostatic.potential_max", "electrostatic", ["electrostatic_potential_max"], []},
      {"electrostatic.field_peak", "electrostatic",
       ["electrostatic_peak_field", "electrostatic_field_peak_magnitude"],
       [
         "peak_element_id",
         "peak_average_potential",
         "peak_electric_field_x",
         "peak_electric_field_y",
         "peak_flux_density_x",
         "peak_flux_density_y",
         "peak_potential_gradient_magnitude"
       ]},
      {"thermal.temperature_max", "thermal", ["thermal_temperature_max"], []},
      {"thermal.flux_peak", "thermal", ["thermal_peak_flux", "thermal_flux_peak_magnitude"],
       [
         "peak_element_id",
         "peak_average_temperature",
         "peak_heat_flux_x",
         "peak_heat_flux_y",
         "peak_temperature_gradient_x",
         "peak_temperature_gradient_y",
         "peak_temperature_gradient_magnitude"
       ]},
      {"thermo.temperature_delta_max", "thermo", ["thermo_temperature_delta_max"], []},
      {"thermo.displacement_peak", "thermo",
       ["thermo_peak_displacement", "thermo_displacement_peak_magnitude"],
       [
         "peak_node_id",
         "peak_displacement_x",
         "peak_displacement_y",
         "peak_node_temperature_delta"
       ]},
      {"thermo.stress_peak", "thermo", ["thermo_peak_stress", "thermo_stress_peak"],
       [
         "peak_element_id",
         "peak_stress_x",
         "peak_stress_y",
         "peak_tau_xy",
         "peak_principal_stress_1",
         "peak_principal_stress_2",
         "peak_max_in_plane_shear",
         "peak_element_temperature_delta"
       ]},
      {"thermo.thermal_strain_peak", "thermo",
       ["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"],
       [
         "thermo_peak_thermal_strain_id",
         "peak_element_id",
         "peak_thermal_strain",
         "peak_mechanical_strain_x",
         "peak_mechanical_strain_y",
         "peak_total_strain_x",
         "peak_total_strain_y",
         "peak_gamma_xy",
         "peak_element_temperature_delta"
       ]}
    ]
    |> Enum.reduce(%{}, fn {metric_key, source, fields, extras}, acc ->
      case Map.get(payloads, source) do
        %{} = payload ->
          case Enum.find(fields, &Map.has_key?(payload, &1)) do
            nil ->
              acc

            field ->
              Map.put(
                acc,
                metric_key,
                Enum.reduce(extras, %{"source" => source, "value_field" => field}, fn extra,
                                                                                      ctx ->
                  case Map.fetch(payload, extra) do
                    {:ok, value} -> Map.put(ctx, extra, value)
                    :error -> ctx
                  end
                end)
              )
          end

        _ ->
          acc
      end
    end)
  end

  defp report_focus_context(_bundle), do: %{}

  defp report_focus_payloads(bundle) do
    focus_metrics = report_focus_metrics(bundle)
    focus_context = report_focus_context(bundle)

    Enum.reduce(focus_metrics, %{}, fn {metric_id, value}, acc ->
      case Map.get(focus_context, metric_id) do
        %{} = context ->
          Map.put(acc, metric_id, %{
            "focus_contract" => "kyuubiki.workflow_focus_payload/v1",
            "metric_id" => metric_id,
            "source" => Map.get(context, "source"),
            "value" => value,
            "value_field" => Map.get(context, "value_field"),
            "context" => context
          })

        _ ->
          acc
      end
    end)
  end

  defp resolve_focus_payload(
         %{"focus_contract" => "kyuubiki.workflow_focus_payload/v1"} = payload,
         _config
       ),
       do: {:ok, payload}

  defp resolve_focus_payload(payload, %{"metric_id" => metric_id})
       when is_map(payload) and is_binary(metric_id) do
    focus_payloads =
      case Map.get(payload, "report_focus_payloads") do
        %{} = entries -> entries
        _ -> payload
      end

    case Map.fetch(focus_payloads, metric_id) do
      {:ok, %{} = focus_payload} -> {:ok, focus_payload}
      _ -> {:error, :focus_metric_not_found}
    end
  end

  defp resolve_focus_payload(_payload, _config), do: {:error, :focus_metric_not_found}

  defp resolve_focus_chain_input(
         %{"chain_contract" => "kyuubiki.workflow_focus_chain_input/v1"} = payload,
         _config
       ),
       do: {:ok, payload}

  defp resolve_focus_chain_input(payload, config), do: compose_focus_chain_input(payload, config)

  defp resolve_focus_bridge_request(
         %{"request_contract" => "kyuubiki.workflow_focus_bridge_request/v1"} = payload,
         _config
       ),
       do: {:ok, payload}

  defp resolve_focus_bridge_request(payload, config),
    do: compose_focus_bridge_request(payload, config)

  defp resolve_named_focus_bridge_request(%{"request" => %{} = request}, _config),
    do: resolve_focus_bridge_request(request, %{})

  defp resolve_named_focus_bridge_request(%{"bridge_request" => %{} = request}, _config),
    do: resolve_focus_bridge_request(request, %{})

  defp resolve_named_focus_bridge_request(payload, config),
    do: resolve_focus_bridge_request(payload, config)

  defp resolve_named_bridge_payload(%{"bridge_payload" => bridge_payload}, _config),
    do: {:ok, bridge_payload}

  defp resolve_named_bridge_payload(_payload, %{"bridge_payload" => bridge_payload}),
    do: {:ok, bridge_payload}

  defp resolve_named_bridge_payload(_payload, _config), do: :error

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
    |> maybe_push_highlight(
      focus_metrics,
      "electrostatic.potential_max",
      "Electrostatic potential peak",
      triggered_fields,
      ["electrostatic_potential_max"]
    )
    |> maybe_push_highlight(
      focus_metrics,
      "electrostatic.field_peak",
      "Electrostatic field peak",
      triggered_fields,
      ["electrostatic_peak_field", "electrostatic_field_peak_magnitude"]
    )
    |> maybe_push_highlight(
      focus_metrics,
      "thermal.temperature_max",
      "Thermal temperature peak",
      triggered_fields,
      ["thermal_temperature_max"]
    )
    |> maybe_push_highlight(
      focus_metrics,
      "thermo.temperature_delta_max",
      "Thermo temperature delta peak",
      triggered_fields,
      ["thermo_temperature_delta_max"]
    )
    |> maybe_push_highlight(
      focus_metrics,
      "thermo.stress_peak",
      "Thermo stress peak",
      triggered_fields,
      ["thermo_peak_stress", "thermo_stress_peak"]
    )
    |> maybe_push_highlight(
      focus_metrics,
      "thermo.thermal_strain_peak",
      "Thermo thermal strain peak",
      triggered_fields,
      ["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"]
    )
  end

  defp report_highlights(_focus_metrics, _guard), do: []

  defp maybe_push_highlight(
         highlights,
         focus_metrics,
         metric_key,
         label,
         triggered_fields,
         source_fields
       ) do
    case Map.fetch(focus_metrics, metric_key) do
      {:ok, value} ->
        attention = Enum.any?(source_fields, &(&1 in triggered_fields))

        highlights ++
          [%{"id" => metric_key, "label" => label, "value" => value, "attention" => attention}]

      :error ->
        highlights
    end
  end
end
