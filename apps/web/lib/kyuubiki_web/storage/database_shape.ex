defmodule KyuubikiWeb.Storage.DatabaseShape do
  @moduledoc false
  alias KyuubikiWeb.Storage.{CheckpointSchema, MigrationQuery, RuntimeSchema}

  @legacy_columns ~w(model_version_id queue_timeout_ms execution_timeout_ms execution_started_at)

  def contracts(backend, version \\ 2) do
    Map.new(statements(backend, version), fn sql ->
      [_, table] = Regex.run(~r/CREATE TABLE IF NOT EXISTS (\w+)/, sql)
      primary = primary_keys(sql)

      columns =
        sql
        |> String.split("\n")
        |> Enum.flat_map(fn line ->
          case Regex.run(
                 ~r/^\s+(\w+) (TEXT|JSONB?|INTEGER|BIGINT|REAL|DOUBLE PRECISION|TIMESTAMPTZ|BOOLEAN)\b(.*)/,
                 line
               ) do
            [_, name, type, tail] ->
              required =
                String.contains?(tail, "NOT NULL") or (backend == :postgres and name in primary)

              [{name, %{type: type, required: required, primary: name in primary}}]

            _ ->
              []
          end
        end)
        |> Map.new()

      {table, columns}
    end)
  end

  def tables!(connection, :sqlite) do
    MigrationQuery.rows!(
      connection,
      "SELECT name FROM sqlite_schema WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )
    |> List.flatten()
  end

  def tables!(connection, :postgres) do
    MigrationQuery.rows!(
      connection,
      "SELECT table_name FROM information_schema.tables WHERE table_schema = current_schema() ORDER BY table_name"
    )
    |> List.flatten()
  end

  def columns!(connection, :sqlite, table) do
    MigrationQuery.rows!(connection, "PRAGMA table_xinfo(#{table})")
    |> Map.new(fn [_id, name, type, required, _default, primary, hidden] ->
      if hidden != 0, do: raise("generated columns are not supported by the migration baseline")
      {name, %{type: String.upcase(type), required: required == 1, primary: primary > 0}}
    end)
  end

  def columns!(connection, :postgres, table) do
    MigrationQuery.rows!(
      connection,
      """
      SELECT c.column_name, c.data_type, c.is_nullable,
        EXISTS (SELECT 1 FROM information_schema.table_constraints t
          JOIN information_schema.key_column_usage k USING (constraint_catalog, constraint_schema, constraint_name)
          WHERE t.constraint_type = 'PRIMARY KEY' AND t.table_schema = c.table_schema
            AND t.table_name = c.table_name AND k.column_name = c.column_name)
      FROM information_schema.columns c WHERE c.table_schema = current_schema() AND c.table_name = $1
      """,
      [table]
    )
    |> Map.new(fn [name, type, nullable, primary] ->
      type =
        Map.get(
          %{"timestamp with time zone" => "TIMESTAMPTZ", "jsonb" => "JSONB"},
          type,
          String.upcase(type)
        )

      {name, %{type: type, required: nullable == "NO", primary: primary}}
    end)
  end

  def validate!(connection, backend, mode, version \\ 2) do
    tables = tables!(connection, backend)
    contracts = contracts(backend, version)

    Enum.each(tables, fn table ->
      if (String.starts_with?(table, "kyuubiki_") or String.starts_with?(table, "central_")) and
           not Map.has_key?(contracts, table) and table != "kyuubiki_data_migrations" do
        raise "unknown managed table #{table}; refusing schema downgrade"
      end
    end)

    Enum.each(contracts, fn {table, expected} ->
      if table in tables do
        actual = columns!(connection, backend, table)

        allowed_missing =
          if mode == :legacy and table == "kyuubiki_jobs", do: @legacy_columns, else: []

        Enum.each(expected, fn {name, spec} ->
          case Map.fetch(actual, name) do
            {:ok, ^spec} ->
              :ok

            :error ->
              unless name in allowed_missing,
                do: raise("missing database column #{table}.#{name}")

            _ ->
              raise "incompatible database column #{table}.#{name}; explicit conversion required"
          end
        end)

        if Map.keys(actual) -- Map.keys(expected) != [], do: raise("unknown columns in #{table}")
        validate_foreign_keys!(connection, backend, table)
      else
        if mode == :current,
          do: raise("missing managed table #{table}; refusing silent recreation")
      end
    end)
  end

  def legacy_additions(connection, backend) do
    if "kyuubiki_jobs" in tables!(connection, backend) do
      existing = columns!(connection, backend, "kyuubiki_jobs")
      specs = contracts(backend)["kyuubiki_jobs"]

      for name <- @legacy_columns,
          not Map.has_key?(existing, name),
          do: "ALTER TABLE kyuubiki_jobs ADD COLUMN #{name} #{specs[name].type}"
    else
      []
    end
  end

  defp validate_foreign_keys!(connection, backend, table) do
    sql = Enum.find(statements(backend, 2), &String.contains?(&1, "EXISTS #{table} ("))

    expected =
      Regex.scan(~r/^\s+(\w+) [^\n]*REFERENCES (\w+)\((\w+)\) ON DELETE (CASCADE|SET NULL)/m, sql)
      |> Enum.map(fn [_, column, target, key, delete] -> [column, target, key, delete] end)
      |> MapSet.new()

    actual = foreign_keys!(connection, backend, table) |> MapSet.new()
    # Older PostgreSQL startup added this optional relationship only when the column was absent.
    actual =
      if table == "kyuubiki_jobs",
        do:
          MapSet.delete(actual, [
            "model_version_id",
            "kyuubiki_model_versions",
            "version_id",
            "SET NULL"
          ]),
        else: actual

    if actual != expected, do: raise("incompatible foreign-key contract in #{table}")
  end

  defp foreign_keys!(connection, :sqlite, table) do
    MigrationQuery.rows!(connection, "PRAGMA foreign_key_list(#{table})")
    |> Enum.map(fn [_id, sequence, target, column, key, _update, delete, _match] ->
      if sequence != 0, do: raise("unsupported composite foreign key in #{table}")
      [column, target, key, delete]
    end)
  end

  defp foreign_keys!(connection, :postgres, table) do
    MigrationQuery.rows!(
      connection,
      """
      SELECT a.attname, target.relname, b.attname, c.confdeltype::text,
             tn.nspname = current_schema(), array_length(c.conkey, 1)
      FROM pg_constraint c
      JOIN pg_class r ON r.oid = c.conrelid
      JOIN pg_namespace n ON n.oid = r.relnamespace
      JOIN pg_class target ON target.oid = c.confrelid
      JOIN pg_namespace tn ON tn.oid = target.relnamespace
      JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = c.conkey[1]
      JOIN pg_attribute b ON b.attrelid = c.confrelid AND b.attnum = c.confkey[1]
      WHERE c.contype = 'f' AND n.nspname = current_schema() AND r.relname = $1
      """,
      [table]
    )
    |> Enum.map(fn [column, target, key, delete, local, count] ->
      if not local or count != 1, do: raise("unsupported foreign key in #{table}")
      [column, target, key, Map.get(%{"c" => "CASCADE", "n" => "SET NULL"}, delete, delete)]
    end)
  end

  defp primary_keys(sql) do
    case Regex.run(~r/PRIMARY KEY \(([^)]+)\)/, sql) do
      [_, keys] -> keys |> String.split(",") |> Enum.map(&String.trim/1)
      nil -> Regex.scan(~r/(\w+) \w+ PRIMARY KEY/, sql) |> Enum.map(&Enum.at(&1, 1))
    end
  end

  defp statements(backend, version) do
    RuntimeSchema.statements(backend) ++
      if(version >= 2, do: CheckpointSchema.statements(backend), else: [])
  end
end
