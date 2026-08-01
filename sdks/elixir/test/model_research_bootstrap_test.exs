defmodule KyuubikiSdk.ModelResearchBootstrapTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.ModelResearchBootstrap

  setup do
    root = Path.expand("../../..", __DIR__)

    bootstrap =
      root
      |> Path.join("docs/model-research-bootstrap.json")
      |> File.read!()
      |> Jason.decode!()

    %{root: root, bootstrap: bootstrap}
  end

  test "repository bootstrap is ready for all official SDKs", context do
    for sdk <- ~w(rust python elixir) do
      assert {:ok, report} =
               ModelResearchBootstrap.inspect(
                 context.bootstrap,
                 sdk,
                 &File.regular?(Path.join(context.root, &1))
               )

      assert report["ready_for_planning"], inspect(report["blockers"])
      assert report["execution_authority"] == "none_preflight_only"
      assert report["missing_resources"] == []
      assert is_map(report["selected_surface"])
    end
  end

  test "Elixir report exposes native preflight entrypoint", context do
    assert {:ok, report} =
             ModelResearchBootstrap.inspect(
               context.bootstrap,
               :elixir,
               &File.regular?(Path.join(context.root, &1))
             )

    assert report["selected_surface"]["preflight_path"] ==
             "sdks/elixir/lib/kyuubiki_sdk/model_research_bootstrap.ex"

    assert report["selected_surface"]["inspect"] ==
             "KyuubikiSdk.ModelResearchBootstrap.inspect/3"
  end

  test "missing and unsafe resources fail closed", context do
    assert {:ok, missing} =
             ModelResearchBootstrap.inspect(context.bootstrap, :elixir, fn path ->
               path != "llms.txt" and File.regular?(Path.join(context.root, path))
             end)

    refute missing["ready_for_planning"]
    assert missing["missing_resources"] == ["llms.txt"]

    unsafe = put_in(context.bootstrap, ["required_documents", Access.at(0), "path"], "../secret")

    assert {:ok, report} =
             ModelResearchBootstrap.inspect(
               unsafe,
               :elixir,
               &File.regular?(Path.join(context.root, &1))
             )

    refute report["ready_for_planning"]
    assert Enum.any?(report["blockers"], &String.contains?(&1, "safe project-relative path"))

    authority = put_in(context.bootstrap, ["preflight", "execution_authority"], "model_owned")

    assert {:ok, authority_report} =
             ModelResearchBootstrap.inspect(
               authority,
               :elixir,
               &File.regular?(Path.join(context.root, &1))
             )

    refute authority_report["ready_for_planning"]
    assert Enum.any?(authority_report["blockers"], &String.contains?(&1, "none_preflight_only"))
  end

  test "malformed nested surfaces return blockers instead of raising", context do
    malformed = Map.put(context.bootstrap, "sdk_surfaces", "not-an-object")

    assert {:ok, report} =
             ModelResearchBootstrap.inspect(
               malformed,
               :elixir,
               &File.regular?(Path.join(context.root, &1))
             )

    refute report["ready_for_planning"]
    assert "selected SDK surface is missing: elixir" in report["blockers"]
  end
end
