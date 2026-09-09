defmodule KyuubikiWeb.PersistenceTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Persistence

  setup do
    original_data_dir = System.get_env("KYUUBIKI_DATA_DIR")

    data_dir =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-persistence-test-#{System.unique_integer([:positive])}"
      )

    System.put_env("KYUUBIKI_DATA_DIR", data_dir)

    on_exit(fn ->
      File.rm_rf!(data_dir)

      if original_data_dir,
        do: System.put_env("KYUUBIKI_DATA_DIR", original_data_dir),
        else: System.delete_env("KYUUBIKI_DATA_DIR")
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

  test "JSON object keys remain digest-stable across a durable round trip" do
    path = Persistence.results_path()

    payload = %{
      "job-1" => %{
        "bundle_domain_counts" => %{nil => 2, "" => 1},
        "nested" => [%{true => 1, false => 2, :queued => 3, 7 => 4}],
        "values" => [nil, true, false, -0.0, 1.0e-200, 1.0e200]
      }
    }

    expected = payload |> Jason.encode!() |> Jason.decode!()
    Persistence.write_json!(path, payload)
    assert Persistence.read_json(path, %{}) === expected
    refute File.exists?("#{path}.corrupt")
    refute File.exists?("#{path}.recovery.json")
    assert :ok = Persistence.write_json!(path, %{"next" => true})
    assert Persistence.read_json("#{path}.previous", %{}) === expected
    assert Persistence.read_json(path, %{}) == %{"next" => true}
  end

  test "ambiguous JSON key aliases cannot replace the current generation" do
    path = Persistence.results_path()
    Persistence.write_json!(path, %{"valid" => true})
    before = File.read!(path)

    for aliases <- [%{nil => 1, "nil" => 2}, %{:queued => 1, "queued" => 2}, %{7 => 1, "7" => 2}] do
      assert_raise Jason.EncodeError, ~r/duplicate key/, fn ->
        Persistence.write_json!(path, %{"nested" => [aliases]})
      end

      assert File.read!(path) == before
      refute File.exists?("#{path}.previous")
      refute File.exists?("#{path}.next")
    end
  end

  test "previous string-key envelopes stay readable without rewriting" do
    path = Persistence.jobs_path()
    payload = %{"job-1" => %{"revision" => 1, "status" => "queued"}}

    digest =
      :crypto.hash(:sha256, ~s({"job-1":{"revision":1,"status":"queued"}}))
      |> Base.encode16(case: :lower)

    Persistence.ensure_dir!()

    bytes =
      Jason.encode!(%{
        "schema_version" => "kyuubiki.persistence-envelope/v1",
        "digest_algorithm" => "sha256",
        "payload_sha256" => digest,
        "payload" => payload
      })

    File.write!(path, bytes)
    assert Persistence.read_json(path, %{}) == payload
    assert File.read!(path) == bytes
  end

  test "old inconsistent nil-key digests are not silently certified" do
    path = Persistence.results_path()
    Persistence.ensure_dir!()

    old_digest =
      :crypto.hash(:sha256, ~s({"bundle_domain_counts":{"":2}}))
      |> Base.encode16(case: :lower)

    File.write!(
      path,
      Jason.encode!(%{
        "schema_version" => "kyuubiki.persistence-envelope/v1",
        "digest_algorithm" => "sha256",
        "payload_sha256" => old_digest,
        "payload" => %{"bundle_domain_counts" => %{"nil" => 2}}
      })
    )

    assert Persistence.read_json(path, %{"unverified" => true}) == %{"unverified" => true}
    assert File.exists?("#{path}.corrupt")
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
