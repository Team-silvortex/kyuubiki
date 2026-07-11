defmodule KyuubikiSdk.MaterialResearchBundleTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.MaterialResearchBundle

  @fixture_path Path.expand("../../../schemas/examples.material-research-bundle.json", __DIR__)

  test "validates shared bundle fixture" do
    assert {:ok, bundle} =
             @fixture_path
             |> File.read!()
             |> Jason.decode!()
             |> MaterialResearchBundle.validate()

    assert bundle["schema_version"] == MaterialResearchBundle.schema_version()
    assert bundle["study"] == "heat-spreader"
    assert bundle["summary"]["winner_candidate_id"] == "pyrolytic_graphite_in_plane"
  end

  test "rejects bad retained artifact schema" do
    bundle =
      @fixture_path
      |> File.read!()
      |> Jason.decode!()
      |> put_in(["chain", "schema_version"], "wrong")

    assert {:error, error} = MaterialResearchBundle.validate(bundle)
    assert error.message =~ "chain.schema_version"
  end

  test "rejects bad checksum shape" do
    bundle =
      @fixture_path
      |> File.read!()
      |> Jason.decode!()
      |> put_in(["artifact_checksums", "chain_sha256"], "not-a-digest")

    assert {:error, error} = MaterialResearchBundle.validate(bundle)
    assert error.message =~ "chain_sha256"
  end

  test "rejects summary and plan decision mismatch" do
    bundle =
      @fixture_path
      |> File.read!()
      |> Jason.decode!()
      |> put_in(["next_round_execution_plan", "decision"], "repair_validation")

    assert {:error, error} = MaterialResearchBundle.validate(bundle)
    assert error.message =~ "next_round_execution_plan.decision"
  end
end
