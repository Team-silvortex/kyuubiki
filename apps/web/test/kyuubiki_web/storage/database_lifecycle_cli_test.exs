defmodule KyuubikiWeb.Storage.DatabaseLifecycleCliTest do
  use ExUnit.Case, async: false
  import ExUnit.CaptureIO
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture
  alias KyuubikiWeb.Storage.SqliteLifecycleFiles, as: Files

  test "maintenance command chain snapshots upgrades verifies and restores without changing app backend" do
    dir = Fixture.directory()
    source = Path.join(dir, "source.sqlite3")
    db = Fixture.open!(source)
    Fixture.legacy!(db)
    Fixture.close!(db)
    backend = KyuubikiWeb.Storage.backend()
    backup = Path.join(dir, "backup.sqlite3")
    snapshot = run(["backup", source, "--out", backup])
    plan = run(["plan", backup])
    assert plan["source_sha256"] == snapshot["output_sha256"]
    upgrade = Path.join(dir, "upgraded.sqlite3")
    receipt = run(["upgrade", backup, "--out", upgrade, "--expect-sha256", plan["source_sha256"]])
    receipt_file = Path.join(dir, "receipt.json")
    File.write!(receipt_file, Jason.encode!(receipt))
    assert run(["verify", upgrade, "--receipt", receipt_file])["status"] == "verified"
    restored = Path.join(dir, "restored.sqlite3")
    run(["restore", backup, "--out", restored, "--expect-sha256", plan["source_sha256"]])
    assert Files.digest!(restored) == plan["source_sha256"]
    assert KyuubikiWeb.Storage.backend() == backend
  end

  test "command parser rejects wrong irrelevant repeated and missing approval options before mutation" do
    for args <- [
          ["plan", "missing", "--out", "should-not-exist"],
          ["backup", "missing", "--out", "one", "--out", "two"],
          ["upgrade", "missing", "--out", "new"],
          ["restore", "missing", "--out", "new", "--unknown"],
          ["verify", "missing"]
        ] do
      assert_raise Mix.Error, ~r/usage:/, fn -> run(args) end
    end
  end

  defp run(args) do
    capture_io(fn -> Mix.Tasks.Kyuubiki.Data.run(args) end) |> Jason.decode!()
  end
end
