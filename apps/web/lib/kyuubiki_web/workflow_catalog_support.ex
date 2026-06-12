defmodule KyuubikiWeb.WorkflowCatalogSupport do
  @moduledoc false

  def electrostatic_to_heat_bridge_contract_example(scale) do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => ["node_i", "node_j", "node_k", "node_l"]
      },
      "transform" => %{
        "scale" => scale,
        "reduction" => "mean",
        "default_value" => 0.0
      },
      "target" => %{"field" => "heat_load"}
    }
  end

  def heat_to_thermo_bridge_contract_example do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{"field" => "temperature"},
      "transform" => %{"scale" => 1.0, "default_value" => 0.0},
      "target" => %{"field" => "temperature_delta"}
    }
  end

  def thermo_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "h0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "tq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.02,
          "youngs_modulus" => 210.0e9,
          "poisson_ratio" => 0.3,
          "thermal_expansion" => 11.0e-6
        }
      ]
    }
  end
end
