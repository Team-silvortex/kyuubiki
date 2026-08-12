defmodule KyuubikiWeb.Orchestra.OperatorTaskAdmission do
  @moduledoc """
  Applies the language-neutral TaskIR authority and routing admission policy.
  """

  @schema_version "kyuubiki.operator-task-admission/v1"
  @central_authorities ["central_operator_library", "single_orchestrator"]
  @local_authorities ["agent_local", "offline_mesh", "self_directed"]
  @orchestra_fetch "orchestra_fetch"
  @local_execution_modes ["agent_native", "local_builtin", "local_bundle"]
  @cache_scopes ["job", "session", "agent", "none"]
  @max_routing_values 64
  @max_routing_value_bytes 128

  @spec check(map(), map()) :: {:ok, map()} | {:error, term()}
  def check(task, summary) when is_map(task) and is_map(summary) do
    violations =
      []
      |> validate_operator_identity(summary)
      |> validate_authority_and_execution(summary)
      |> validate_cache_scope(summary)
      |> validate_package_authority(summary)
      |> validate_string_list(
        get_in(task, ["runtime_hints", "required_capabilities"]),
        "runtime_hints.required_capabilities"
      )
      |> validate_string_list(
        get_in(task, ["runtime_hints", "placement_tags"]),
        "runtime_hints.placement_tags"
      )
      |> Enum.reverse()

    report = %{
      "schema_version" => @schema_version,
      "accepted" => violations == [],
      "task_id" => summary["task_id"],
      "operator_id" => summary["operator_id"],
      "authority_mode" => summary["authority_mode"],
      "execution_mode" => summary["execution_mode"],
      "cache_scope" => summary["cache_scope"],
      "agent_fetchable" => summary["agent_fetchable"],
      "package_ref" => summary["package_ref"],
      "violations" => violations
    }

    if report["accepted"] do
      {:ok, report}
    else
      {:error, {:operator_task_admission_rejected, report}}
    end
  end

  def check(_task, _summary), do: {:error, :invalid_operator_task_admission_request}

  defp validate_operator_identity(violations, summary) do
    id = summary["operator_id"]

    maybe_violation(
      violations,
      not (is_binary(id) and Regex.match?(~r/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/, id)),
      "operator_id_unsafe_for_resolution",
      "operator.id",
      "operator id must use 1-128 ASCII alphanumeric, dot, underscore, or hyphen bytes"
    )
  end

  defp validate_authority_and_execution(violations, summary) do
    authority = summary["authority_mode"]
    execution = summary["execution_mode"]

    violations
    |> require_known_value(
      authority,
      central_authority?(authority) or authority in @local_authorities,
      "runtime_hints.authority_mode",
      "authority_mode_missing",
      "authority_mode_unsupported"
    )
    |> require_known_value(
      execution,
      execution == @orchestra_fetch or execution in @local_execution_modes,
      "runtime_hints.execution_mode",
      "execution_mode_missing",
      "execution_mode_unsupported"
    )
    |> validate_agent_fetchable(authority, summary["agent_fetchable"])
    |> maybe_violation(
      central_authority?(authority) and execution != @orchestra_fetch,
      "central_authority_requires_orchestra_fetch",
      "runtime_hints.execution_mode",
      "central_operator_library tasks must use orchestra_fetch"
    )
    |> maybe_violation(
      execution == @orchestra_fetch and not central_authority?(authority),
      "orchestra_fetch_requires_central_authority",
      "runtime_hints.authority_mode",
      "orchestra_fetch requires central_operator_library authority"
    )
  end

  defp validate_agent_fetchable(violations, _authority, nil) do
    violation(
      violations,
      "agent_fetchable_missing",
      "runtime_hints.agent_fetchable",
      "agent_fetchable must be declared explicitly"
    )
  end

  defp validate_agent_fetchable(violations, authority, fetchable) do
    cond do
      fetchable == true and not central_authority?(authority) ->
        violation(
          violations,
          "agent_fetch_requires_central_authority",
          "runtime_hints.agent_fetchable",
          "agent package fetch requires central authority"
        )

      fetchable == false and central_authority?(authority) ->
        violation(
          violations,
          "central_authority_requires_agent_fetch",
          "runtime_hints.agent_fetchable",
          "central authority tasks must remain agent fetchable"
        )

      true ->
        violations
    end
  end

  defp validate_cache_scope(violations, summary) do
    case summary["cache_scope"] do
      nil ->
        violation(
          violations,
          "cache_scope_missing",
          "runtime_hints.cache_scope",
          "cache_scope must be declared explicitly"
        )

      value when value not in @cache_scopes ->
        violation(
          violations,
          "cache_scope_unsupported",
          "runtime_hints.cache_scope",
          "cache_scope `#{value}` is not supported"
        )

      _value ->
        violations
    end
  end

  defp validate_package_authority(violations, summary) do
    authority = summary["authority_mode"]
    execution = summary["execution_mode"]
    package_ref = summary["package_ref"]
    expected = "orchestra://operator-package/#{summary["operator_id"]}"

    violations
    |> maybe_violation(
      central_authority?(authority) and is_nil(package_ref),
      "central_package_ref_missing",
      "execution_program.package_ref",
      "central operator tasks must declare an orchestra package reference"
    )
    |> maybe_violation(
      central_authority?(authority) and not is_nil(package_ref) and package_ref != expected,
      "central_package_ref_mismatch",
      "execution_program.package_ref",
      "central package reference must be `#{expected}`, got `#{package_ref}`"
    )
    |> maybe_violation(
      not central_authority?(authority) and orchestra_package?(package_ref),
      "local_authority_forbids_orchestra_package",
      "execution_program.package_ref",
      "local and offline authority cannot resolve orchestra package references"
    )
    |> maybe_violation(
      execution == "local_bundle" and not safe_bundle_package?(package_ref),
      "local_bundle_package_ref_invalid",
      "execution_program.package_ref",
      "local_bundle execution requires a bundle:// package reference"
    )
  end

  defp validate_string_list(violations, nil, _field), do: violations

  defp validate_string_list(violations, values, field) when is_list(values) do
    violations =
      maybe_violation(
        violations,
        length(values) > @max_routing_values,
        "routing_values_over_budget",
        field,
        "#{field} exceeds #{@max_routing_values} entries"
      )

    {violations, _seen} =
      Enum.reduce(values, {violations, MapSet.new()}, fn value, {issues, seen} ->
        issues = validate_routing_value(issues, value, field, seen)
        seen = if is_binary(value), do: MapSet.put(seen, value), else: seen
        {issues, seen}
      end)

    violations
  end

  defp validate_string_list(violations, _values, field) do
    violation(
      violations,
      "routing_values_not_array",
      field,
      "#{field} must be an array of strings"
    )
  end

  defp validate_routing_value(violations, value, field, _seen) when not is_binary(value) do
    violation(
      violations,
      "routing_value_not_string",
      field,
      "#{field} entries must be strings"
    )
  end

  defp validate_routing_value(violations, value, field, seen) do
    violations
    |> maybe_violation(
      byte_size(value) == 0 or byte_size(value) > @max_routing_value_bytes,
      "routing_value_invalid_length",
      field,
      "#{field} entries must contain 1-#{@max_routing_value_bytes} bytes"
    )
    |> maybe_violation(
      MapSet.member?(seen, value),
      "routing_value_duplicate",
      field,
      "#{field} contains duplicate value `#{value}`"
    )
  end

  defp require_known_value(violations, nil, _known, field, missing_code, _unsupported_code) do
    violation(violations, missing_code, field, "#{field} must be declared explicitly")
  end

  defp require_known_value(violations, value, false, field, _missing_code, unsupported_code) do
    violation(violations, unsupported_code, field, "#{field} value `#{value}` is not supported")
  end

  defp require_known_value(violations, _value, true, _field, _missing, _unsupported),
    do: violations

  defp maybe_violation(violations, true, code, field, message),
    do: violation(violations, code, field, message)

  defp maybe_violation(violations, false, _code, _field, _message), do: violations

  defp violation(violations, code, field, message) do
    [%{"code" => code, "field" => field, "message" => message} | violations]
  end

  defp orchestra_package?(value) when is_binary(value),
    do: String.starts_with?(value, "orchestra://")

  defp orchestra_package?(_value), do: false

  defp safe_bundle_package?("bundle://" <> path) do
    path != "" and not String.contains?(path, "\\") and
      Enum.all?(String.split(path, "/"), fn segment ->
        segment != "" and segment not in [".", ".."] and
          Regex.match?(~r/^[A-Za-z0-9._-]+$/, segment)
      end)
  end

  defp safe_bundle_package?(_value), do: false
  defp central_authority?(value), do: value in @central_authorities
end
