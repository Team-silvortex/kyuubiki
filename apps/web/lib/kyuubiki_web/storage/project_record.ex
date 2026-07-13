defmodule KyuubikiWeb.Storage.ProjectRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:project_id, :string, autogenerate: false}
  schema "kyuubiki_projects" do
    field(:name, :string)
    field(:description, :string)
    timestamps(type: :utc_datetime_usec)
  end
end
