defmodule KyuubikiWeb.WorkflowSweepChecks do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check

  def count(value, path, minimum \\ 0) do
    value =
      if is_float(value) && value >= 0 && value == trunc(value), do: trunc(value), else: value

    value = Check.unsigned(value, path)
    if value < minimum, do: Check.fail("#{path} must be a whole count >= #{minimum}")
    value
  end

  def flag(map, key, default \\ true, path \\ "payload") do
    case Map.get(map, key, default) do
      value when is_boolean(value) -> value
      _ -> Check.fail("#{path}.#{key} must be a boolean")
    end
  end

  def text(map, key, default, path) do
    value = Map.get(map, key, default)

    if !is_binary(value) || String.trim(value) == "",
      do: Check.fail("#{path}.#{key} must be a nonblank string")

    value
  end

  def json(value, path) when is_map(value) do
    value = Check.object(value, path)
    Enum.each(value, fn {key, child} -> json(child, "#{path}.#{key}") end)
    value
  end

  def json(value, path) when is_list(value) do
    Enum.with_index(value) |> Enum.each(fn {child, index} -> json(child, "#{path}[#{index}]") end)
    value
  end

  def json(value, path) when is_number(value) do
    Check.number(value, path)
    value
  end

  def json(value, _path) when is_binary(value) or is_boolean(value) or is_nil(value), do: value
  def json(_value, path), do: Check.fail("#{path} must contain JSON values")

  def object(value, path), do: value |> Check.object(path) |> json(path)

  def values(value, path) do
    values = Check.list(value, path)
    if values == [], do: Check.fail("#{path} must not be empty")
    json(values, path)
  end

  def product(counts),
    do:
      Enum.reduce(counts, 1, fn count, total ->
        Check.unsigned(total * count, "case_count_estimate")
      end)

  def budget(payload) do
    budget = Check.object(Map.get(payload, "sweep_budget", %{}), "payload.sweep_budget")
    {budget, flag(budget, "case_budget_exceeded", false, "payload.sweep_budget")}
  end

  def require_count(payload, key, actual) do
    if Map.has_key?(payload, key) && count(payload[key], "payload.#{key}") != actual,
      do: Check.fail("payload.#{key} disagrees with actual count #{actual}")
  end

  def axes_count(axes, deferred \\ false) do
    axes = Check.list(axes, "payload.axes")
    if axes == [], do: Check.fail("payload.axes must not be empty")

    axes
    |> Enum.with_index()
    |> Enum.map(fn {axis, index} ->
      path = "payload.axes[#{index}]"
      axis = Check.object(axis, path)
      segments(text(axis, "path", nil, path), path)

      if flag(axis, "values_deferred", false, path) do
        if !deferred, do: Check.fail("#{path}.values_deferred requires a blocked budget")

        if Map.has_key?(axis, "values"),
          do: Check.fail("#{path} cannot declare both deferred and concrete values")

        count(axis["value_count"], "#{path}.value_count", 1)
      else
        size = length(values(axis["values"], "#{path}.values"))
        require_count(axis, "value_count", size)
        size
      end
    end)
    |> product()
  end

  def prepare_axes(base, raw) do
    axes_count(raw)

    axes =
      raw
      |> Enum.with_index()
      |> Enum.map(fn {axis, index} ->
        context = "payload.axes[#{index}]"
        path = text(axis, "path", nil, context) |> String.trim()
        label = text(axis, "label", path, context)
        target = resolve(base, segments(path, context), context)
        %{path: path, label: label, target: target, values: axis["values"]}
      end)

    labels = Enum.map(axes, & &1.label)

    if length(Enum.uniq(labels)) != length(labels),
      do: Check.fail("payload.axes contains duplicate labels")

    axes
    |> Enum.sort_by(& &1.target)
    |> Enum.chunk_every(2, 1, :discard)
    |> Enum.each(fn [left, right] ->
      if Enum.take(right.target, length(left.target)) == left.target,
        do: Check.fail("payload.axes paths #{left.path} and #{right.path} overlap")
    end)

    axes
  end

  def segments(path, context) do
    segments = String.split(path, ".")

    if Enum.any?(segments, &(String.trim(&1) == "")),
      do: Check.fail("#{context}: path segments must not be empty")

    segments
  end

  defp resolve(map, [key], _context) when is_map(map), do: [{:key, key}]

  defp resolve(map, [key | rest], context) when is_map(map) do
    if !Map.has_key?(map, key), do: Check.fail("#{context}: missing parent field #{key}")
    [{:key, key} | resolve(map[key], rest, context)]
  end

  defp resolve(list, [segment | rest], context) when is_list(list) do
    if !Regex.match?(~r/^[0-9]+$/, segment),
      do: Check.fail("#{context}: array index #{segment} must be nonnegative decimal")

    index = String.to_integer(segment)

    case Enum.fetch(list, index) do
      {:ok, value} ->
        [{:index, index} | if(rest == [], do: [], else: resolve(value, rest, context))]

      :error ->
        Check.fail("#{context}: array index #{segment} is out of range")
    end
  end

  defp resolve(_value, _rest, context),
    do: Check.fail("#{context}: path requires an object or array parent")

  def assign(_value, [], new), do: new

  def assign(map, [{:key, key} | rest], new),
    do: Map.put(map, key, assign(Map.get(map, key), rest, new))

  def assign(list, [{:index, index} | rest], new),
    do: List.update_at(list, index, &assign(&1, rest, new))

  def case_ids(cases) do
    ids =
      cases
      |> Enum.with_index()
      |> Enum.map(fn {entry, index} ->
        path = "payload.cases[#{index}]"
        optional_case_id(entry, path) || "case_#{index}"
      end)

    if length(Enum.uniq(ids)) != length(ids),
      do: Check.fail("payload.cases contains duplicate case ids")

    ids
  end

  def optional_case_id(entry, path) do
    entry = Check.object(entry, path)

    aliases =
      for key <- ~w(id case_id caseId),
          Map.has_key?(entry, key),
          do: text(entry, key, nil, path)

    if length(Enum.uniq(aliases)) > 1,
      do: Check.fail("#{path} has conflicting case identity aliases")

    List.first(aliases)
  end
end
