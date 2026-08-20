defmodule KyuubikiWeb.PersistenceTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Persistence

  setup do
    data_dir =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-persistence-test-#{System.unique_integer([:positive])}"
      )

    System.put_env("KYUUBIKI_DATA_DIR", data_dir)

    on_exit(fn ->
      Persistence.clear!()
      System.delete_env("KYUUBIKI_DATA_DIR")
    end)

    :ok
  end

  test "writes and reloads a digest-verified persistence envelope" do
    path = Persistence.jobs_path()
    payload = %{"job-1" => %{"status" => "queued", "revision" => 1}}

    Persistence.write_json!(path, payload)

    envelope = path |> File.read!() |> Jason.decode!()
    assert envelope["schema_version"] == "kyuubiki.persistence-envelope/v1"
    assert envelope["digest_algorithm"] == "sha256"
    assert envelope["payload_sha256"] =~ ~r/\A[0-9a-f]{64}\z/
    assert Persistence.read_json(path, %{}) == payload
  end

  test "recovers the previous verified generation after tamper" do
    path = Persistence.jobs_path()
    first = %{"job-1" => %{"status" => "queued"}}
    second = %{"job-1" => %{"status" => "completed"}}
    Persistence.write_json!(path, first)
    Persistence.write_json!(path, second)

    tampered =
      path
      |> File.read!()
      |> Jason.decode!()
      |> put_in(["payload", "job-1", "status"], "tampered")

    File.write!(path, Jason.encode!(tampered))

    assert Persistence.read_json(path, %{}) == first
    receipt = "#{path}.recovery.json" |> File.read!() |> Jason.decode!()
    assert receipt["status"] == "recovered_previous_generation"
    assert receipt["previous_generation_used"]
    assert File.exists?("#{path}.corrupt")
    assert Persistence.read_json(path, %{}) == first
  end

  test "recovers when a commit loses the primary generation" do
    path = Persistence.jobs_path()
    payload = %{"job-1" => %{"status" => "queued"}}
    Persistence.write_json!(path, payload)
    File.rename!(path, "#{path}.previous")

    assert Persistence.read_json(path, %{}) == payload
    receipt = "#{path}.recovery.json" |> File.read!() |> Jason.decode!()
    assert receipt["status"] == "recovered_previous_generation"
    assert receipt["primary_error"] =~ "primary_generation_missing"
    assert File.exists?(path)
  end

  test "quarantines an unrecoverable tampered generation without cascading failure" do
    path = Persistence.results_path()
    Persistence.write_json!(path, %{"job-1" => %{"value" => 1}})

    envelope =
      path
      |> File.read!()
      |> Jason.decode!()
      |> Map.put("payload_sha256", String.duplicate("0", 64))

    File.write!(path, Jason.encode!(envelope))

    assert Persistence.read_json(path, %{"safe" => true}) == %{"safe" => true}
    receipt = "#{path}.recovery.json" |> File.read!() |> Jason.decode!()
    assert receipt["status"] == "quarantined_and_defaulted"
    refute receipt["previous_generation_used"]
    assert receipt["corrupt_copy_retained"]
    assert File.exists?("#{path}.corrupt")
  end
end
