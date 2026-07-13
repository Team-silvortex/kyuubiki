defmodule KyuubikiWeb.Storage.ResultRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:job_id, :string, autogenerate: false}
  schema "kyuubiki_analysis_results" do
    field(:payload, :map)
    timestamps(type: :utc_datetime_usec)
  end
end
