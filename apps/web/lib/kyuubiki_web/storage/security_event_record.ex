defmodule KyuubikiWeb.Storage.SecurityEventRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:event_id, :string, autogenerate: false}
  schema "kyuubiki_security_events" do
    field(:event_type, :string)
    field(:source, :string)
    field(:action, :string)
    field(:risk, :string)
    field(:status, :string)
    field(:note, :string)
    field(:context, :map)
    field(:occurred_at, :utc_datetime_usec)
    timestamps(type: :utc_datetime_usec)
  end
end
