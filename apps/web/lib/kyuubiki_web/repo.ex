defmodule KyuubikiWeb.Repo do
  @moduledoc false

  use Ecto.Repo,
    otp_app: :kyuubiki_web,
    adapter: Ecto.Adapters.Postgres
end
