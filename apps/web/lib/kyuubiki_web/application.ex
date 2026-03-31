defmodule KyuubikiWeb.Application do
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = storage_children() ++ maybe_http_server()

    Supervisor.start_link(children, strategy: :one_for_one, name: KyuubikiWeb.Supervisor)
  end

  defp storage_children do
    if KyuubikiWeb.Storage.postgres?() do
      [
        KyuubikiWeb.Repo,
        {KyuubikiWeb.Storage.SchemaSetup, []}
      ]
    else
      [
        {KyuubikiWeb.Jobs.MemoryBackend, []},
        {KyuubikiWeb.AnalysisResultMemoryBackend, []},
        {KyuubikiWeb.Library.MemoryBackend, []}
      ]
    end
  end

  defp maybe_http_server do
    if Mix.env() == :test do
      []
    else
      [{Plug.Cowboy, scheme: :http, plug: KyuubikiWeb.Router, options: [port: port()]}]
    end
  end

  defp port do
    System.get_env("PORT", "4000") |> String.to_integer()
  end
end
