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
    modules = payload["modules"]
    assert length(operators) >= 8
    assert Enum.any?(modules, &(&1["id"] == "mechanical.solver"))

    frame_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.frame_3d" end)

    electrostatic_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.electrostatic_bar_1d" end)

    acoustic_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.acoustic_bar_1d" end)

    electrostatic_heat_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.electrostatic_field_to_heat_quad_2d"
      end)

    heat_thermo_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.temperature_field_to_thermo_quad_2d"
      end)

    heat_thermo_triangle_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.temperature_field_to_thermo_triangle_2d"
      end)

    assert frame_operator["kind"] == "solver"
    assert frame_operator["origin"] == "built_in"
    assert frame_operator["input_schema"]["schema"] == "kyuubiki.operator.frame_3d.input"
    assert frame_operator["execution"]["authority_mode"] == "central_operator_library"
    assert frame_operator["execution"]["execution_mode"] == "orchestra_fetch"

    assert frame_operator["execution"]["package_ref"] ==
             "orchestra://operator-package/solve.frame_3d"

    assert "frame" in frame_operator["execution"]["placement_tags"]
    assert "solver_rpc" in frame_operator["execution"]["required_capabilities"]
    assert frame_operator["module"]["id"] == "mechanical.solver"
    assert frame_operator["module"]["label"] == "Structural Solvers"
    assert frame_operator["module"]["lane"] == "physics"

    assert frame_operator["module"]["management"]["library_authority"] ==
             "central_operator_library"

    assert frame_operator["module"]["management"]["agent_replication"] == "forbidden"
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

    assert acoustic_operator["kind"] == "solver"
    assert acoustic_operator["family"] == "acoustic_bar_1d"
    assert acoustic_operator["domain"] == "acoustic"
    assert acoustic_operator["module"]["id"] == "acoustic.solver"
    assert acoustic_operator["module"]["lane"] == "physics"
    assert "wave" in acoustic_operator["capability_tags"]

    assert electrostatic_heat_bridge_operator["kind"] == "workflow_bridge"
    assert electrostatic_heat_bridge_operator["module"]["id"] == "electromagnetic.workflow_bridge"
    assert electrostatic_heat_bridge_operator["module"]["lane"] == "coupling"

    assert electrostatic_heat_bridge_operator["execution"]["source_ref"] ==
             "orchestra://operator/bridge.electrostatic_field_to_heat_quad_2d"

    assert "bridge" in electrostatic_heat_bridge_operator["execution"]["placement_tags"]

    assert "workflow_bridge_runtime" in electrostatic_heat_bridge_operator["execution"][
             "required_capabilities"
           ]

    assert electrostatic_heat_bridge_operator["config_schema"]["schema"] ==
             "kyuubiki.bridge-contract.electrostatic_to_heat.v1"

    assert electrostatic_heat_bridge_operator["config_example"]["contract"]["source"]["field"] ==
             "electric_field_magnitude"

    assert electrostatic_heat_bridge_operator["config_example"]["contract"]["target"]["field"] ==
             "heat_load"

    assert electrostatic_heat_bridge_operator["contract_support"]["source"]["distributions"][
             "node_to_node"
           ] == ["potential", "charge_density"]

    assert electrostatic_heat_bridge_operator["contract_support"]["transform"]["reductions"] == [
             "mean",
             "sum",
             "area_weighted_mean",
             "min",
             "max"
           ]

    assert electrostatic_heat_bridge_operator["contract_support"]["target"]["fields"] == [
             "heat_load",
             "temperature"
           ]

    assert heat_thermo_bridge_operator["config_schema"]["schema"] ==
             "kyuubiki.bridge-contract.heat_to_thermo.v1"

    assert [
             %{
               "id" => "heat_result",
               "artifact_type" => "result/heat_plane_quad_2d",
               "dataset_value" => "heat_result",
               "schema_ref" => %{
                 "schema" => "kyuubiki.operator.heat_plane_quad_2d.output",
                 "version" => "1"
               }
             }
           ] = heat_thermo_bridge_operator["inputs"]

    assert [
             %{
               "id" => "thermo_model",
               "artifact_type" => "study_model/thermal_plane_quad_2d",
               "dataset_value" => "thermo_model",
               "schema_ref" => %{
                 "schema" => "kyuubiki.operator.thermal_plane_quad_2d.input",
                 "version" => "1"
               }
             }
           ] = heat_thermo_bridge_operator["outputs"]

    assert heat_thermo_bridge_operator["config_example"]["contract"]["target"]["field"] ==
             "temperature_delta"

    assert heat_thermo_bridge_operator["contract_support"]["source"]["distributions"][
             "element_to_nodes"
           ] == [
             "average_temperature",
             "heat_flux_x",
             "heat_flux_y",
             "heat_flux",
             "heat_flux_magnitude"
           ]

    assert heat_thermo_bridge_operator["contract_support"]["transform"][
             "default_reduction_by_distribution"
           ] == %{
             "node_to_node" => "copy",
             "element_to_nodes" => "mean"
           }

    assert heat_thermo_bridge_operator["contract_support"]["target"]["fields"] == [
             "temperature_delta"
           ]

    assert [
             %{
               "id" => "heat_result",
               "artifact_type" => "result/heat_plane_triangle_2d",
               "dataset_value" => "heat_result",
               "schema_ref" => %{
                 "schema" => "kyuubiki.operator.heat_plane_triangle_2d.output",
                 "version" => "1"
               }
             }
           ] = heat_thermo_triangle_bridge_operator["inputs"]

    assert [
             %{
               "id" => "thermo_model",
               "artifact_type" => "study_model/thermal_plane_triangle_2d",
               "dataset_value" => "thermo_model",
               "schema_ref" => %{
                 "schema" => "kyuubiki.operator.thermal_plane_triangle_2d.input",
                 "version" => "1"
               }
             }
           ] = heat_thermo_triangle_bridge_operator["outputs"]

    electrostatic_plane_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "solve.electrostatic_plane_triangle_2d"
      end)

    electrostatic_plane_quad_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "solve.electrostatic_plane_quad_2d"
      end)

    modal_frame_operator =
      Enum.find(operators, fn operator -> operator["id"] == "solve.modal_frame_2d" end)

    magnetostatic_quad_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "solve.magnetostatic_plane_quad_2d"
      end)

    magnetostatic_heat_bridge_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "bridge.magnetostatic_field_to_heat_quad_2d"
      end)

    magnetostatic_diagnostics_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "extract.magnetostatic_result_diagnostics"
      end)

    magnetostatic_peak_operator =
      Enum.find(operators, fn operator ->
        operator["id"] == "extract.magnetostatic_peak_field"
      end)

    assert electrostatic_plane_operator["kind"] == "solver"
    assert electrostatic_plane_operator["family"] == "electrostatic_plane_triangle_2d"
    assert electrostatic_plane_operator["domain"] == "electromagnetic"
    assert "triangle" in electrostatic_plane_operator["capability_tags"]
    assert electrostatic_plane_quad_operator["kind"] == "solver"
    assert electrostatic_plane_quad_operator["family"] == "electrostatic_plane_quad_2d"
    assert electrostatic_plane_quad_operator["domain"] == "electromagnetic"
    assert "quad" in electrostatic_plane_quad_operator["capability_tags"]
    assert modal_frame_operator["validation"]["baseline_status"] == "verified"
    assert "modal" in modal_frame_operator["capability_tags"]
    assert magnetostatic_quad_operator["validation"]["baseline_status"] == "verified"
    assert "magnetostatic" in magnetostatic_quad_operator["capability_tags"]
    assert magnetostatic_heat_bridge_operator["kind"] == "workflow_bridge"
    assert magnetostatic_heat_bridge_operator["validation"]["baseline_status"] == "verified"
    assert magnetostatic_diagnostics_operator["kind"] == "extract"
    assert magnetostatic_diagnostics_operator["validation"]["baseline_status"] == "verified"
    assert magnetostatic_peak_operator["kind"] == "extract"
    assert magnetostatic_peak_operator["validation"]["baseline_status"] == "verified"

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

  test "filters built-in operators by managed module" do
    conn =
      :get
      |> conn("/api/v1/operators?module=thermal.solver")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    operators = payload["operators"]
    modules = payload["modules"]
    assert Enum.any?(operators, &(&1["id"] == "solve.heat_plane_quad_2d"))
    assert Enum.all?(operators, &(&1["module"]["id"] == "thermal.solver"))

    assert Enum.all?(
             operators,
             &(&1["module"]["management"]["agent_cache_policy"] == "job_fetch")
           )

    assert [%{"id" => "thermal.solver"} = thermal_module] = modules
    assert thermal_module["operator_count"] == length(operators)
    assert thermal_module["verified_count"] == length(operators)
    assert "heat" in thermal_module["capability_tags"]
  end

  test "surfaces CFD operators as a fluid physics module" do
    conn =
      :get
      |> conn("/api/v1/operators?q=stokes&domain=fluid&kind=solver")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    operators = payload["operators"]
    fluid_module = Enum.find(payload["modules"], &(&1["id"] == "fluid.solver"))

    assert Enum.any?(operators, &(&1["id"] == "solve.stokes_flow_quad_2d"))
    assert Enum.all?(operators, &(&1["domain"] == "fluid"))
    assert fluid_module["lane"] == "physics"
    assert fluid_module["label"] == "Fluid Solvers"
    assert "cfd" in fluid_module["capability_tags"]
  end

  test "surfaces verified modal physics operators as mechanical module coverage" do
    conn =
      :get
      |> conn("/api/v1/operators?q=modal")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    operators = payload["operators"]
    mechanical_module = Enum.find(payload["modules"], &(&1["id"] == "mechanical.solver"))

    assert Enum.any?(operators, &(&1["id"] == "solve.modal_frame_2d"))
    assert Enum.any?(operators, &(&1["id"] == "solve.modal_frame_3d"))
    assert mechanical_module["verified_count"] >= 2
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
