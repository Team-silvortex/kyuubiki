defmodule KyuubikiWeb.WorkflowMaterialCardBatchRuntime do
  @moduledoc false

  alias KyuubikiWeb.WorkflowMaterialCardRuntime

  def validate_material_card_batch(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, cards} <- material_cards(payload),
         {:ok, reports} <- validate_cards(cards, config) do
      {:ok, summarize_reports(reports)}
    end
  end

  def validate_material_card_batch(_payload, _config),
    do: {:error, :invalid_material_card_batch_payload}

  defp material_cards(%{"material_cards" => cards}) when is_map(cards) and map_size(cards) > 0 do
    {:ok, Enum.map(cards, fn {id, card} -> {id, card} end)}
  end

  defp material_cards(%{"materials" => cards}) when is_list(cards) and cards != [] do
    {:ok, Enum.with_index(cards, &{card_id(&1, &2), &1})}
  end

  defp material_cards(%{"materials" => cards}) when is_map(cards) and map_size(cards) > 0 do
    {:ok, Enum.map(cards, fn {id, card} -> {id, card} end)}
  end

  defp material_cards(_payload), do: {:error, :missing_material_cards}

  defp validate_cards(cards, config) do
    reports =
      cards
      |> Enum.map(fn {candidate_id, card} ->
        case WorkflowMaterialCardRuntime.validate_material_card(card, config) do
          {:ok, report} ->
            report
            |> Map.put("candidate_id", candidate_id)
            |> Map.put_new("material_card_id", candidate_id)
            |> Map.put("material_card_parameters", parameter_snapshot(card))

          {:error, reason} ->
            invalid_card_report(candidate_id, reason)
        end
      end)

    {:ok, reports}
  end

  defp summarize_reports(reports) do
    grouped = Enum.group_by(reports, & &1["material_card_preflight_status"])
    usable = Enum.filter(reports, &(&1["material_card_preflight_status"] in ["pass", "warn"]))
    blocked = Map.get(grouped, "fail", [])

    %{
      "material_card_batch_schema" => "kyuubiki.material-card-batch-preflight/v1",
      "material_card_batch_count" => length(reports),
      "material_card_batch_pass_count" => grouped |> Map.get("pass", []) |> length(),
      "material_card_batch_warn_count" => grouped |> Map.get("warn", []) |> length(),
      "material_card_batch_fail_count" => length(blocked),
      "material_card_batch_usable_count" => length(usable),
      "material_card_batch_blocked_count" => length(blocked),
      "material_card_batch_best_review_ready_id" => best_review_ready_id(usable),
      "material_card_batch_issue_counts" => issue_counts(reports),
      "material_card_batch_trust_levels" => trust_level_counts(reports),
      "material_card_batch_reports" => reports
    }
  end

  defp invalid_card_report(candidate_id, reason) do
    %{
      "candidate_id" => candidate_id,
      "material_card_id" => candidate_id,
      "material_card_preflight_status" => "fail",
      "material_card_issue_count" => 1,
      "material_card_error_count" => 1,
      "material_card_warning_count" => 0,
      "material_card_quality_gates" => %{
        "status" => "fail",
        "schema_valid" => false,
        "units_valid" => false,
        "provenance_valid" => false,
        "parameters_valid" => false
      },
      "material_card_issues" => [
        %{
          "severity" => "error",
          "code" => "invalid_material_card",
          "field" => "material_card",
          "message" => inspect(reason)
        }
      ],
      "material_card_reliability_envelope" => %{
        "schema" => "kyuubiki.material-card-reliability/v1",
        "trust_level" => "blocked",
        "material_card_id" => candidate_id,
        "source_id" => "",
        "confidence_level" => "unknown",
        "limitations" => ["invalid_material_card"]
      }
    }
  end

  defp parameter_snapshot(%{"parameters" => parameters}) when is_map(parameters) do
    Map.new(parameters, fn {name, parameter} ->
      {name,
       %{
         "kind" => Map.get(parameter, "kind"),
         "unit" => Map.get(parameter, "unit"),
         "value" => Map.get(parameter, "value")
       }}
    end)
  end

  defp parameter_snapshot(_card), do: %{}

  defp issue_counts(reports) do
    reports
    |> Enum.flat_map(&Map.get(&1, "material_card_issues", []))
    |> Enum.reduce(%{}, fn issue, acc ->
      Map.update(acc, Map.get(issue, "code", "unknown"), 1, &(&1 + 1))
    end)
  end

  defp trust_level_counts(reports) do
    reports
    |> Enum.map(&(get_in(&1, ["material_card_reliability_envelope", "trust_level"]) || "unknown"))
    |> Enum.reduce(%{}, fn trust_level, acc -> Map.update(acc, trust_level, 1, &(&1 + 1)) end)
  end

  defp best_review_ready_id(usable) do
    usable
    |> Enum.find(fn report ->
      get_in(report, ["material_card_reliability_envelope", "trust_level"]) == "review_ready"
    end)
    |> case do
      nil -> ""
      report -> Map.get(report, "material_card_id", "")
    end
  end

  defp card_id(%{"material_id" => id}, _index) when is_binary(id) and id != "", do: id
  defp card_id(_card, index), do: "material_card_#{index + 1}"
end
