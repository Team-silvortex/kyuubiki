defmodule KyuubikiWeb.WorkflowDiagnosticBundleRuntime do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  @contract "kyuubiki.workflow_diagnostics/v1"

  def compose(payload, config) do
    Check.run("transform.compose_diagnostics_bundle", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      plain = Check.boolean(config, "include_non_diagnostics", false)
      retain = Check.boolean(config, "include_payloads", true)
      index = Check.boolean(config, "include_numeric_fields", true)
      Check.converged(payload, "payload")

      sources =
        payload
        |> Enum.sort_by(&elem(&1, 0))
        |> Enum.flat_map(fn {id, value} ->
          if id in ["converged", "stability_result"], do: [], else: source(id, value, plain)
        end)

      if sources == [], do: Check.fail("did not find any diagnostics payloads")
      items = Enum.map(sources, &elem(&1, 2))

      numeric_fields =
        sources
        |> Enum.flat_map(fn {_, source, _} ->
          source |> Enum.filter(fn {_, value} -> is_number(value) end) |> Enum.map(&elem(&1, 0))
        end)
        |> Enum.uniq()
        |> Enum.sort()

      domains = unique(items, "domain")

      bundle = %{
        "bundle_contract" => "kyuubiki.workflow_diagnostics_bundle/v1",
        "bundle_kind" => "workflow_diagnostics_bundle",
        "bundle_source_count" => length(items),
        "bundle_sources" => Enum.map(items, & &1["source"]),
        "bundle_domains" => domains,
        "bundle_subjects" => unique(items, "subject"),
        "bundle_domain_counts" =>
          items |> Enum.map(& &1["domain"]) |> Enum.reject(&is_nil/1) |> Enum.frequencies(),
        "bundle_metric_groups" =>
          items |> Enum.flat_map(& &1["metric_groups"]) |> Enum.uniq() |> Enum.sort(),
        "bundle_items" => items,
        "bundle_total_node_count" => total(items, "node"),
        "bundle_total_element_count" => total(items, "element"),
        "bundle_numeric_field_count" => length(numeric_fields)
      }

      bundle =
        if retain,
          do:
            Map.put(
              bundle,
              "bundle_payloads",
              Map.new(sources, fn {id, source, _} -> {id, source} end)
            ),
          else: bundle

      if index, do: Map.put(bundle, "bundle_numeric_fields", numeric_fields), else: bundle
    end)
  end

  def guard(payload, config) do
    Check.run("transform.evaluate_diagnostics_bundle_guard", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      rules = config |> Map.get("rules") |> Check.list("config.rules")
      if rules == [], do: Check.fail("config.rules requires at least one rule")
      Check.converged(payload, "payload")

      if Map.has_key?(payload, "bundle_payloads") do
        payload["bundle_payloads"]
        |> Check.object("payload.bundle_payloads")
        |> Enum.each(fn {source, entry} ->
          path = "payload.bundle_payloads.#{source}"
          entry |> Check.object(path) |> Check.converged(path)
        end)
      end

      triggers =
        rules
        |> Enum.with_index()
        |> Enum.map(fn {rule, index} -> guard_rule(payload, rule, index) end)
        |> Enum.reject(&is_nil/1)

      blocks = Enum.count(triggers, &(&1["severity"] == "block"))
      warnings = Enum.count(triggers, &(&1["severity"] == "warn"))

      status =
        cond do
          blocks > 0 -> "block"
          warnings > 0 -> "warn"
          true -> "pass"
        end

      %{
        "guard_contract" => "kyuubiki.workflow_guard_result/v1",
        "guard_scope" => "workflow_diagnostics_bundle",
        "guard_status" => status,
        "guard_passed" => status == "pass",
        "guard_trigger_count" => length(triggers),
        "guard_checked_rule_count" => length(rules),
        "guard_warn_count" => warnings,
        "guard_block_count" => blocks,
        "guard_triggers" => triggers,
        "guard_summary" => summary(status, triggers),
        "guard_recommendation" =>
          %{
            "block" => "hold_and_review",
            "warn" => "review_before_continue",
            "pass" => "continue"
          }[status]
      }
    end)
  end

  defp source(id, value, plain) when not is_map(value) do
    if plain, do: Check.fail("payload.#{id} must be an object diagnostic source"), else: []
  end

  defp source(id, value, plain) do
    path = "payload.#{id}"
    value = Check.object(value, path)

    selected =
      case Map.fetch(value, "diagnostic_contract") do
        {:ok, @contract} ->
          true

        {:ok, _} ->
          Check.fail("#{path}.diagnostic_contract must be #{@contract}")

        :error ->
          if Enum.any?(Map.keys(value), &String.starts_with?(&1, "diagnostic_")),
            do:
              Check.fail(
                "#{path}.diagnostic_contract is required for a declared diagnostic source"
              )

          plain
      end

    if selected do
      Check.converged(value, path)
      prefix = Check.text(value, "diagnostic_prefix", path)
      Check.text(value, "diagnostic_domain", path)
      Check.text(value, "diagnostic_subject", path)

      groups =
        value
        |> Map.get("diagnostic_metric_groups", [])
        |> Check.list("#{path}.diagnostic_metric_groups")

      groups
      |> Enum.with_index()
      |> Enum.each(fn {group, index} ->
        if not is_binary(group) or String.trim(group) == "",
          do: Check.fail("#{path}.diagnostic_metric_groups[#{index}] must be a nonblank string")
      end)

      node_count = count(value, prefix, "node", path)
      element_count = count(value, prefix, "element", path)

      measured =
        Enum.reduce(value, false, fn {key, number}, measured ->
          if is_number(number) do
            Check.number(number, "#{path}.#{key}")

            measured or
              (not String.starts_with?(key, "diagnostic_") and
                 (is_nil(prefix) or key not in ["#{prefix}_node_count", "#{prefix}_element_count"]))
          else
            measured
          end
        end)

      if not measured, do: Check.fail("#{path} has no numeric diagnostic measurements")

      [
        {id, value,
         %{
           "source" => id,
           "domain" => value["diagnostic_domain"],
           "subject" => value["diagnostic_subject"],
           "prefix" => value["diagnostic_prefix"],
           "node_count" => node_count,
           "element_count" => element_count,
           "metric_groups" => groups
         }}
      ]
    else
      []
    end
  end

  defp count(source, prefix, label, path) do
    canonical = "diagnostic_#{label}_count"
    fallback = if prefix, do: "#{prefix}_#{label}_count"

    field =
      cond do
        Map.has_key?(source, canonical) -> canonical
        fallback && Map.has_key?(source, fallback) -> fallback
        true -> nil
      end

    if field, do: Check.unsigned(source[field], "#{path}.#{field}")
  end

  defp total(items, label) do
    {total, complete} =
      Enum.reduce(items, {0, true}, fn item, {sum, complete} ->
        case item["#{label}_count"] do
          nil -> {sum, false}
          value -> {Check.unsigned(sum + value, "bundle_total_#{label}_count"), complete}
        end
      end)

    if complete, do: total
  end

  defp unique(items, key),
    do: items |> Enum.map(& &1[key]) |> Enum.reject(&is_nil/1) |> Enum.uniq() |> Enum.sort()

  defp guard_rule(payload, value, index) do
    path = "config.rules[#{index}]"
    rule = Check.object(value, path)

    field =
      Check.text(rule, "field", path) || Check.fail("#{path}.field must be a nonblank string")

    threshold = Check.optional_number(rule, "threshold", path)
    legacy = Check.optional_number(rule, "value", path)

    if threshold != nil and legacy != nil and threshold != legacy,
      do: Check.fail("#{path}.threshold conflicts with #{path}.value")

    threshold =
      threshold || legacy || Check.fail("#{path}.threshold (or value) must be a finite number")

    comparison = Check.choice(rule, "comparison", "gte", ["gt", "gte", "lt", "lte", "eq"], path)
    severity = Check.choice(rule, "severity", "warn", ["warn", "block"], path)
    label = Check.text(rule, "label", path, field)
    source = Check.text(rule, "source", path)

    {object, source_path} =
      if source do
        source_path = "payload.bundle_payloads.#{source}"

        {Check.object(get_in(payload, ["bundle_payloads", source]), "#{path}: #{source_path}"),
         source_path}
      else
        {payload, "payload"}
      end

    value = Check.number(object[field], "#{path}: #{source_path}.#{field}")

    triggered =
      case comparison do
        "gt" -> value > threshold
        "gte" -> value >= threshold
        "lt" -> value < threshold
        "lte" -> value <= threshold
        "eq" -> value == threshold
      end

    if triggered,
      do: %{
        "field" => field,
        "source" => source || "bundle",
        "value" => value,
        "threshold" => threshold,
        "comparison" => comparison,
        "severity" => severity,
        "label" => label
      }
  end

  defp summary("pass", _triggers), do: "All diagnostics bundle guard rules passed."

  defp summary(status, triggers) do
    lead =
      triggers
      |> Enum.take(2)
      |> Enum.map_join(", ", fn trigger ->
        "#{trigger["source"]}.#{trigger["label"]}=#{trigger["value"]}"
      end)

    "#{String.upcase(status)}: #{length(triggers)} trigger(s) (#{lead})."
  end
end
