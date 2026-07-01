defmodule KyuubikiWeb.CanonicalJsonTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.CanonicalJson

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
