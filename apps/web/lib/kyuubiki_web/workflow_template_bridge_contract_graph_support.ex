defmodule KyuubikiWeb.WorkflowTemplateBridgeContractGraphSupport do
  @moduledoc false

  def edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }
  end

  def heat_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "n0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "hq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.02,
          "conductivity" => 45.0
        }
      ]
    }
  end

  def heat_triangle_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "t0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "t2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "ht0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.02,
          "conductivity" => 45.0
        }
      ]
    }
  end
end
