defmodule KyuubikiWeb.Storage.JobRecord do
  @moduledoc false

  use Ecto.Schema

  @primary_key {:job_id, :string, autogenerate: false}
  schema "kyuubiki_jobs" do
    field(:project_id, :string)
    field(:model_version_id, :string)
    field(:simulation_case_id, :string)
    field(:worker_id, :string)
    field(:message, :string)
    field(:status, :string)
    field(:progress, :float)
    field(:residual, :float)
    field(:iteration, :integer)
    field(:created_at, :utc_datetime_usec)
    field(:updated_at, :utc_datetime_usec)
  end
end
