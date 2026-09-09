defmodule KyuubikiWeb.Storage.SqliteLifecycleTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Storage.{DatabaseMigrations, MigrationQuery, SqliteLifecycle}
  alias KyuubikiWeb.Storage.SqliteLifecycleFiles, as: Files
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture

  setup do
    dir = Fixture.directory()
    source = Path.join(dir, "source.sqlite3")
    db = Fixture.open!(source)
    Fixture.legacy!(db, :sqlite, old_jobs: true)
    Fixture.close!(db)
    %{dir: dir, source: source, digest: Files.digest!(source)}
  end

  test "copy upgrade and relocated verification preserve the source and scientific payload",
       ctx do
    output = Path.join(ctx.dir, "upgraded.sqlite3")
    receipt = SqliteLifecycle.upgrade!(ctx.source, output, ctx.digest)
    assert Files.digest!(ctx.source) == ctx.digest
    assert receipt["source_data_version"] == 0
    assert receipt["database"]["data_version"] == 1
    assert receipt["source_retained"]
    assert is_boolean(receipt["directory_synced"])
    moved = Path.join(ctx.dir, "relocated.sqlite3")
    File.rename!(output, moved)
    assert SqliteLifecycle.verify!(moved, receipt)["status"] == "verified"
    db = Fixture.open!(moved)

    assert [[~s({"temperature":[273.15,320.5],"winner":"copper"})]] ==
             MigrationQuery.rows!(db, "SELECT payload FROM kyuubiki_analysis_results")

    Fixture.close!(db)
    assert Path.wildcard(Path.join(ctx.dir, ".kyuubiki-data-*")) == []
  end

  test "consistent backup includes committed WAL that main-file copy does not contain", ctx do
    db = Fixture.open!(ctx.source)

    try do
      MigrationQuery.rows!(db, "PRAGMA journal_mode=WAL")
      MigrationQuery.rows!(db, "PRAGMA wal_autocheckpoint=0")

      MigrationQuery.rows!(
        db,
        "INSERT INTO kyuubiki_projects(project_id, name) VALUES ('wal', 'committed in WAL')"
      )

      assert File.stat!(ctx.source <> "-wal").size > 0
      assert_raise RuntimeError, ~r/WAL/, fn -> SqliteLifecycle.plan!(ctx.source) end
      output = Path.join(ctx.dir, "backup.sqlite3")
      receipt = SqliteLifecycle.backup!(ctx.source, output)
      assert receipt["operation"] == "backup"
      assert SqliteLifecycle.verify!(output, receipt)["status"] == "verified"
      snapshot = Fixture.open!(output)

      assert [["committed in WAL"]] ==
               MigrationQuery.rows!(
                 snapshot,
                 "SELECT name FROM kyuubiki_projects WHERE project_id='wal'"
               )

      Fixture.close!(snapshot)
    after
      Fixture.close!(db)
    end
  end

  test "restore uses a separate path and retains exact snapshot bytes", ctx do
    output = Path.join(ctx.dir, "restore.sqlite3")
    receipt = SqliteLifecycle.restore!(ctx.source, output, ctx.digest)
    assert Files.digest!(output) == ctx.digest
    assert receipt["database"]["status"] == "legacy_unversioned"

    assert_raise RuntimeError, ~r/overwrite/, fn ->
      SqliteLifecycle.restore!(ctx.source, output, ctx.digest)
    end

    assert Files.digest!(output) == ctx.digest
  end

  test "stale approval and in-place upgrade are refused", ctx do
    output = Path.join(ctx.dir, "out.sqlite3")

    assert_raise RuntimeError, ~r/digest/, fn ->
      SqliteLifecycle.upgrade!(ctx.source, output, String.duplicate("0", 64))
    end

    assert_raise RuntimeError, ~r/overwrite/, fn ->
      SqliteLifecycle.upgrade!(ctx.source, ctx.source, ctx.digest)
    end

    refute File.exists?(output)
    assert Files.digest!(ctx.source) == ctx.digest
  end

  test "malformed source fails without publishing a target or leaving a stage", ctx do
    File.write!(ctx.source, "not a database")
    output = Path.join(ctx.dir, "out.sqlite3")

    assert_raise MatchError, fn ->
      SqliteLifecycle.upgrade!(ctx.source, output, Files.digest!(ctx.source))
    end

    refute File.exists?(output)
    assert Path.wildcard(Path.join(ctx.dir, ".kyuubiki-data-*")) == []
  end

  test "broken references are not certified as a restorable research backup", ctx do
    db = Fixture.open!(ctx.source)

    MigrationQuery.rows!(
      db,
      "INSERT INTO kyuubiki_analysis_results(job_id, payload) VALUES ('missing', '{}')"
    )

    Fixture.close!(db)
    output = Path.join(ctx.dir, "backup.sqlite3")

    assert_raise RuntimeError, ~r/foreign-key/, fn ->
      SqliteLifecycle.backup!(ctx.source, output)
    end

    refute File.exists?(output)
  end

  test "future ledger cannot be upgraded or silently restored with an older reader", ctx do
    db = Fixture.open!(ctx.source)
    DatabaseMigrations.upgrade_candidate!(db, :sqlite)
    MigrationQuery.rows!(db, "UPDATE kyuubiki_data_migrations SET version=10")
    Fixture.close!(db)

    assert_raise RuntimeError, ~r/history/, fn ->
      SqliteLifecycle.restore!(
        ctx.source,
        Path.join(ctx.dir, "new.sqlite3"),
        Files.digest!(ctx.source)
      )
    end
  end

  test "existing destination sidecars and concurrent publications are never reused", ctx do
    output = Path.join(ctx.dir, "target.sqlite3")
    File.write!(output <> "-wal", "existing")

    assert_raise RuntimeError, ~r/overwrite/, fn ->
      SqliteLifecycle.backup!(ctx.source, output)
    end

    File.rm!(output <> "-wal")

    assert_raise RuntimeError, ~r/overwrite/, fn ->
      Files.publish!(output, fn candidate ->
        File.cp!(ctx.source, candidate)
        File.write!(output, "another writer")
        %{}
      end)
    end

    assert File.read!(output) == "another writer"
  end

  test "symlink sources, dangling destinations and symlink sidecars fail closed", ctx do
    if match?({:unix, _}, :os.type()) do
      link = Path.join(ctx.dir, "link.sqlite3")
      File.ln_s!(ctx.source, link)
      assert_raise RuntimeError, ~r/regular/, fn -> SqliteLifecycle.plan!(link) end
      output = Path.join(ctx.dir, "dangling.sqlite3")
      File.ln_s!(Path.join(ctx.dir, "missing"), output)

      assert_raise RuntimeError, ~r/overwrite/, fn ->
        SqliteLifecycle.backup!(ctx.source, output)
      end

      File.ln_s!(ctx.source, ctx.source <> "-wal")

      assert_raise RuntimeError, ~r/sidecar/, fn ->
        SqliteLifecycle.backup!(ctx.source, Path.join(ctx.dir, "backup.sqlite3"))
      end
    end
  end

  test "receipt tampering and changed database content are detected", ctx do
    output = Path.join(ctx.dir, "current.sqlite3")
    receipt = SqliteLifecycle.upgrade!(ctx.source, output, ctx.digest)

    assert_raise RuntimeError, ~r/receipt/, fn ->
      SqliteLifecycle.verify!(output, Map.put(receipt, "database", %{}))
    end

    db = Fixture.open!(output)
    MigrationQuery.rows!(db, "UPDATE kyuubiki_projects SET name='changed'")
    Fixture.close!(db)
    assert_raise RuntimeError, ~r/digest/, fn -> SqliteLifecycle.verify!(output, receipt) end
  end

  test "current copy upgrade does not increment the data version or duplicate ledger rows", ctx do
    output = Path.join(ctx.dir, "current.sqlite3")
    first = SqliteLifecycle.upgrade!(ctx.source, output, ctx.digest)

    second =
      SqliteLifecycle.upgrade!(output, Path.join(ctx.dir, "next.sqlite3"), first["output_sha256"])

    assert second["database"]["data_version"] == 1
    assert second["output_sha256"] == first["output_sha256"]
  end

  test "incomplete receipts and false byte counts cannot verify", ctx do
    output = Path.join(ctx.dir, "current.sqlite3")
    receipt = SqliteLifecycle.upgrade!(ctx.source, output, ctx.digest)

    assert_raise RuntimeError, ~r/receipt/, fn ->
      SqliteLifecycle.verify!(output, Map.delete(receipt, "source_retained"))
    end

    assert_raise RuntimeError, ~r/byte count/, fn ->
      SqliteLifecycle.verify!(output, Map.put(receipt, "output_bytes", 1))
    end

    if match?({:unix, _}, :os.type()),
      do: assert(Bitwise.band(File.stat!(output).mode, 0o777) == 0o600)
  end

  test "successful publication retains its receipt when staging cleanup needs attention", ctx do
    if match?({:unix, _}, :os.type()) do
      output = Path.join(ctx.dir, "published.sqlite3")

      receipt =
        Files.publish!(output, fn candidate ->
          File.cp!(ctx.source, candidate)
          File.chmod!(Path.dirname(candidate), 0o500)
          %{"operation" => "restore"}
        end)

      try do
        assert receipt["output_sha256"] == ctx.digest
        assert receipt["cleanup_pending"] =~ ".kyuubiki-data-"
        assert File.exists?(output)
      after
        if receipt["cleanup_pending"] do
          stage = Path.join(ctx.dir, receipt["cleanup_pending"])
          File.chmod!(stage, 0o700)
          File.rm_rf!(stage)
        end
      end
    end
  end
end
