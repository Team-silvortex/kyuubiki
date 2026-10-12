defmodule KyuubikiWeb.WorkflowQualityReadiness do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowQualityComposition, as: Quality

  @domain_lists ~w(coupled_readiness_blocking_domains coupled_readiness_required_missing coupled_readiness_warning_domains)

  def resolve(sources) do
    Enum.reduce(sources, nil, fn
      {nil, _path}, selected ->
        selected

      {value, path}, selected ->
        object = Check.object(value, path)
        validate(object, path)

        # Aliases carry one report, not competing reports to merge or prioritize.
        if selected != nil && selected !== object,
          do: Check.fail("#{path} conflicts with another coupled_readiness report")

        object
    end)
  end

  def blocked?(nil), do: false

  def blocked?(value) do
    value["coupled_readiness_ready"] == false ||
      value["coupled_readiness_state"] == "block" ||
      value["coupled_readiness_recommendation"] == "hold_and_repair_inputs" ||
      has_domains?(value, "coupled_readiness_blocking_domains") ||
      has_domains?(value, "coupled_readiness_required_missing")
  end

  def review?(nil), do: false

  def review?(value) do
    value["coupled_readiness_state"] == "warn" ||
      value["coupled_readiness_recommendation"] == "review_before_next_round" ||
      has_domains?(value, "coupled_readiness_warning_domains")
  end

  defp has_domains?(value, field), do: Map.get(value, field, []) != []

  def iteration_hint(value) do
    blocked = blocked?(value)
    blocking = Map.get(value, "coupled_readiness_blocking_domains", [])
    missing = Map.get(value, "coupled_readiness_required_missing", [])
    warnings = Map.get(value, "coupled_readiness_warning_domains", [])

    %{
      "action" => if(blocked, do: "fix_coupled_readiness", else: "review_coupled_readiness"),
      "focus_domain" => List.first(blocking ++ missing ++ warnings),
      "focus_source" => "coupled_readiness",
      "focus_field" => nil,
      "blocking_count" => length(blocking) + length(missing),
      "warning_count" => length(warnings),
      "blocking_domains" => blocking,
      "required_missing" => missing,
      "warning_domains" => warnings,
      "recommendation" =>
        if(blocked, do: "hold_and_repair_inputs", else: "review_before_next_round"),
      "readiness_state" => value["coupled_readiness_state"]
    }
  end

  defp validate(object, path) do
    Check.converged(object, path)

    Quality.boolean(
      Map.get(object, "coupled_readiness_ready", true),
      "#{path}.coupled_readiness_ready"
    )

    Check.choice(object, "coupled_readiness_state", "ready", ~w(ready pass warn block), path)

    Check.choice(
      object,
      "coupled_readiness_recommendation",
      "continue_to_next_round",
      ~w(continue_to_next_round review_before_next_round hold_and_repair_inputs),
      path
    )

    for field <- @domain_lists do
      domains = Check.list(Map.get(object, field, []), "#{path}.#{field}")

      for {domain, index} <- Enum.with_index(domains) do
        if !is_binary(domain) || String.trim(domain) == "",
          do: Check.fail("#{path}.#{field}[#{index}] must be nonblank text")
      end
    end
  end
end
