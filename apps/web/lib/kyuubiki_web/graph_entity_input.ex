defmodule KyuubikiWeb.GraphEntityInput do
  @moduledoc false

  def normalize(entities, prefix) when is_list(entities) do
    entities
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, [], MapSet.new()}, fn {entity, index}, {:ok, output, ids} ->
      with {:ok, normalized, id} <- normalize_entity(entity, prefix, index),
           false <- MapSet.member?(ids, id) do
        {:cont, {:ok, [normalized | output], MapSet.put(ids, id)}}
      else
        _ -> {:halt, {:error, :invalid_graph_entity_ids}}
      end
    end)
    |> case do
      {:ok, output, _ids} -> {:ok, Enum.reverse(output)}
      error -> error
    end
  end

  def normalize(_entities, _prefix), do: {:error, :invalid_graph_entity_ids}

  defp normalize_entity(entity, prefix, index) when is_map(entity) do
    case Map.get(entity, "id", Map.get(entity, :id, "")) do
      "" ->
        id = "#{prefix}#{index}"
        {:ok, entity |> Map.delete(:id) |> Map.put("id", id), id}

      id when is_binary(id) ->
        {:ok, entity, id}

      _ ->
        {:error, :invalid_graph_entity_ids}
    end
  end

  defp normalize_entity(_entity, _prefix, _index), do: {:error, :invalid_graph_entity_ids}
end
