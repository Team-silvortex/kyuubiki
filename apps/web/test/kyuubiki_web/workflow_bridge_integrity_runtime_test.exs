defmodule KyuubikiWeb.WorkflowBridgeIntegrityRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes bridge integrity transform operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "transform.validate_electrostatic_heat_bridge")
    assert MapSet.member?(operators, "transform.validate_heat_thermo_bridge")
    assert MapSet.member?(operators, "extract.bridge_integrity_diagnostics")
  end

  test "validates electrostatic to heat bridge payloads and annotates the bridged model" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "electric_field_magnitude" => 4.0
        }
      ]
    }

    bridged_heat_model = %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 40.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 40.0},
        %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 40.0}
      ],
      "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
    }

    payload = %{
      "electrostatic_result" => electrostatic_result,
      "heat_model" => bridged_heat_model
    }

    config = %{
      "contract" =>
        WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(10.0, [
          "node_i",
          "node_j",
          "node_k"
        ])
    }

    assert {:ok, validated_model} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_electrostatic_heat_bridge",
               payload,
               config
             )

    assert get_in(validated_model, ["bridge_integrity", "status"]) == "pass"
    assert get_in(validated_model, ["bridge_integrity", "bridge_kind"]) == "electrostatic_to_heat"
    assert get_in(validated_model, ["bridge_integrity", "target_field"]) == "heat_load"
  end

  test "rejects electrostatic to heat bridge payloads when bridged values drift" do
    payload = %{
      "electrostatic_result" => %{
        "nodes" => [
          %{"id" => "e0", "x" => 0.0, "y" => 0.0},
          %{"id" => "e1", "x" => 1.0, "y" => 0.0},
          %{"id" => "e2", "x" => 0.0, "y" => 1.0}
        ],
        "elements" => [
          %{
            "id" => "et0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "electric_field_magnitude" => 4.0
          }
        ]
      },
      "heat_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 10.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 40.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 40.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      }
    }

    assert {:error, {:bridge_integrity_failed, report}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_electrostatic_heat_bridge",
               payload,
               %{
                 "contract" =>
                   WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(10.0, [
                     "node_i",
                     "node_j",
                     "node_k"
                   ])
               }
             )

    assert report["value_mismatch_count"] == 1
  end

  test "validates heat to thermo bridge payloads and annotates the bridged model" do
    payload = %{
      "heat_result" => %{
        "nodes" => [
          %{"id" => "n0", "x" => 0.0, "y" => 0.0, "temperature" => 30.0},
          %{"id" => "n1", "x" => 1.0, "y" => 0.0, "temperature" => 45.0},
          %{"id" => "n2", "x" => 0.0, "y" => 1.0, "temperature" => 60.0}
        ],
        "elements" => [%{"id" => "e0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      },
      "thermo_model" => %{
        "nodes" => [
          %{"id" => "t0", "x" => 0.0, "y" => 0.0, "temperature_delta" => 30.0},
          %{"id" => "t1", "x" => 1.0, "y" => 0.0, "temperature_delta" => 45.0},
          %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature_delta" => 60.0}
        ],
        "elements" => [%{"id" => "te0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      }
    }

    assert {:ok, validated_model} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_heat_thermo_bridge",
               payload,
               %{"contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()}
             )

    assert get_in(validated_model, ["bridge_integrity", "status"]) == "pass"
    assert get_in(validated_model, ["bridge_integrity", "bridge_kind"]) == "heat_to_thermo"
    assert get_in(validated_model, ["bridge_integrity", "target_field"]) == "temperature_delta"
  end

  test "extracts bridge integrity reports into diagnostics bundles" do
    validated_model = %{
      "nodes" => [],
      "elements" => [],
      "bridge_integrity" => %{
        "status" => "pass",
        "bridge_kind" => "electrostatic_to_heat",
        "distribution" => "element_to_nodes",
        "source_field" => "electric_field_magnitude",
        "target_field" => "heat_load",
        "checked_node_count" => 3,
        "checked_element_count" => 1,
        "value_mismatch_count" => 0,
        "missing_value_count" => 0,
        "coordinate_mismatch_count" => 0,
        "node_count_mismatch" => false,
        "element_count_mismatch" => false,
        "max_abs_error" => 0.0,
        "tolerance" => 1.0e-6
      }
    }

    assert {:ok, diagnostics} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.bridge_integrity_diagnostics",
               validated_model,
               %{"output_prefix" => "eh_bridge"}
             )

    assert diagnostics["diagnostic_contract"] == "kyuubiki.workflow_diagnostics/v1"
    assert diagnostics["diagnostic_domain"] == "workflow_bridge"
    assert diagnostics["diagnostic_subject"] == "bridge_integrity"
    assert diagnostics["eh_bridge_integrity_passed"] == 1.0
    assert diagnostics["eh_bridge_value_mismatch_count"] == 0.0
    assert diagnostics["eh_bridge_bridge_kind"] == "electrostatic_to_heat"

    assert {:ok, bundle} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_bundle",
               %{"bridge" => diagnostics},
               %{}
             )

    assert bundle["bundle_domains"] == ["workflow_bridge"]
    assert bundle["bundle_sources"] == ["bridge"]
    assert "eh_bridge_max_abs_error" in bundle["bundle_numeric_fields"]
  end
end
