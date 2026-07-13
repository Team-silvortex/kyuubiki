defmodule KyuubikiWeb.Storage.ModelVersionRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:version_id, :string, autogenerate: false}
  schema "kyuubiki_model_versions" do
    field(:project_id, :string)
    field(:model_id, :string)
    field(:name, :string)
    field(:version_number, :integer)
    field(:kind, :string)
    field(:material, :string)
    field(:model_schema_version, :string)
    field(:payload, :map)
    timestamps(type: :utc_datetime_usec)
  end
end
