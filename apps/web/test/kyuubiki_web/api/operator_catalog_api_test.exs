defmodule KyuubikiWeb.Api.OperatorCatalogApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "lists built-in operators and fetches a descriptor" do
    list_conn =
      :get
      |> conn("/api/v1/operators")
      |> Router.call(@opts)

    assert list_conn.status == 200
    payload = Jason.decode!(list_conn.resp_body)
    operators = payload["operators"]
    assert length(operators) >= 8

    frame_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.frame_3d" end)

    electrostatic_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.electrostatic_bar_1d" end)

    electrostatic_heat_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.electrostatic_field_to_heat_quad_2d"
      end)

    heat_thermo_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.temperature_field_to_thermo_quad_2d"
      end)

    assert frame_operator["kind"] == "solver"
    assert frame_operator["origin"] == "built_in"
    assert frame_operator["input_schema"]["schema"] == "kyuubiki.operator.frame_3d.input"
    assert frame_operator["validation"]["baseline_status"] == "verified"
    assert frame_operator["validation"]["baseline_cases"] == ["frame_3d_baseline"]
    assert "orchestrated_api" in frame_operator["validation"]["smoke_paths"]

    assert [
             %{
               "id" => "model",
               "artifact_type" => "model/frame_3d",
               "dataset_value" => "model"
             }
           ] = frame_operator["inputs"]

    assert [
             %{
               "id" => "result",
               "artifact_type" => "result/frame_3d",
               "dataset_value" => "result"
             }
           ] = frame_operator["outputs"]

    assert electrostatic_operator["kind"] == "solver"
    assert electrostatic_operator["family"] == "electrostatic_bar_1d"
    assert electrostatic_operator["domain"] == "electromagnetic"
    assert electrostatic_operator["validation"]["baseline_status"] == "verified"
    assert "electrostatic" in electrostatic_operator["capability_tags"]

    assert [
             %{
               "id" => "model",
               "artifact_type" => "model/electrostatic_bar_1d",
               "dataset_value" => "model"
             }
           ] = electrostatic_operator["inputs"]

    assert [
             %{
               "id" => "result",
               "artifact_type" => "result/electrostatic_bar_1d",
               "dataset_value" => "result"
             }
           ] = electrostatic_operator["outputs"]

    assert electrostatic_heat_bridge_operator["kind"] == "workflow_bridge"

    assert electrostatic_heat_bridge_operator["config_schema"]["schema"] ==
             "kyuubiki.bridge-contract.electrostatic_to_heat.v1"

    assert electrostatic_heat_bridge_operator["config_example"]["contract"]["source"]["field"] ==
             "electric_field_magnitude"

    assert electrostatic_heat_bridge_operator["config_example"]["contract"]["target"]["field"] ==
             "heat_load"

    assert heat_thermo_bridge_operator["config_schema"]["schema"] ==
             "kyuubiki.bridge-contract.heat_to_thermo.v1"

    assert heat_thermo_bridge_operator["config_example"]["contract"]["target"]["field"] ==
             "temperature_delta"

    electrostatic_plane_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "solve.electrostatic_plane_triangle_2d"
      end)

    electrostatic_plane_quad_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "solve.electrostatic_plane_quad_2d"
      end)

    assert electrostatic_plane_operator["kind"] == "solver"
    assert electrostatic_plane_operator["family"] == "electrostatic_plane_triangle_2d"
    assert electrostatic_plane_operator["domain"] == "electromagnetic"
    assert "triangle" in electrostatic_plane_operator["capability_tags"]
    assert electrostatic_plane_quad_operator["kind"] == "solver"
    assert electrostatic_plane_quad_operator["family"] == "electrostatic_plane_quad_2d"
    assert electrostatic_plane_quad_operator["domain"] == "electromagnetic"
    assert "quad" in electrostatic_plane_quad_operator["capability_tags"]

    fetch_conn =
      :get
      |> conn("/api/v1/operators/export.summary_csv")
      |> Router.call(@opts)

    assert fetch_conn.status == 200
    fetched = Jason.decode!(fetch_conn.resp_body)["operator"]
    assert fetched["id"] == "export.summary_csv"
    assert fetched["kind"] == "export"
    assert fetched["outputs"] |> hd() |> Map.fetch!("artifact_type") == "export/summary_csv"
  end

  test "filters built-in operators by query contract fields" do
    conn =
      :get
      |> conn(
        "/api/v1/operators?q=heat&domain=thermal&kind=solver&validation=verified&capability=heat"
      )
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    operators = payload["operators"]
    assert Enum.any?(operators, &(&1["id"] == "solve.heat_plane_quad_2d"))
    assert Enum.all?(operators, &(&1["domain"] == "thermal"))
    assert Enum.all?(operators, &(&1["kind"] == "solver"))
    assert Enum.all?(operators, &(&1["validation"]["baseline_status"] == "verified"))
    assert Enum.all?(operators, &("heat" in (&1["capability_tags"] || [])))
  end

  test "returns 404 for an unknown operator descriptor" do
    conn =
      :get
      |> conn("/api/v1/operators/solve.missing_operator")
      |> Router.call(@opts)

    assert conn.status == 404
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] == "operator_not_found"
    assert payload["operator_id"] == "solve.missing_operator"
  end
end
