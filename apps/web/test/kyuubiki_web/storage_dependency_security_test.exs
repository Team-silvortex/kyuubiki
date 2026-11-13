defmodule KyuubikiWeb.StorageDependencySecurityTest do
  use ExUnit.Case, async: true

  test "Ecto constraint and loaded lock exclude unbounded dynamic query cache versions" do
    {:hex, :ecto, version, _, _, _, _, _} = Mix.Dep.Lock.read()[:ecto]
    assert Version.match?(version, ">= 3.14.3")

    requirement = Keyword.fetch!(Mix.Project.config()[:deps], :ecto)
    assert Version.match?("3.14.3", requirement)
    refute Version.match?("3.14.2", requirement)
    refute Version.match?("3.14.1", requirement)
    assert to_string(Application.spec(:ecto, :vsn)) == version
  end
end
