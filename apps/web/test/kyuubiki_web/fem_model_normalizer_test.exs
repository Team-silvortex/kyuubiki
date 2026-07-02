defmodule KyuubikiWeb.FemModelNormalizerTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.FemModelNormalizer

  test "preserves transient solver controls beyond graph topology" do
    params = %{
      "nodes" => [%{"id" => "fixed"}, %{"id" => "tip"}],
      "elements" => [%{"id" => "s0", "node_i" => 0, "node_j" => 1}],
      "time_step" => 0.01,
      "steps" => 10
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_transient_spring_1d(params)
    assert normalized["time_step"] == 0.01
    assert normalized["steps"] == 10
  end

  test "preserves harmonic frequency sweep controls" do
    params = %{
      nodes: [%{id: "fixed"}, %{id: "tip"}],
      elements: [%{id: "s0", node_i: 0, node_j: 1}],
      frequencies_hz: [0.0, 0.5, 1.0]
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_harmonic_spring_1d(params)
    assert normalized["nodes"] == params.nodes
    assert normalized["elements"] == params.elements
    assert normalized[:frequencies_hz] == [0.0, 0.5, 1.0]
  end
end
