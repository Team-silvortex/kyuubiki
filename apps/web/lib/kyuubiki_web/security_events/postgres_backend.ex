defmodule KyuubikiWeb.SecurityEvents.PostgresBackend do
  @moduledoc false

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.SecurityEventRecord

  def create(attrs) when is_map(attrs) do
    with {:ok, event_id} <- require_binary(attrs, "event_id"),
         {:ok, event_type} <- require_binary(attrs, "event_type"),
         {:ok, source} <- require_binary(attrs, "source"),
         {:ok, action} <- require_binary(attrs, "action"),
         {:ok, risk} <- require_binary(attrs, "risk"),
         {:ok, status} <- require_binary(attrs, "status"),
         {:ok, occurred_at} <- parse_occurred_at(Map.get(attrs, "occurred_at")) do
      record_attrs = %{
        event_id: event_id,
        event_type: event_type,
        source: source,
        action: action,
        risk: risk,
        status: status,
        note: stringify_note(Map.get(attrs, "note")),
        context: normalize_context(Map.get(attrs, "context")),
        occurred_at: occurred_at,
        inserted_at: DateTime.utc_now(),
        updated_at: DateTime.utc_now()
      }

      record =
        %SecurityEventRecord{}
        |> Ecto.Changeset.change(record_attrs)
        |> repo_insert!()

      {:ok, serialize(record)}
    end
  end

  def list do
    repo_all(SecurityEventRecord)
    |> Enum.sort_by(& &1.occurred_at, {:desc, DateTime})
    |> Enum.map(&serialize/1)
  end

  def reset do
    repo_delete_all(SecurityEventRecord)
    :ok
  end

  defp serialize(record) do
    %{
      "event_id" => record.event_id,
      "event_type" => record.event_type,
      "source" => record.source,
      "action" => record.action,
      "risk" => record.risk,
      "status" => record.status,
      "note" => record.note,
      "context" => record.context || %{},
      "occurred_at" => DateTime.to_iso8601(record.occurred_at),
      "inserted_at" => DateTime.to_iso8601(record.inserted_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    }
  end

  defp parse_occurred_at(value) when is_binary(value) do
    case DateTime.from_iso8601(value) do
      {:ok, datetime, _offset} ->
        {:ok, datetime |> DateTime.to_unix(:microsecond) |> DateTime.from_unix!(:microsecond)}

      _ ->
        {:error, {:invalid_security_event_field, "occurred_at"}}
    end
  end

  defp parse_occurred_at(_value), do: {:error, {:invalid_security_event_field, "occurred_at"}}

  defp require_binary(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) ->
        if byte_size(String.trim(value)) > 0 do
          {:ok, value}
        else
          {:error, {:invalid_security_event_field, key}}
        end

      _ ->
        {:error, {:invalid_security_event_field, key}}
    end
  end

  defp stringify_note(value) when is_binary(value), do: value
  defp stringify_note(value) when is_nil(value), do: nil
  defp stringify_note(value), do: to_string(value)

  defp normalize_context(value) when is_map(value), do: value
  defp normalize_context(_value), do: %{}

  defp repo do
    Storage.repo_module!()
  end

  defp repo_all(queryable), do: apply(repo(), :all, [queryable])
  defp repo_insert!(changeset), do: apply(repo(), :insert!, [changeset])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])
end
