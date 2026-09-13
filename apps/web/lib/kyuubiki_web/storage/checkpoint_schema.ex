defmodule KyuubikiWeb.Storage.CheckpointSchema do
  @moduledoc false

  # Frozen revision 2. A receipt survives model/version deletion until its project is deleted.
  def statements(backend) when backend in [:sqlite, :postgres] do
    json = if backend == :sqlite, do: "JSON", else: "JSONB"

    [
      """
      CREATE TABLE IF NOT EXISTS kyuubiki_checkpoint_requests (
        request_key TEXT PRIMARY KEY,
        request_digest TEXT NOT NULL,
        project_id TEXT NOT NULL REFERENCES kyuubiki_projects(project_id) ON DELETE CASCADE,
        model_id TEXT NOT NULL,
        version_id TEXT NOT NULL,
        response_meta #{json} NOT NULL
      )
      """
    ]
  end
end
