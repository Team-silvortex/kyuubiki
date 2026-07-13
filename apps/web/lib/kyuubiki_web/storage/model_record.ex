defmodule KyuubikiWeb.Storage.ModelRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:model_id, :string, autogenerate: false}
  schema "kyuubiki_models" do
    field(:project_id, :string)
    field(:name, :string)
    field(:kind, :string)
    field(:material, :string)
    field(:model_schema_version, :string)
    field(:payload, :map)
    field(:latest_version_id, :string)
    field(:latest_version_number, :integer)
    timestamps(type: :utc_datetime_usec)
  end
end
