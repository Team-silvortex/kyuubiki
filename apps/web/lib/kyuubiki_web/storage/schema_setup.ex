defmodule KyuubikiWeb.Storage.SchemaSetup do
  @moduledoc false
  use GenServer

  def start_link(_opts), do: GenServer.start_link(__MODULE__, :ok, name: __MODULE__)

  @impl true
  def init(:ok) do
    storage = KyuubikiWeb.Storage
    KyuubikiWeb.Storage.DatabaseMigrations.boot!(storage.repo_module!(), storage.backend())
    {:ok, %{}}
  end
end
