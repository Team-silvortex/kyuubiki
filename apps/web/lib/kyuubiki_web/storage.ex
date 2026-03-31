defmodule KyuubikiWeb.Storage do
  @moduledoc false

  def backend do
    Application.get_env(:kyuubiki_web, :storage_backend, :memory)
  end

  def postgres?, do: backend() == :postgres
end
