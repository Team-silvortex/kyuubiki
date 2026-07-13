defmodule KyuubikiWeb.SqliteRepo do
  @moduledoc false

  use Ecto.Repo,
    otp_app: :kyuubiki_web,
    adapter: Ecto.Adapters.SQLite3
end
