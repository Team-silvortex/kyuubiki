defmodule KyuubikiWeb.WorkflowMaterialCardRuntime do
  @moduledoc false

  @schema_version "kyuubiki.material-card/v1"
  @valid_unit_systems ["si", "engineering", "mixed"]
  @valid_confidence_levels ["unknown", "screening", "datasheet", "measured", "certified"]

  def validate_material_card(payload, config) when is_map(payload) and is_map(config) do
    card = Map.get(payload, "material_card", payload)

    if is_map(card) do
      issues =
        []
        |> require_string(card, "schema_version")
        |> require_string(card, "material_id")
        |> require_string(card, "display_name")
        |> validate_schema_version(card)
        |> validate_unit_system(card)
        |> validate_provenance(card)
        |> validate_confidence(card)
        |> validate_parameters(card)
        |> validate_required_parameters(card, config)
        |> validate_expected_units(card, config)
        |> validate_temperature_scope(card, config)

      report(card, issues)
    else
      {:error, :invalid_material_card_payload}
    end
  end

  def validate_material_card(_payload, _config), do: {:error, :invalid_material_card_payload}

  defp report(card, issues) do
    errors = Enum.filter(issues, &(Map.get(&1, "severity") == "error"))
    warnings = Enum.filter(issues, &(Map.get(&1, "severity") == "warning"))
    status = if errors == [], do: if(warnings == [], do: "pass", else: "warn"), else: "fail"

    {:ok,
     %{
       "material_card_preflight_status" => status,
       "material_card_id" => Map.get(card, "material_id", ""),
       "material_card_display_name" => Map.get(card, "display_name", ""),
       "material_card_unit_system" => Map.get(card, "unit_system", "unknown"),
       "material_card_confidence_level" => get_in(card, ["confidence", "level"]) || "unknown",
       "material_card_parameter_count" => card |> Map.get("parameters", %{}) |> map_size_safe(),
       "material_card_issue_count" => length(issues),
       "material_card_error_count" => length(errors),
       "material_card_warning_count" => length(warnings),
       "material_card_quality_gates" => quality_gates(status, issues),
       "material_card_issues" => issues,
       "material_card_reliability_envelope" => reliability_envelope(card, status, issues)
     }}
  end

  defp require_string(issues, card, key) do
    case Map.get(card, key) do
      value when is_binary(value) and value != "" ->
        issues

      _ ->
        [
          issue(
            "error",
            "missing_required_field",
            key,
            "Required material-card field is missing."
          )
          | issues
        ]
    end
  end

  defp validate_schema_version(issues, %{"schema_version" => @schema_version}), do: issues

  defp validate_schema_version(issues, %{"schema_version" => _}) do
    [
      issue(
        "error",
        "unsupported_schema_version",
        "schema_version",
        "Expected kyuubiki.material-card/v1."
      )
      | issues
    ]
  end

  defp validate_schema_version(issues, _card), do: issues

  defp validate_unit_system(issues, %{"unit_system" => unit_system})
       when unit_system in @valid_unit_systems,
       do: issues

  defp validate_unit_system(issues, %{"unit_system" => _}) do
    [
      issue(
        "error",
        "unsupported_unit_system",
        "unit_system",
        "Unit system must be si, engineering, or mixed."
      )
      | issues
    ]
  end

  defp validate_unit_system(issues, _card), do: issues

  defp validate_provenance(issues, %{"provenance" => %{} = provenance}) do
    issues
    |> require_nested_string(provenance, "provenance", "source_id")
    |> require_nested_string(provenance, "provenance", "source_label")
  end

  defp validate_provenance(issues, _card) do
    [
      issue(
        "error",
        "missing_provenance",
        "provenance",
        "Material source provenance is required."
      )
      | issues
    ]
  end

  defp validate_confidence(issues, %{"confidence" => %{"level" => level}})
       when level in @valid_confidence_levels,
       do: issues

  defp validate_confidence(issues, %{"confidence" => %{"level" => _}}) do
    [
      issue(
        "error",
        "unsupported_confidence_level",
        "confidence.level",
        "Confidence level is not recognized."
      )
      | issues
    ]
  end

  defp validate_confidence(issues, _card) do
    [
      issue(
        "error",
        "missing_confidence",
        "confidence.level",
        "Material confidence level is required."
      )
      | issues
    ]
  end

  defp validate_parameters(issues, %{"parameters" => parameters})
       when is_map(parameters) and map_size(parameters) > 0 do
    Enum.reduce(parameters, issues, fn {name, parameter}, acc ->
      validate_parameter(acc, name, parameter)
    end)
  end

  defp validate_parameters(issues, _card) do
    [
      issue(
        "error",
        "missing_parameters",
        "parameters",
        "At least one material parameter is required."
      )
      | issues
    ]
  end

  defp validate_parameter(issues, name, %{"kind" => kind, "unit" => unit} = parameter)
       when kind in ["scalar", "tensor_diagonal", "table"] and is_binary(unit) and unit != "" do
    if parameter_value_present?(parameter) do
      issues
    else
      [
        issue(
          "error",
          "missing_parameter_value",
          "parameters.#{name}.value",
          "Parameter value is required."
        )
        | issues
      ]
    end
  end

  defp validate_parameter(issues, name, _parameter) do
    [
      issue(
        "error",
        "invalid_parameter",
        "parameters.#{name}",
        "Parameter needs kind, unit, and value."
      )
      | issues
    ]
  end

  defp validate_required_parameters(issues, card, %{"required_parameters" => required})
       when is_list(required) do
    parameters = Map.get(card, "parameters", %{})

    required
    |> Enum.filter(&is_binary/1)
    |> Enum.reject(&Map.has_key?(parameters, &1))
    |> Enum.reduce(issues, fn name, acc ->
      [
        issue(
          "error",
          "missing_required_parameter",
          "parameters.#{name}",
          "Required material parameter is missing."
        )
        | acc
      ]
    end)
  end

  defp validate_required_parameters(issues, _card, _config), do: issues

  defp validate_expected_units(issues, card, %{"expected_units" => expected_units})
       when is_map(expected_units) do
    parameters = Map.get(card, "parameters", %{})

    Enum.reduce(expected_units, issues, fn {name, expected_unit}, acc ->
      actual_unit = get_in(parameters, [name, "unit"])

      if is_binary(expected_unit) and is_binary(actual_unit) and actual_unit != expected_unit do
        [
          issue(
            "error",
            "unit_mismatch",
            "parameters.#{name}.unit",
            "Expected #{expected_unit}, got #{actual_unit}."
          )
          | acc
        ]
      else
        acc
      end
    end)
  end

  defp validate_expected_units(issues, _card, _config), do: issues

  defp validate_temperature_scope(issues, card, %{"required_temperature_c" => temperature})
       when is_number(temperature) do
    case Map.get(card, "applicability", %{}) |> Map.get("temperature_range_c") do
      [min_c, max_c]
      when is_number(min_c) and is_number(max_c) and temperature >= min_c and temperature <= max_c ->
        issues

      [min_c, max_c] when is_number(min_c) and is_number(max_c) ->
        [
          issue(
            "warning",
            "temperature_out_of_scope",
            "applicability.temperature_range_c",
            "Required temperature is outside card applicability."
          )
          | issues
        ]

      _ ->
        [
          issue(
            "warning",
            "missing_temperature_scope",
            "applicability.temperature_range_c",
            "Temperature applicability is not declared."
          )
          | issues
        ]
    end
  end

  defp validate_temperature_scope(issues, _card, _config), do: issues

  defp require_nested_string(issues, object, prefix, key) do
    case Map.get(object, key) do
      value when is_binary(value) and value != "" ->
        issues

      _ ->
        [
          issue(
            "error",
            "missing_required_field",
            "#{prefix}.#{key}",
            "Required nested field is missing."
          )
          | issues
        ]
    end
  end

  defp parameter_value_present?(%{"value" => value}), do: not is_nil(value)
  defp parameter_value_present?(_parameter), do: false

  defp quality_gates(status, issues) do
    %{
      "status" => status,
      "schema_valid" =>
        not Enum.any?(
          issues,
          &(&1["code"] in ["missing_required_field", "unsupported_schema_version"])
        ),
      "units_valid" =>
        not Enum.any?(issues, &(&1["code"] in ["unsupported_unit_system", "unit_mismatch"])),
      "provenance_valid" => not Enum.any?(issues, &(&1["code"] == "missing_provenance")),
      "parameters_valid" => not Enum.any?(issues, &String.starts_with?(&1["field"], "parameters"))
    }
  end

  defp reliability_envelope(card, status, issues) do
    %{
      "schema" => "kyuubiki.material-card-reliability/v1",
      "trust_level" => trust_level(card, status),
      "material_card_id" => Map.get(card, "material_id", ""),
      "source_id" => get_in(card, ["provenance", "source_id"]) || "",
      "confidence_level" => get_in(card, ["confidence", "level"]) || "unknown",
      "limitations" => limitations(status, issues)
    }
  end

  defp trust_level(_card, "fail"), do: "blocked"

  defp trust_level(%{"confidence" => %{"level" => level}}, _status)
       when level in ["measured", "certified"], do: "review_ready"

  defp trust_level(_card, _status), do: "screening_only"

  defp limitations("pass", _issues), do: ["material_card_preflight_only"]
  defp limitations(_status, issues), do: issues |> Enum.map(& &1["code"]) |> Enum.uniq()

  defp issue(severity, code, field, message) do
    %{"severity" => severity, "code" => code, "field" => field, "message" => message}
  end

  defp map_size_safe(value) when is_map(value), do: map_size(value)
  defp map_size_safe(_value), do: 0
end
