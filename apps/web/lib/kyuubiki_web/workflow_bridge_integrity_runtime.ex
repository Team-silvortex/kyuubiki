defmodule KyuubikiWeb.WorkflowBridgeIntegrityRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowOperatorBridgeRuntime
  alias KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime

  def validate_electrostatic_heat_bridge(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, source_result} <-
           fetch_payload_map(payload, Map.get(config, "source_key", "electrostatic_result")),
         {:ok, target_model} <-
           fetch_payload_map(payload, Map.get(config, "target_key", "heat_model")),
         {:ok, contract} <-
           WorkflowOperatorBridgeRuntime.resolve_electrostatic_to_heat_bridge_contract(config),
         {:ok, expected_model} <-
           rebuild_expected_electrostatic_heat_model(source_result, target_model, contract),
         {:ok, report} <-
           compare_bridged_models(
             expected_model,
             target_model,
             contract,
             "electrostatic_to_heat",
             config
           ) do
      {:ok, Map.put(target_model, "bridge_integrity", report)}
    end
  end

  def validate_electrostatic_heat_bridge(_payload, _config),
    do: {:error, :invalid_bridge_integrity_payload}

  def validate_heat_thermo_bridge(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, source_result} <-
           fetch_payload_map(payload, Map.get(config, "source_key", "heat_result")),
         {:ok, target_model} <-
           fetch_payload_map(payload, Map.get(config, "target_key", "thermo_model")),
         {:ok, contract} <-
           WorkflowOperatorHeatBridgeRuntime.resolve_heat_to_thermo_bridge_contract(config),
         {:ok, expected_model} <-
           rebuild_expected_heat_thermo_model(source_result, target_model, contract),
         {:ok, report} <-
           compare_bridged_models(
             expected_model,
             target_model,
             contract,
             "heat_to_thermo",
             config
           ) do
      {:ok, Map.put(target_model, "bridge_integrity", report)}
    end
  end

  def validate_heat_thermo_bridge(_payload, _config),
    do: {:error, :invalid_bridge_integrity_payload}

  def extract_bridge_integrity_diagnostics(payload, config)
      when is_map(payload) and is_map(config) do
    with {:ok, integrity} <- resolve_bridge_integrity(payload, config) do
      prefix = normalize_prefix(Map.get(config, "output_prefix", "bridge"))

      {:ok,
       %{
         "diagnostic_contract" => "kyuubiki.workflow_diagnostics/v1",
         "diagnostic_domain" => "workflow_bridge",
         "diagnostic_subject" => "bridge_integrity",
         "diagnostic_prefix" => prefix,
         "diagnostic_node_count" => Map.get(integrity, "checked_node_count", 0),
         "diagnostic_element_count" => Map.get(integrity, "checked_element_count", 0),
         "diagnostic_metric_groups" => ["bridge", "integrity"],
         "#{prefix}_integrity_passed" =>
           if(Map.get(integrity, "status") == "pass", do: 1.0, else: 0.0),
         "#{prefix}_checked_node_count" =>
           numeric_or_zero(Map.get(integrity, "checked_node_count")),
         "#{prefix}_checked_element_count" =>
           numeric_or_zero(Map.get(integrity, "checked_element_count")),
         "#{prefix}_value_mismatch_count" =>
           numeric_or_zero(Map.get(integrity, "value_mismatch_count")),
         "#{prefix}_missing_value_count" =>
           numeric_or_zero(Map.get(integrity, "missing_value_count")),
         "#{prefix}_coordinate_mismatch_count" =>
           numeric_or_zero(Map.get(integrity, "coordinate_mismatch_count")),
         "#{prefix}_max_abs_error" => numeric_or_zero(Map.get(integrity, "max_abs_error")),
         "#{prefix}_tolerance" => numeric_or_zero(Map.get(integrity, "tolerance")),
         "#{prefix}_node_count_mismatch" =>
           boolean_number(Map.get(integrity, "node_count_mismatch")),
         "#{prefix}_element_count_mismatch" =>
           boolean_number(Map.get(integrity, "element_count_mismatch")),
         "#{prefix}_bridge_kind" => Map.get(integrity, "bridge_kind"),
         "#{prefix}_source_field" => Map.get(integrity, "source_field"),
         "#{prefix}_target_field" => Map.get(integrity, "target_field"),
         "#{prefix}_distribution" => Map.get(integrity, "distribution")
       }}
    end
  end

  def extract_bridge_integrity_diagnostics(_payload, _config),
    do: {:error, :invalid_bridge_integrity_diagnostics_payload}

  defp fetch_payload_map(payload, key) when is_map(payload) and is_binary(key) do
    case Map.get(payload, key) do
      %{} = value -> {:ok, value}
      _ -> {:error, {:missing_bridge_integrity_input, key}}
    end
  end

  defp fetch_payload_map(_payload, key), do: {:error, {:missing_bridge_integrity_input, key}}

  defp resolve_bridge_integrity(%{"bridge_integrity" => %{} = integrity}, _config),
    do: {:ok, integrity}

  defp resolve_bridge_integrity(payload, config) do
    source_key = Map.get(config, "source_key", "model")

    case Map.get(payload, source_key) do
      %{"bridge_integrity" => %{} = integrity} -> {:ok, integrity}
      %{} = nested -> resolve_bridge_integrity(nested, %{})
      _ -> {:error, :missing_bridge_integrity_report}
    end
  end

  defp rebuild_expected_electrostatic_heat_model(source_result, target_model, contract) do
    case detect_shape(target_model) do
      :quad ->
        WorkflowOperatorBridgeRuntime.bridge_electrostatic_result_to_heat_plane_quad_model(
          source_result,
          target_model,
          contract
        )

      :triangle ->
        WorkflowOperatorBridgeRuntime.bridge_electrostatic_result_to_heat_plane_triangle_model(
          source_result,
          target_model,
          contract
        )
    end
  end

  defp rebuild_expected_heat_thermo_model(source_result, target_model, contract) do
    normalized_source_result = normalize_heat_source_result(source_result, contract)

    case detect_shape(target_model) do
      :quad ->
        WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_quad_model(
          normalized_source_result,
          target_model,
          contract
        )

      :triangle ->
        WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_triangle_model(
          normalized_source_result,
          target_model,
          contract
        )
    end
  end

  defp compare_bridged_models(expected_model, target_model, contract, bridge_kind, config) do
    expected_nodes = Map.get(expected_model, "nodes", [])
    actual_nodes = Map.get(target_model, "nodes", [])
    expected_elements = Map.get(expected_model, "elements", [])
    actual_elements = Map.get(target_model, "elements", [])
    tolerance = normalize_tolerance(Map.get(config, "tolerance", 1.0e-6))
    target_field = Map.fetch!(contract, :target_field)

    {value_mismatch_count, missing_value_count, coordinate_mismatch_count, max_abs_error} =
      Enum.zip(expected_nodes, actual_nodes)
      |> Enum.reduce({0, 0, 0, 0.0}, fn {expected_node, actual_node}, acc ->
        compare_node_pair(expected_node, actual_node, target_field, tolerance, acc)
      end)

    report = %{
      "status" => "pass",
      "bridge_kind" => bridge_kind,
      "distribution" => Map.get(contract, :distribution),
      "source_field" => Map.get(contract, :source_field),
      "target_field" => target_field,
      "checked_node_count" => length(actual_nodes),
      "checked_element_count" => length(actual_elements),
      "value_mismatch_count" => value_mismatch_count,
      "missing_value_count" => missing_value_count,
      "coordinate_mismatch_count" => coordinate_mismatch_count,
      "element_count_mismatch" => length(expected_elements) != length(actual_elements),
      "node_count_mismatch" => length(expected_nodes) != length(actual_nodes),
      "max_abs_error" => max_abs_error,
      "tolerance" => tolerance
    }

    if report["value_mismatch_count"] == 0 and report["missing_value_count"] == 0 and
         report["coordinate_mismatch_count"] == 0 and report["element_count_mismatch"] == false and
         report["node_count_mismatch"] == false do
      {:ok, report}
    else
      {:error, {:bridge_integrity_failed, report}}
    end
  end

  defp compare_node_pair(expected_node, actual_node, target_field, tolerance, acc) do
    {value_mismatch_count, missing_value_count, coordinate_mismatch_count, max_abs_error} = acc

    coordinate_mismatch_count =
      if node_coordinates_match?(expected_node, actual_node, tolerance),
        do: coordinate_mismatch_count,
        else: coordinate_mismatch_count + 1

    case {fetch_numeric(actual_node, target_field), fetch_numeric(expected_node, target_field)} do
      {{:ok, actual_value}, {:ok, expected_value}} ->
        abs_error = abs(actual_value - expected_value)

        if abs_error <= tolerance do
          {value_mismatch_count, missing_value_count, coordinate_mismatch_count,
           max(max_abs_error, abs_error)}
        else
          {value_mismatch_count + 1, missing_value_count, coordinate_mismatch_count,
           max(max_abs_error, abs_error)}
        end

      _ ->
        {value_mismatch_count, missing_value_count + 1, coordinate_mismatch_count, max_abs_error}
    end
  end

  defp node_coordinates_match?(expected_node, actual_node, tolerance) do
    coordinate_axes(expected_node, actual_node)
    |> Enum.all?(fn axis ->
      close_enough?(Map.get(expected_node, axis), Map.get(actual_node, axis), tolerance)
    end)
  end

  defp coordinate_axes(expected_node, actual_node) do
    ["x", "y", "z"]
    |> Enum.filter(fn axis ->
      Map.has_key?(expected_node, axis) or Map.has_key?(actual_node, axis)
    end)
  end

  defp close_enough?(left, right, tolerance) when is_number(left) and is_number(right),
    do: abs(left * 1.0 - right * 1.0) <= tolerance

  defp close_enough?(nil, nil, _tolerance), do: true
  defp close_enough?(_left, _right, _tolerance), do: false

  defp fetch_numeric(node, field) when is_map(node) and is_binary(field) do
    case Map.get(node, field) do
      value when is_number(value) -> {:ok, value * 1.0}
      _ -> :error
    end
  end

  defp fetch_numeric(_node, _field), do: :error

  defp normalize_tolerance(value) when is_number(value) and value > 0, do: value * 1.0
  defp normalize_tolerance(_value), do: 1.0e-6

  defp numeric_or_zero(value) when is_number(value), do: value * 1.0
  defp numeric_or_zero(_value), do: 0.0

  defp boolean_number(true), do: 1.0
  defp boolean_number(_value), do: 0.0

  defp normalize_prefix(prefix) when is_binary(prefix) and prefix != "" do
    prefix
    |> String.trim()
    |> String.replace(~r/[^a-zA-Z0-9]+/u, "_")
    |> String.trim("_")
    |> case do
      "" -> "bridge"
      normalized -> normalized
    end
  end

  defp normalize_prefix(_prefix), do: "bridge"

  defp normalize_heat_source_result(%{} = source_result, %{distribution: "node_to_node"}) do
    case {Map.get(source_result, "elements"), get_in(source_result, ["input", "elements"])} do
      {elements, nil} when is_list(elements) ->
        Map.put(source_result, "input", %{"elements" => elements})

      _ ->
        source_result
    end
  end

  defp normalize_heat_source_result(source_result, _contract), do: source_result

  defp detect_shape(%{"elements" => [first | _]}) when is_map(first) do
    if Map.has_key?(first, "node_l"), do: :quad, else: :triangle
  end

  defp detect_shape(_target_model), do: :quad
end
