defmodule KyuubikiWeb.SecurityEvents.MemoryBackend do
  @moduledoc false

  use Agent

  alias KyuubikiWeb.Persistence

  def start_link(_opts) do
    Agent.start_link(fn -> load_events() end, name: __MODULE__)
  end

  def create(attrs) when is_map(attrs) do
    with {:ok, event} <- normalize_event(attrs) do
      Agent.update(__MODULE__, fn events ->
        updated = [event | events] |> Enum.take(500)
        Persistence.write_json!(Persistence.security_events_path(), updated)
        updated
      end)

      {:ok, event}
    end
  end

  def list do
    Agent.get(__MODULE__, fn events -> events end)
  end

  def reset do
    Agent.update(__MODULE__, fn _ ->
      Persistence.write_json!(Persistence.security_events_path(), [])
      []
    end)
  end

  defp load_events do
    Persistence.read_json(Persistence.security_events_path(), [])
    |> Enum.filter(&is_map/1)
  end

  defp normalize_event(attrs) do
    with {:ok, event_id} <- require_binary(attrs, "event_id"),
         {:ok, event_type} <- require_binary(attrs, "event_type"),
         {:ok, source} <- require_binary(attrs, "source"),
         {:ok, action} <- require_binary(attrs, "action"),
         {:ok, risk} <- require_binary(attrs, "risk"),
         {:ok, status} <- require_binary(attrs, "status"),
         {:ok, occurred_at} <- require_binary(attrs, "occurred_at") do
      {:ok,
       %{
         "event_id" => event_id,
         "event_type" => event_type,
         "source" => source,
         "action" => action,
         "risk" => risk,
         "status" => status,
         "note" => stringify_note(Map.get(attrs, "note")),
         "context" => normalize_context(Map.get(attrs, "context")),
         "occurred_at" => occurred_at,
         "inserted_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
         "updated_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
       }}
    end
  end

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
end
