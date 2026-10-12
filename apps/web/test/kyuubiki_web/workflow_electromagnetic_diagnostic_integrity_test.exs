defmodule KyuubikiWeb.WorkflowElectromagneticDiagnosticIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for {domain, scalar, scalar_alias, secondary, x, y, magnitude, energy, energy_alias} <- [
        {"electrostatic", "potential", "phi", "charge_density", "electric_field_x",
         "electric_field_y", "electric_field_magnitude", "energy_density",
         "electric_energy_density"},
        {"magnetostatic", "vector_potential", "a", "current_density", "magnetic_field_strength_x",
         "magnetic_field_strength_y", "magnetic_field_strength_magnitude", "energy_area_density",
         "energy_density"}
      ] do
    @domain domain
    @scalar scalar
    @scalar_alias scalar_alias
    @secondary secondary
    @x x
    @y y
    @magnitude magnitude
    @energy energy
    @energy_alias energy_alias
    @input %{
      "nodes" => [%{"id" => "n0", scalar => 1}, %{"id" => "n1", scalar => 3}],
      "elements" => [%{"id" => "e0", x => 3, y => 4}, %{"id" => "e1", x => 5, y => 12}]
    }

    test "#{domain}: metadata alone is not measurement evidence" do
      for payload <- [
            %{"nodes" => [], "elements" => []},
            %{"nodes" => [%{}], "elements" => [%{}]}
          ] do
        rejects(@domain, payload, %{}, "diagnostic fields")
      end
    end

    test "#{domain}: invalid collections and rows cannot be filtered out" do
      for source <- ~w(nodes elements), invalid <- [nil, false, 3, %{}] do
        rejects(@domain, Map.put(@input, source, invalid), %{}, "payload.#{source}")

        rejects(
          @domain,
          put_in(@input, [source, Access.at(1)], invalid),
          %{},
          "payload.#{source}[1]"
        )
      end
    end

    test "#{domain}: nonconvergence cannot disappear in the summary" do
      for value <- [false, nil, "true"] do
        rejects(@domain, Map.put(@input, "converged", value), %{}, "payload.converged")
      end

      rejects(
        @domain,
        Map.put(@input, "stability_result", %{"converged" => false}),
        %{},
        "payload.stability_result.converged"
      )
    end

    test "#{domain}: incomplete scalar groups cannot shrink sample counts" do
      for field <- [@scalar, @secondary], index <- 0..2 do
        rows = List.duplicate(%{field => 0}, 3)
        payload = Map.put(@input, "nodes", List.replace_at(rows, index, %{}))
        rejects(@domain, payload, %{}, "payload.nodes[#{index}].#{field}")
      end
    end

    test "#{domain}: invalid canonical values cannot use a healthy alias" do
      for invalid <- [nil, false, "0", [], %{}] do
        payload =
          put_in(@input, ["nodes", Access.at(1)], %{@scalar => invalid, @scalar_alias => 0})

        rejects(@domain, payload, %{}, "payload.nodes[1].#{@scalar}")
      end
    end

    test "#{domain}: mixed energy aliases participate in the same peak" do
      payload =
        @input
        |> put_in(["elements", Access.at(0), @energy], 1)
        |> put_in(["elements", Access.at(1), @energy_alias], 50)

      assert {:ok, summary} = run(@domain, payload)
      assert summary["#{@domain}_energy_density_peak"] == 50
      assert summary["#{@domain}_energy_density_peak_element_id"] == "e1"
    end

    test "#{domain}: bad energy groups and partial vectors cannot vanish" do
      payload = put_in(@input, ["elements", Access.at(0), @energy], 1)
      rejects(@domain, payload, %{}, "payload.elements[1].#{@energy}")
      payload = update_in(@input, ["elements", Access.at(1)], &Map.delete(&1, @y))
      rejects(@domain, payload, %{}, "payload.elements[1].#{@y}")
      payload = put_in(@input, ["elements", Access.at(0)], %{})
      rejects(@domain, payload, %{}, "payload.elements[0].field")
    end

    test "#{domain}: corrupt magnitudes and components cannot fall back" do
      for invalid <- [nil, "13", -1] do
        payload = put_in(@input, ["elements", Access.at(1), @magnitude], invalid)
        rejects(@domain, payload, %{}, "payload.elements[1].#{@magnitude}")
      end

      payload =
        @input
        |> put_in(["elements", Access.at(1), @magnitude], 13)
        |> put_in(["elements", Access.at(1), @x], nil)

      rejects(@domain, payload, %{}, "payload.elements[1].#{@x}")
    end

    test "#{domain}: invalid configuration never selects defaults" do
      for config <- [false, [], "default"] do
        rejects(@domain, @input, config, "config")
      end

      for key <- [
            "node_source",
            "element_source",
            "output_prefix",
            "#{@scalar}_field",
            "field_z_field"
          ],
          invalid <- [nil, false, " "] do
        rejects(@domain, @input, %{key => invalid}, "config.#{key}")
      end

      rejects(@domain, @input, %{"output_prefix" => "---"}, "config.output_prefix")
    end

    test "#{domain}: explicit mappings require exactly the requested field" do
      rejects(@domain, @input, %{"#{@scalar}_field" => "requested"}, "payload.nodes[0].requested")

      rejects(
        @domain,
        @input,
        %{"field_magnitude_field" => "requested"},
        "payload.elements[0].requested"
      )

      payload = put_in(@input, ["elements", Access.at(0), @magnitude], 5)

      rejects(
        @domain,
        payload,
        %{"field_x_field" => "requested"},
        "payload.elements[0].requested"
      )
    end

    test "#{domain}: entirely absent optional groups stay absent and zero stays valid" do
      assert {:ok, summary} =
               run(@domain, %{"nodes" => [], "elements" => [%{@magnitude => 0}]}, nil)

      assert summary["#{@domain}_field_peak_magnitude"] == 0
      refute Map.has_key?(summary, "#{@domain}_#{@scalar}_mean")
      assert {:ok, summary} = run(@domain, %{"nodes" => [%{@scalar => 0}], "elements" => []})
      assert summary["#{@domain}_#{@scalar}_mean"] == 0
      refute Map.has_key?(summary, "#{@domain}_field_peak_magnitude")
    end

    test "#{domain}: stable norms preserve large and small representable vectors" do
      for scale <- [1.0e200, 1.0e-200] do
        payload = %{"nodes" => [], "elements" => [%{@x => 3 * scale, @y => 4 * scale}]}
        assert {:ok, summary} = run(@domain, payload)
        assert_in_delta summary["#{@domain}_field_peak_magnitude"] / (5 * scale), 1, 1.0e-14
      end
    end

    test "#{domain}: unrepresentable aggregates reject without an arithmetic crash" do
      for values <- [[1.0e308, 1.0e308], [-1.0e308, 1.0e308]] do
        payload = Map.put(@input, "nodes", Enum.map(values, &%{@scalar => &1}))
        rejects(@domain, payload, %{}, "non-finite")
      end

      payload = %{"nodes" => [], "elements" => [%{@x => 1.7e308, @y => 1.7e308}]}
      rejects(@domain, payload, %{}, "non-finite")
    end

    test "#{domain}: custom sources and three-dimensional vectors retain last peak ties" do
      config = %{
        "node_source" => "points",
        "element_source" => "cells",
        "output_prefix" => "sample A",
        "#{@scalar}_field" => "value",
        "field_x_field" => "x",
        "field_y_field" => "y",
        "field_z_field" => "z"
      }

      payload = %{
        "points" => [%{"value" => -3}, %{"value" => 1}],
        "cells" => [
          %{"id" => "a", "x" => 3, "y" => 4, "z" => 12},
          %{"id" => "b", "x" => 3, "y" => 4, "z" => 12}
        ]
      }

      assert {:ok, summary} = run(@domain, payload, config)
      assert summary["sample_A_#{@scalar}_mean"] == -1
      assert summary["sample_A_field_peak_magnitude"] == 13
      assert summary["sample_A_field_peak_element_id"] == "b"
      assert summary["sample_A_field_peak_z"] == 12

      rejects(
        @domain,
        update_in(payload, ["cells", Access.at(1)], &Map.delete(&1, "z")),
        config,
        "payload.cells[1].z"
      )
    end

    test "#{domain}: healthy scalar reductions match independent references" do
      for count <- [1, 2, 3, 64, 257] do
        values = Enum.map(0..(count - 1), &(&1 - 2))
        payload = %{"nodes" => Enum.map(values, &%{@scalar => &1}), "elements" => []}
        assert {:ok, summary} = run(@domain, payload)
        assert summary["#{@domain}_#{@scalar}_count"] == count
        assert summary["#{@domain}_#{@scalar}_sum"] == Enum.sum(values)
        assert summary["#{@domain}_#{@scalar}_mean"] == Enum.sum(values) / count
        assert summary["#{@domain}_#{@scalar}_nonzero_count"] == Enum.count(values, &(&1 != 0))
      end
    end

    test "#{domain}: tail corruption cannot hide behind a healthy prefix" do
      nodes = List.duplicate(%{@scalar => 1}, 4096) |> List.replace_at(4095, %{@scalar => nil})
      rejects(@domain, Map.put(@input, "nodes", nodes), %{}, "payload.nodes[4095].#{@scalar}")
    end
  end

  test "magnetic whole-group energy fallback cannot patch partial density samples" do
    payload = %{
      "nodes" => [],
      "elements" => [%{"energy_area_density" => 1}, %{"stored_energy" => 100}]
    }

    rejects("magnetostatic", payload, %{}, "payload.elements[1].energy_area_density")
    payload = %{"nodes" => [], "elements" => [%{"stored_energy" => 1}, %{"stored_energy" => 100}]}
    assert {:ok, summary} = run("magnetostatic", payload)
    assert summary["magnetostatic_energy_density_peak"] == 100
  end

  test "electrostatic charge-density source selection cannot silently choose nodes" do
    payload = %{
      "points" => [%{"rho" => 100}],
      "cells" => [%{"rho" => -2}, %{"rho" => 5}]
    }

    config = %{
      "node_source" => "points",
      "element_source" => "cells",
      "charge_density_field" => "rho"
    }

    for source <- ~w(element elements) do
      selected = Map.put(config, "charge_density_source", source)
      assert {:ok, summary} = run("electrostatic", payload, selected)
      assert summary["electrostatic_charge_density_count"] == 2
      assert summary["electrostatic_charge_density_sum"] == 3

      rejects(
        "electrostatic",
        put_in(payload, ["cells", Access.at(1)], %{}),
        selected,
        "payload.cells[1].rho"
      )
    end

    for source <- ~w(node nodes) do
      assert {:ok, summary} =
               run("electrostatic", payload, Map.put(config, "charge_density_source", source))

      assert summary["electrostatic_charge_density_sum"] == 100
    end

    for invalid <- [nil, false, " ", "automatic"] do
      rejects(
        "electrostatic",
        payload,
        Map.put(config, "charge_density_source", invalid),
        "config.charge_density_source"
      )
    end
  end

  defp run(domain, payload, config \\ %{}),
    do: Runtime.run_extract_operator("extract.#{domain}_result_diagnostics", payload, config)

  defp rejects(domain, payload, config, path) do
    assert {:error, reason} = run(domain, payload, config)
    assert is_binary(reason)
    assert reason =~ path
  end
end
