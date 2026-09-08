defmodule KyuubikiWeb.WorkflowHeatBridgeContract do
  @moduledoc false
  @fields ["node_i", "node_j", "node_k", "node_l"]

  def resolve(config) when is_map(config) do
    with {:ok, contract} <- object(Map.get(config, "contract", config)),
         {:ok, source} <- section(contract, "source"),
         {:ok, transform} <- section(contract, "transform"),
         {:ok, target} <- section(contract, "target"),
         {:ok, field} <- text(source, "field", "temperature"),
         {:ok, distribution} <- text(source, "distribution", "node_to_node"),
         {:ok, target_field} <- text(target, "field", "temperature_delta"),
         {:ok, reduction} <-
           text(
             transform,
             "reduction",
             if(distribution == "node_to_node", do: "copy", else: "mean")
           ),
         {:ok, scale} <- number(transform, "scale", 1.0, :invalid_bridge_scale),
         {:ok, default} <- number(transform, "default_value", 0.0, :invalid_bridge_default_value),
         {:ok, reference} <-
           number(transform, "reference_temperature", 0.0, :invalid_bridge_reference_temperature),
         {:ok, indexes} <- indexes(source),
         field = if(field == "heat_flux", do: "heat_flux_magnitude", else: field),
         :ok <- valid_source(field, distribution),
         :ok <- reference_source(field, reference),
         :ok <-
           supported(
             reduction,
             ["copy", "mean", "sum", "area_weighted_mean", "min", "max"],
             :unsupported_bridge_reduction
           ),
         :ok <-
           supported(target_field, ["temperature_delta"], :invalid_bridge_contract_target_field) do
      {:ok,
       %{
         source_field: field,
         distribution: distribution,
         target_field: target_field,
         node_index_fields: indexes,
         reduction: reduction,
         scale: scale,
         reference_temperature: reference,
         default_value: default
       }}
    end
  end

  def resolve(_), do: {:error, :invalid_bridge_contract}

  def for_shape(contract, shape) do
    fields = if shape == :triangle, do: Enum.take(@fields, 3), else: @fields
    indexes = if contract.node_index_fields == [], do: fields, else: contract.node_index_fields

    if Enum.all?(indexes, &(&1 in fields)),
      do: {:ok, Map.put(contract, :node_index_fields, indexes)},
      else: {:error, :invalid_bridge_contract_node_index_fields}
  end

  def finite_number?(value) when is_number(value), do: abs(value) <= 1.7976931348623157e308
  def finite_number?(_), do: false

  defp object(value) when is_map(value), do: {:ok, value}
  defp object(_), do: {:error, :invalid_bridge_contract}

  defp section(contract, key) do
    case Map.get(contract, key) do
      nil -> {:ok, %{}}
      value -> object(value)
    end
  end

  defp text(map, key, default) do
    value = Map.get(map, key, default)

    if is_binary(value) and value != "",
      do: {:ok, value},
      else: {:error, :invalid_bridge_contract}
  end

  defp number(map, key, default, error) do
    value = Map.get(map, key, default)
    if finite_number?(value), do: {:ok, value}, else: {:error, error}
  end

  defp indexes(source) do
    case Map.fetch(source, "node_index_fields") do
      :error ->
        {:ok, []}

      {:ok, values} when is_list(values) and values != [] ->
        if Enum.all?(values, &(&1 in @fields)) and Enum.uniq(values) == values,
          do: {:ok, values},
          else: {:error, :invalid_bridge_contract_node_index_fields}

      _ ->
        {:error, :invalid_bridge_contract_node_index_fields}
    end
  end

  defp valid_source(field, "node_to_node"),
    do: supported(field, ["temperature", "heat_load"], :invalid_bridge_contract_source_field)

  defp valid_source(field, "element_to_nodes"),
    do:
      supported(
        field,
        ["average_temperature", "heat_flux_x", "heat_flux_y", "heat_flux_magnitude"],
        :invalid_bridge_contract_source_field
      )

  defp valid_source(_, _), do: {:error, :unsupported_bridge_distribution}

  defp reference_source(field, value)
       when value != 0 and field not in ["temperature", "average_temperature"],
       do: {:error, :invalid_bridge_reference_temperature_source}

  defp reference_source(_, _), do: :ok
  defp supported(value, allowed, error), do: if(value in allowed, do: :ok, else: {:error, error})
end
