defmodule KyuubikiWeb.SecurityEvents.Store do
  @moduledoc """
  Append-only store for structured security-sensitive automation events.
  """

  alias KyuubikiWeb.Storage

  def create(attrs), do: backend().create(attrs)
  def list(filters \\ %{}), do: backend().list(filters)
  def reset, do: backend().reset()

  defp backend do
    if Storage.sql?() do
      KyuubikiWeb.SecurityEvents.PostgresBackend
    else
      KyuubikiWeb.SecurityEvents.MemoryBackend
    end
  end
end
