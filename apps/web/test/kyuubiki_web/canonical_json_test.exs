defmodule KyuubikiWeb.CanonicalJsonTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.CanonicalJson

  test "shared TaskIR float fixture uses exact fixed-15 ties-to-even rounding" do
    path = Path.expand("../../../../schemas/examples.task-ir-canonical-floats.json", __DIR__)

    for example <- path |> File.read!() |> Jason.decode!() |> Map.fetch!("cases") do
      assert CanonicalJson.encode!(example["value"]) == example["canonical"]

      assert CanonicalJson.encode!(example["value"] |> Jason.encode!() |> Jason.decode!()) ==
               example["canonical"]
    end
  end

  test "encodes object keys in lexicographic order recursively" do
    assert CanonicalJson.encode!(%{
             "z" => 1,
             "a" => %{"b" => true, "a" => nil},
             "list" => [%{"y" => 2, "x" => 1}]
           }) == ~s({"a":{"a":null,"b":true},"list":[{"x":1,"y":2}],"z":1})
  end

  test "encodes floats without exponent notation" do
    assert CanonicalJson.encode!(%{
             "a" => 160.0,
             "b" => 1.2e-5,
             "c" => 7.0e10,
             "d" => 0.33
           }) == ~s({"a":160.0,"b":0.000012,"c":70000000000.0,"d":0.33})
  end
end
