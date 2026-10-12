defmodule KyuubikiWeb.Storage.ProjectRequestSchema do
  @moduledoc false

  # Frozen revision 3. No foreign key: deleted projects retain a replay tombstone.
  def statements(backend) when backend in [:sqlite, :postgres] do
    json = if backend == :sqlite, do: "JSON", else: "JSONB"

    [
      """
      CREATE TABLE IF NOT EXISTS kyuubiki_project_requests (
        request_key TEXT PRIMARY KEY,
        request_digest TEXT NOT NULL,
        project_id TEXT NOT NULL,
        response_meta #{json} NOT NULL
      )
      """
    ]
  end
end
