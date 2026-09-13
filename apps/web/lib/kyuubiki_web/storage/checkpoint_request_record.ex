defmodule KyuubikiWeb.Storage.CheckpointRequestRecord do
  @moduledoc false
  use Ecto.Schema

  @primary_key {:request_key, :string, autogenerate: false}
  schema "kyuubiki_checkpoint_requests" do
    field(:request_digest, :string)
    field(:project_id, :string)
    field(:model_id, :string)
    field(:version_id, :string)
    field(:response_meta, :map)
  end
end
