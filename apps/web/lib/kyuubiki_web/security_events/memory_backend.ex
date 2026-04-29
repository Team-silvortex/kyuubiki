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

  def list(filters \\ %{}) do
    Agent.get(__MODULE__, fn events ->
      events
      |> Enum.filter(&matches_filters?(&1, filters))
      |> Enum.take(normalize_limit(filters))
    end)
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

  defp normalize_limit(filters) do
    case Map.get(filters, "limit") do
      value when is_binary(value) ->
        case Integer.parse(value) do
          {parsed, ""} when parsed > 0 -> min(parsed, 500)
          _ -> 100
        end

      value when is_integer(value) and value > 0 ->
        min(value, 500)

      _ ->
        100
    end
  end

  defp matches_filters?(event, filters) do
    matches_exact?(event["source"], Map.get(filters, "source")) and
      matches_exact?(event["risk"], Map.get(filters, "risk")) and
      matches_exact?(event["status"], Map.get(filters, "status")) and
      matches_contains?(event["action"], Map.get(filters, "action")) and
      matches_time_after?(event["occurred_at"], Map.get(filters, "occurred_after")) and
      matches_time_before?(event["occurred_at"], Map.get(filters, "occurred_before")) and
      matches_context?(event["context"], "study_kind", Map.get(filters, "study_kind")) and
      matches_context?(event["context"], "project_id", Map.get(filters, "project_id")) and
      matches_context?(event["context"], "model_version_id", Map.get(filters, "model_version_id"))
  end

  defp matches_exact?(_value, nil), do: true
  defp matches_exact?(_value, ""), do: true
  defp matches_exact?(value, filter), do: value == filter

  defp matches_contains?(_value, nil), do: true
  defp matches_contains?(_value, ""), do: true

  defp matches_contains?(value, filter) when is_binary(value) and is_binary(filter) do
    String.contains?(String.downcase(value), String.downcase(filter))
  end

  defp matches_contains?(_value, _filter), do: false

  defp matches_time_after?(_value, nil), do: true
  defp matches_time_after?(_value, ""), do: true

  defp matches_time_after?(value, filter) when is_binary(value) and is_binary(filter) do
    case {DateTime.from_iso8601(value), DateTime.from_iso8601(filter)} do
      {{:ok, value_dt, _}, {:ok, filter_dt, _}} ->
        DateTime.compare(value_dt, filter_dt) in [:gt, :eq]

      _ ->
        false
    end
  end

  defp matches_time_after?(_value, _filter), do: false

  defp matches_time_before?(_value, nil), do: true
  defp matches_time_before?(_value, ""), do: true

  defp matches_time_before?(value, filter) when is_binary(value) and is_binary(filter) do
    case {DateTime.from_iso8601(value), DateTime.from_iso8601(filter)} do
      {{:ok, value_dt, _}, {:ok, filter_dt, _}} ->
        DateTime.compare(value_dt, filter_dt) in [:lt, :eq]

      _ ->
        false
    end
  end

  defp matches_time_before?(_value, _filter), do: false

  defp matches_context?(context, _key, nil) when is_map(context), do: true
  defp matches_context?(context, _key, "") when is_map(context), do: true

  defp matches_context?(context, key, filter) when is_map(context),
    do: Map.get(context, key) == filter

  defp matches_context?(_context, _key, nil), do: true
  defp matches_context?(_context, _key, ""), do: true
  defp matches_context?(_context, _key, _filter), do: false
end
