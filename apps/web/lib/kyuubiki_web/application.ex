defmodule KyuubikiWeb.Application do
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {KyuubikiWeb.Jobs.Store, []}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: KyuubikiWeb.Supervisor)
  end
end
