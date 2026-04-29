defmodule KyuubikiWeb.SecurityEvents.Store do
  @moduledoc """
  Append-only store for structured security-sensitive automation events.
  """

  alias KyuubikiWeb.Storage

  def create(attrs), do: backend().create(attrs)
  def list, do: backend().list()
  def reset, do: backend().reset()

  defp backend do
    if Storage.sql?() do
      KyuubikiWeb.SecurityEvents.PostgresBackend
    else
      KyuubikiWeb.SecurityEvents.MemoryBackend
    end
  end
end
