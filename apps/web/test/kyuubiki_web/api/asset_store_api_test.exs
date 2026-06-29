defmodule KyuubikiWeb.Api.AssetStoreApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.AssetStore

  setup do
    original_config = Application.get_env(:kyuubiki_web, AssetStore, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AssetStore, original_config)
    end)

    :ok
  end

  test "lists operators workflow templates and frontend DSL templates from the built-in source" do
    conn =
      :get
      |> conn("/api/v1/store")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    entries = payload["entries"]
    source_ids = Enum.map(payload["sources"], & &1["id"])

    assert "builtin.local" in source_ids
    assert Enum.any?(entries, &(&1["kind"] == "operator" and &1["id"] == "solve.frame_3d"))

    assert Enum.any?(
             entries,
             &(&1["kind"] == "workflow_template" and
                 &1["id"] == "workflow.heat-to-thermo-quad-2d")
           )

    assert Enum.any?(
             entries,
             &(&1["kind"] == "frontend_dsl_template" and
                 &1["id"] == "frontend.dsl.layout_report")
           )

    assert payload["summary"]["kinds"]["operator"] > 0
    assert payload["summary"]["sources"]["builtin.local"] == length(entries)
  end

  test "filters store entries and fetches a single entry" do
    list_conn =
      :get
      |> conn("/api/v1/store?kind=frontend_dsl_template&q=layout")
      |> Router.call(@opts)

    assert list_conn.status == 200
    entries = Jason.decode!(list_conn.resp_body)["entries"]
    assert Enum.map(entries, & &1["id"]) == ["frontend.dsl.layout_report"]

    fetch_conn =
      :get
      |> conn("/api/v1/store/frontend_dsl_template/frontend.dsl.layout_report")
      |> Router.call(@opts)

    assert fetch_conn.status == 200

    assert Jason.decode!(fetch_conn.resp_body)["entry"]["payload"]["dsl_version"] ==
             "kyuubiki.frontend-dsl/v1"
  end

  test "loads configured local catalog file sources" do
    path = Path.join(System.tmp_dir!(), "kyuubiki-store-source-#{System.unique_integer()}.json")
    on_exit(fn -> File.rm(path) end)

    File.write!(
      path,
      Jason.encode!(%{
        "entries" => [
          %{
            "id" => "operator.external.demo",
            "kind" => "operator",
            "title" => "External demo operator",
            "summary" => "Demo operator from a configured local catalog source.",
            "version" => "0.1.0",
            "tags" => ["external", "demo"]
          }
        ]
      })
    )

    Application.put_env(:kyuubiki_web, AssetStore,
      sources: [
        %{
          id: "local.demo",
          type: "catalog_file",
          label: "Local demo source",
          path: path
        }
      ]
    )

    conn =
      :get
      |> conn("/api/v1/store?source_id=local.demo")
      |> Router.call(@opts)

    assert conn.status == 200
    [entry] = Jason.decode!(conn.resp_body)["entries"]
    assert entry["id"] == "operator.external.demo"
    assert entry["source_id"] == "local.demo"
    assert entry["source_kind"] == "catalog_file"
  end
end
