defmodule KyuubikiWeb.WorkflowParameterSweepRuntime do
  @moduledoc false

  def expand_parameter_sweep(payload, config) when is_map(payload) and is_map(config) do
    with {:ok, payload, config} <- normalize_expand_input(payload, config),
         {:ok, axes} <- parse_axes(Map.get(payload, "axes") || Map.get(config, "axes")),
         {:ok, base} <- fetch_base(payload),
         {:ok, max_cases} <- positive_int(config, "max_cases", 256),
         id_prefix <- Map.get(config, "id_prefix", "case"),
         case_count <- Enum.reduce(axes, 1, fn axis, count -> count * length(axis.values) end),
         :ok <- validate_case_count(case_count, max_cases),
         {:ok, cases} <- expand_axes(base, axes, id_prefix) do
      {:ok,
       %{
         "cases" => cases,
         "case_count" => length(cases),
         "axis_count" => length(axes)
       }}
    end
  end

  def expand_parameter_sweep(_payload, _config), do: {:error, :invalid_parameter_sweep_payload}

  def join_parameter_sweep_results(payload, config) when is_map(payload) and is_map(config) do
    with cases when is_list(cases) and cases != [] <- Map.get(payload, "cases"),
         results when is_list(results) <-
           Map.get(payload, "summaries") || Map.get(payload, "results") do
      summary_field = Map.get(config, "summary_field", "summary")
      output_field = Map.get(config, "output_field", "summary")
      strict = Map.get(config, "strict", false)

      {joined, missing, joined_count} =
        cases
        |> Enum.with_index()
        |> Enum.reduce({[], [], 0}, fn {case_payload, index}, {joined, missing, count} ->
          case_id = case_id(case_payload, index)

          case find_case_result(results, case_id, index) do
            result when is_map(result) ->
              summary = extract_join_summary(result, summary_field)

              if is_nil(summary) do
                {[Map.put(case_payload, "result_status", "missing") | joined],
                 [case_id | missing], count}
              else
                next_case =
                  case_payload
                  |> Map.put(output_field, summary)
                  |> Map.put("result_status", Map.get(result, "status", "joined"))

                {[next_case | joined], missing, count + 1}
              end

            _ ->
              {[Map.put(case_payload, "result_status", "missing") | joined], [case_id | missing],
               count}
          end
        end)

      if strict and missing != [] do
        {:error, {:missing_parameter_sweep_results, length(missing)}}
      else
        {:ok,
         %{
           "cases" => Enum.reverse(joined),
           "case_count" => length(cases),
           "joined_summary_count" => joined_count,
           "missing_summary_count" => length(missing),
           "missing_case_ids" => Enum.reverse(missing)
         }}
      end
    else
      _ -> {:error, :invalid_parameter_sweep_join_payload}
    end
  end

  def join_parameter_sweep_results(_payload, _config),
    do: {:error, :invalid_parameter_sweep_join_payload}

  def summarize_parameter_sweep(payload, config) when is_map(payload) and is_map(config) do
    with cases when is_list(cases) and cases != [] <- Map.get(payload, "cases") do
      fields = config |> Map.get("fields", []) |> Enum.filter(&is_binary/1)
      include_parameters = Map.get(config, "include_parameters", true)

      rows =
        cases
        |> Enum.with_index()
        |> Enum.map(fn {case_payload, index} ->
          summary = Map.get(case_payload, "summary") || Map.get(case_payload, "result") || %{}

          %{"case_id" => case_id(case_payload, index)}
          |> maybe_put_parameters(case_payload, include_parameters)
          |> Map.merge(select_summary_fields(summary, fields))
        end)

      {:ok,
       %{
         "rows" => rows,
         "row_count" => length(rows),
         "numeric_columns" => numeric_columns(rows)
       }}
    else
      _ -> {:error, :invalid_parameter_sweep_summary_payload}
    end
  end

  def summarize_parameter_sweep(_payload, _config),
    do: {:error, :invalid_parameter_sweep_summary_payload}

  def score_parameter_sweep(payload, config) when is_map(payload) and is_map(config) do
    with rows when is_list(rows) and rows != [] <- Map.get(payload, "rows"),
         objectives when is_list(objectives) and objectives != [] <- Map.get(config, "objectives") do
      scored =
        rows
        |> Enum.with_index()
        |> Enum.map(fn {row, index} -> score_sweep_row(row, objectives, index) end)
        |> Enum.sort_by(&Map.get(&1, "objective_score", -1.0e18), :desc)

      {:ok,
       %{
         "best" => hd(scored),
         "scored_rows" => scored,
         "scored_count" => length(scored)
       }}
    else
      _ -> {:error, :invalid_parameter_sweep_score_payload}
    end
  end

  def score_parameter_sweep(_payload, _config),
    do: {:error, :invalid_parameter_sweep_score_payload}

  defp normalize_expand_input(
         %{"quality_sweep_expansion_contract" => _contract, "expansion_enabled" => false},
         _config
       ),
       do: {:error, :disabled_quality_sweep_expansion}

  defp normalize_expand_input(
         %{"quality_sweep_expansion_contract" => _contract} = payload,
         config
       ) do
    case Map.get(payload, "payload") do
      nested_payload when is_map(nested_payload) ->
        nested_config = Map.get(payload, "config", %{})
        {:ok, nested_payload, Map.merge(nested_config, config)}

      _ ->
        {:error, :invalid_quality_sweep_expansion_payload}
    end
  end

  defp normalize_expand_input(payload, config), do: {:ok, payload, config}

  defp fetch_base(payload) do
    case Map.get(payload, "base") || Map.get(payload, "model") do
      base when is_map(base) -> {:ok, base}
      _ -> {:error, :missing_parameter_sweep_base}
    end
  end

  defp parse_axes(axes) when is_list(axes) and axes != [] do
    axes
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, []}, fn {axis, index}, {:ok, acc} ->
      with %{} <- axis,
           path when is_binary(path) and path != "" <- Map.get(axis, "path"),
           values when is_list(values) and values != [] <- Map.get(axis, "values") do
        label = Map.get(axis, "label", path)
        {:cont, {:ok, [%{label: label, path: path, values: values} | acc]}}
      else
        _ -> {:halt, {:error, {:invalid_parameter_sweep_axis, index}}}
      end
    end)
    |> case do
      {:ok, parsed} -> {:ok, Enum.reverse(parsed)}
      error -> error
    end
  end

  defp parse_axes(_axes), do: {:error, :missing_parameter_sweep_axes}

  defp positive_int(config, key, default) do
    value = Map.get(config, key, default)

    cond do
      is_integer(value) and value > 0 -> {:ok, value}
      is_float(value) and value > 0 -> {:ok, round(value)}
      true -> {:error, {:invalid_positive_integer_config, key}}
    end
  end

  defp validate_case_count(0, _max_cases), do: {:error, :empty_parameter_sweep}

  defp validate_case_count(count, max_cases) when count > max_cases,
    do: {:error, {:parameter_sweep_case_limit_exceeded, count, max_cases}}

  defp validate_case_count(_count, _max_cases), do: :ok

  defp expand_axes(base, axes, id_prefix) do
    axes
    |> Enum.reduce([{%{}, base}], fn axis, cases ->
      for {parameters, model} <- cases, value <- axis.values do
        model = unwrap_model(model)
        parameters = Map.put(parameters, axis.label, value)
        {parameters, put_dotted_path(model, axis.path, value)}
      end
    end)
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, []}, fn {{parameters, model_result}, index}, {:ok, acc} ->
      case model_result do
        {:ok, model} ->
          case_payload = %{
            "id" => "#{id_prefix}_#{index}",
            "label" => format_case_label(parameters),
            "parameters" => parameters,
            "model" => model
          }

          {:cont, {:ok, [case_payload | acc]}}

        {:error, reason} ->
          {:halt, {:error, reason}}
      end
    end)
    |> case do
      {:ok, cases} -> {:ok, Enum.reverse(cases)}
      error -> error
    end
  end

  defp put_dotted_path(model, path, value),
    do: put_path(model, String.split(path, ".", trim: true), value)

  defp put_path(_value, [], _new_value), do: {:error, :empty_parameter_sweep_path}

  defp put_path(map, [segment], value) when is_map(map), do: {:ok, Map.put(map, segment, value)}

  defp put_path(list, [segment | rest], value) when is_list(list) do
    with {index, ""} <- Integer.parse(segment),
         item when not is_nil(item) <- Enum.at(list, index),
         {:ok, updated} <- put_path(item, rest, value) do
      {:ok, List.replace_at(list, index, updated)}
    else
      _ -> {:error, {:invalid_parameter_sweep_path_segment, segment}}
    end
  end

  defp put_path(map, [segment | rest], value) when is_map(map) do
    case Map.fetch(map, segment) do
      {:ok, child} ->
        with {:ok, updated} <- put_path(child, rest, value) do
          {:ok, Map.put(map, segment, updated)}
        end

      :error ->
        {:error, {:missing_parameter_sweep_path_segment, segment}}
    end
  end

  defp put_path(_value, [segment | _rest], _new_value),
    do: {:error, {:invalid_parameter_sweep_path_segment, segment}}

  defp unwrap_model({:ok, model}), do: model
  defp unwrap_model(model), do: model

  defp format_case_label(parameters) do
    parameters
    |> Enum.map(fn {key, value} -> "#{key}=#{value}" end)
    |> Enum.join(", ")
  end

  defp case_id(case_payload, index) when is_map(case_payload),
    do: Map.get(case_payload, "id", "case_#{index}")

  defp find_case_result(results, case_id, index) do
    Enum.find(results, fn result ->
      is_map(result) and Enum.any?(["case_id", "id", "caseId"], &(Map.get(result, &1) == case_id))
    end) || Enum.at(results, index)
  end

  defp extract_join_summary(result, summary_field) when is_map(result),
    do:
      Map.get(result, summary_field) || Map.get(result, "summary") || Map.get(result, "result") ||
        result

  defp maybe_put_parameters(row, case_payload, true),
    do: Map.put(row, "parameters", Map.get(case_payload, "parameters"))

  defp maybe_put_parameters(row, _case_payload, _include_parameters), do: row

  defp select_summary_fields(summary, []) when is_map(summary), do: summary

  defp select_summary_fields(summary, fields) when is_map(summary) do
    summary
    |> Map.take(fields)
  end

  defp select_summary_fields(_summary, _fields), do: %{}

  defp numeric_columns(rows) do
    rows
    |> Enum.reduce(%{}, fn row, columns ->
      Enum.reduce(row, columns, fn
        {field, value}, acc when is_number(value) ->
          update_numeric_column(acc, field, value)

        _entry, acc ->
          acc
      end)
    end)
  end

  defp update_numeric_column(columns, field, value) do
    entry = Map.get(columns, field, %{"count" => 0, "min" => value, "max" => value, "sum" => 0.0})
    count = entry["count"] + 1
    sum = entry["sum"] + value

    Map.put(columns, field, %{
      "count" => count,
      "min" => min(entry["min"], value),
      "max" => max(entry["max"], value),
      "sum" => sum,
      "mean" => sum / count
    })
  end

  defp score_sweep_row(row, objectives, index) when is_map(row) do
    {score, feasible, breakdown} =
      Enum.reduce(objectives, {0.0, true, []}, fn objective, {total, feasible, breakdown} ->
        field = Map.get(objective, "field")
        value = Map.get(row, field)
        goal = Map.get(objective, "goal", "min")
        weight = Map.get(objective, "weight", 1.0)
        term_score = objective_score(goal, value, objective) * weight

        {total + term_score, feasible and objective_limit_allows?(objective, value),
         [
           %{
             "field" => field,
             "goal" => goal,
             "weight" => weight,
             "value" => value,
             "score" => term_score
           }
           | breakdown
         ]}
      end)

    row
    |> Map.put("case_id", Map.get(row, "case_id", "case_#{index}"))
    |> Map.put("objective_score", if(feasible, do: score, else: score - 1.0e12))
    |> Map.put("objective_feasible", feasible)
    |> Map.put("objective_breakdown", Enum.reverse(breakdown))
  end

  defp objective_score("max", value, _objective) when is_number(value), do: value

  defp objective_score("target", value, %{"target" => target})
       when is_number(value) and is_number(target),
       do: -abs(value - target)

  defp objective_score(_goal, value, _objective) when is_number(value), do: -value
  defp objective_score(_goal, _value, _objective), do: -1.0e12

  defp objective_limit_allows?(objective, value) when is_number(value) do
    minimum_ok = !is_number(objective["min_allowed"]) or value >= objective["min_allowed"]
    maximum_ok = !is_number(objective["max_allowed"]) or value <= objective["max_allowed"]
    minimum_ok and maximum_ok
  end

  defp objective_limit_allows?(_objective, _value), do: false
end
