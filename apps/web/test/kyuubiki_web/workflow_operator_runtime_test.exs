defmodule KyuubikiWeb.WorkflowOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule StubSolveClient do
    def solve_electrostatic_bar_1d(payload),
      do: {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => payload}}

    def solve_heat_plane_triangle_2d(payload),
      do: {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => payload}}

    def solve_frame_3d(payload), do: {:ok, %{"solver" => "frame_3d", "payload" => payload}}
  end

  setup do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(original, :solve_runtime_client, StubSolveClient)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
    end)

    :ok
  end

  test "dispatches supported solve operators through the configured runtime client" do
    assert {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => %{"model" => 1}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.electrostatic_bar_1d",
               %{"model" => 1}
             )

    assert {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => %{"model" => 2}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.heat_plane_triangle_2d",
               %{"model" => 2}
             )

    assert {:ok, %{"solver" => "frame_3d", "payload" => %{"model" => 3}}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.frame_3d", %{"model" => 3})
  end

  test "rejects unsupported solve operators" do
    assert {:error, {:unsupported_workflow_solve_operator, "solve.unknown"}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.unknown", %{})
  end

  test "catalog exposes newly wired workflow solve operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "solve.electrostatic_bar_1d")
    assert MapSet.member?(operators, "solve.heat_plane_triangle_2d")
    assert MapSet.member?(operators, "solve.frame_3d")
  end

  test "catalog exposes triangle bridge operators that local chains depend on" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "bridge.electrostatic_field_to_heat_triangle_2d")
    assert MapSet.member?(operators, "bridge.temperature_field_to_thermo_triangle_2d")
  end

  test "bridges electrostatic triangle results into a heat triangle seed model" do
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

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      },
      "contract" =>
        WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(10.0, [
          "node_i",
          "node_j",
          "node_k"
        ])
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [40.0, 40.0, 40.0]
  end
end
