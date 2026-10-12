defmodule KyuubikiWeb.WorkflowThermalDiagnosticIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for {domain, scalar, source, x, y, magnitude, group} <- [
        {"thermal", "temperature", "elements", "heat_flux_x", "heat_flux_y",
         "heat_flux_magnitude", "flux"},
        {"thermo", "temperature_delta", "nodes", "ux", "uy", "displacement_magnitude",
         "displacement"}
      ] do
    @domain domain
    @scalar scalar
    @source source
    @x x
    @y y
    @magnitude magnitude
    @group group
    @valid if(source == "nodes",
             do: %{
               "nodes" => [%{scalar => 1, x => 3, y => 4}, %{scalar => 3, x => 5, y => 12}],
               "elements" => []
             },
             else: %{
               "nodes" => [%{scalar => 1}, %{scalar => 3}],
               "elements" => [%{x => 3, y => 4}, %{x => 5, y => 12}]
             }
           )

    test "#{domain}: metadata and nonconvergence are not diagnostic evidence" do
      for payload <- [
            %{"nodes" => [], "elements" => []},
            %{"nodes" => [%{}], "elements" => [%{}]}
          ] do
        rejects(@domain, payload, %{}, "diagnostic fields")
      end

      for marker <- [false, nil, "true"] do
        rejects(@domain, Map.put(@valid, "converged", marker), %{}, "payload.converged")
      end

      rejects(
        @domain,
        Map.put(@valid, "stability_result", %{"converged" => false}),
        %{},
        "stability_result.converged"
      )
    end

    test "#{domain}: scalar samples cannot disappear from first middle or last positions" do
      for index <- 0..2,
          bad <- [%{}, %{@scalar => nil}, %{@scalar => "0"}, %{@scalar => false}] do
        nodes = List.duplicate(%{@scalar => 0}, 3) |> List.replace_at(index, bad)

        rejects(
          @domain,
          Map.put(@valid, "nodes", nodes),
          %{},
          "payload.nodes[#{index}].#{@scalar}"
        )
      end
    end

    test "#{domain}: selected vectors cannot fall back or omit samples" do
      for value <- [nil, "13", -1] do
        rejects(
          @domain,
          put_in(@valid, [@source, Access.at(1), @magnitude], value),
          %{},
          "payload.#{@source}[1].#{@magnitude}"
        )
      end

      rejects(
        @domain,
        update_in(@valid, [@source, Access.at(1)], &Map.delete(&1, @y)),
        %{},
        "payload.#{@source}[1].#{@y}"
      )

      rejects(@domain, put_in(@valid, [@source, Access.at(0)], %{}), %{}, "payload.#{@source}[0]")
    end

    test "#{domain}: malformed collections records and configuration reject" do
      for source <- ~w(nodes elements), bad <- [nil, false, %{}] do
        rejects(@domain, Map.put(@valid, source, bad), %{}, "payload.#{source}")
        rejects(@domain, Map.put(@valid, source, [nil]), %{}, "payload.#{source}[0]")
      end

      for config <- [false, [], "defaults"] do
        rejects(@domain, @valid, config, "config")
      end

      for key <- ["#{@scalar}_field", "#{@group}_z_field", "node_source", "output_prefix"],
          bad <- [nil, false, " "] do
        rejects(@domain, @valid, %{key => bad}, "config.#{key}")
      end

      rejects(@domain, @valid, %{"#{@scalar}_field" => "missing"}, "payload.nodes[0].missing")

      rejects(
        @domain,
        @valid,
        %{"#{@group}_magnitude_field" => "missing"},
        "payload.#{@source}[0].missing"
      )
    end

    test "#{domain}: optional groups stay absent and actual zeros remain measured" do
      assert {:ok, result} = run(@domain, %{"nodes" => [%{@scalar => 0}], "elements" => []}, nil)
      assert result["#{@domain}_#{@scalar}_mean"] == 0
      refute Map.has_key?(result, "#{@domain}_#{@group}_peak_magnitude")
      if @domain == "thermal", do: refute(Map.has_key?(result, "thermal_total_heat_load"))
    end

    test "#{domain}: stable vector norms and required sums have explicit numeric boundaries" do
      for scale <- [1.0e200, 1.0e-200] do
        rows = [%{@x => 3 * scale, @y => 4 * scale}]
        payload = Map.put(%{"nodes" => [], "elements" => []}, @source, rows)
        assert {:ok, result} = run(@domain, payload)
        assert_in_delta result["#{@domain}_#{@group}_peak_magnitude"] / (5 * scale), 1, 1.0e-14
      end

      for values <- [[1.0e308, 1.0e308], [-1.0e308, 1.0e308]] do
        rejects(
          @domain,
          Map.put(@valid, "nodes", Enum.map(values, &%{@scalar => &1})),
          %{},
          "non-finite"
        )
      end
    end

    test "#{domain}: explicit third dimensions and custom collection mappings are honored" do
      config = %{
        "node_source" => "points",
        "element_source" => "cells",
        "output_prefix" => "sample A",
        "#{@group}_x_field" => "x",
        "#{@group}_y_field" => "y",
        "#{@group}_z_field" => "z"
      }

      source = if @source == "nodes", do: "points", else: "cells"

      payload =
        Map.put(%{"points" => [], "cells" => []}, source, [%{"x" => 3, "y" => 4, "z" => 12}])

      assert {:ok, result} = run(@domain, payload, config)
      assert result["sample_A_#{@group}_peak_magnitude"] == 13
      assert result["sample_A_#{@group}_peak_z"] == 12

      rejects(
        @domain,
        update_in(payload, [source, Access.at(0)], &Map.delete(&1, "z")),
        config,
        "payload.#{source}[0].z"
      )
    end

    test "#{domain}: large healthy prefixes cannot hide corrupt tails" do
      nodes = List.duplicate(%{@scalar => 1}, 4096) |> List.replace_at(4095, %{@scalar => nil})
      rejects(@domain, Map.put(@valid, "nodes", nodes), %{}, "payload.nodes[4095].#{@scalar}")
    end
  end

  test "signed heat loads retain cancellation counts peak identity and ranges" do
    nodes = [
      %{"id" => "a", "temperature" => -1, "heat_load" => -4},
      %{"id" => "b", "temperature" => 0, "heat_load" => 0},
      %{"id" => "c", "temperature" => 5, "heat_load" => 4}
    ]

    assert {:ok, result} = run("thermal", %{"nodes" => nodes, "elements" => []})
    assert result["thermal_loaded_node_count"] == 2
    assert result["thermal_total_heat_load"] == 0
    assert result["thermal_temperature_min_node_id"] == "a"
    assert result["thermal_temperature_max_node_id"] == "c"
    assert result["thermal_peak_heat_load"] == 4
    assert result["thermal_peak_heat_load_node_id"] == "c"

    for field <- ~w(heat_load temperature) do
      rejects(
        "thermal",
        %{"nodes" => List.replace_at(nodes, 1, %{}), "elements" => []},
        %{"#{field}_field" => field},
        "payload.nodes[1]"
      )
    end
  end

  test "heat loads cannot be filtered out beside otherwise healthy temperatures" do
    for index <- 0..2, row <- [%{}, %{"heat_load" => nil}, %{"heat_load" => "0"}] do
      nodes =
        List.duplicate(%{"temperature" => 20, "heat_load" => 0}, 3)
        |> List.replace_at(index, Map.put(row, "temperature", 20))

      rejects(
        "thermal",
        %{"nodes" => nodes, "elements" => []},
        %{},
        "payload.nodes[#{index}].heat_load"
      )
    end

    payload = %{"nodes" => [%{"temperature" => 20, "heat_load" => 1}], "elements" => []}
    rejects("thermal", payload, %{"heat_load_field" => "requested"}, "payload.nodes[0].requested")
    rejects("thermal", payload, %{"heat_load_field" => false}, "config.heat_load_field")
  end

  test "temperature gradients are complete even when flux and temperature remain healthy" do
    row = %{
      "temperature_gradient_x" => 3,
      "temperature_gradient_y" => 4,
      "heat_flux_magnitude" => 10
    }

    payload = %{"nodes" => [%{"temperature" => 20}], "elements" => [row, row]}
    assert {:ok, result} = run("thermal", payload)
    assert result["thermal_gradient_peak_magnitude"] == 5

    for index <- 0..1,
        damaged <- [
          Map.delete(row, "temperature_gradient_y"),
          Map.put(row, "temperature_gradient_y", nil)
        ] do
      rejects(
        "thermal",
        put_in(payload, ["elements", Access.at(index)], damaged),
        %{},
        "payload.elements[#{index}].temperature_gradient_y"
      )
    end

    rejects(
      "thermal",
      payload,
      %{"gradient_magnitude_field" => "norm"},
      "payload.elements[0].norm"
    )

    magnitude_only = %{"nodes" => [], "elements" => [%{"norm" => 7}, %{"norm" => 13}]}
    assert {:ok, result} = run("thermal", magnitude_only, %{"gradient_magnitude_field" => "norm"})
    assert result["thermal_gradient_peak_magnitude"] == 13

    rejects(
      "thermal",
      put_in(magnitude_only, ["elements", Access.at(1), "norm"], -1),
      %{"gradient_magnitude_field" => "norm"},
      "payload.elements[1].norm"
    )
  end

  test "thermo mixed aliases and nonzero cooling participate in complete reductions" do
    payload = %{
      "nodes" => [
        %{"temperature_delta" => -2, "ux" => 3, "uy" => 4},
        %{"temperature_delta" => 0, "displacement_x" => 5, "displacement_y" => 12}
      ],
      "elements" => [%{"id" => "a", "von_mises_stress" => 1}, %{"id" => "b", "von_mises" => 50}]
    }

    assert {:ok, result} = run("thermo", payload)
    assert result["thermo_heated_node_count"] == 1
    assert result["thermo_peak_displacement"] == 13
    assert result["thermo_stress_peak"] == 50
    assert result["thermo_peak_stress_id"] == "b"

    rejects(
      "thermo",
      put_in(payload, ["nodes", Access.at(1), "ux"], nil),
      %{},
      "payload.nodes[1].ux"
    )
  end

  test "stress fallback cannot hide corrupt or partial selected samples" do
    payload = %{
      "nodes" => [],
      "elements" => [%{"von_mises_stress" => 1}, %{"stress_x" => 2}],
      "max_stress" => 2
    }

    rejects("thermo", payload, %{}, "payload.elements[1].von_mises_stress")

    rejects(
      "thermo",
      put_in(payload, ["elements", Access.at(1), "von_mises_stress"], nil),
      %{},
      "payload.elements[1].von_mises_stress"
    )

    rejects(
      "thermo",
      %{"nodes" => [], "elements" => [%{"stress_x" => 2}], "max_stress" => nil},
      %{},
      "payload.max_stress"
    )

    rejects("thermo", payload, %{"stress_field" => "requested"}, "payload.elements[0].requested")
  end

  test "stress and strain component peaks compare every selected axis and preserve sign" do
    for field <- ~w(stress thermal_strain mechanical_strain total_strain) do
      payload = %{
        "nodes" => [],
        "elements" => [
          %{"id" => "a", "#{field}_x" => 1, "#{field}_y" => -10},
          %{"id" => "b", "#{field}_x" => 2, "#{field}_y" => -20}
        ]
      }

      assert {:ok, result} = run("thermo", payload)
      assert result["thermo_peak_#{field}"] == -20
      assert result["thermo_peak_#{field}_id"] == "b"

      rejects(
        "thermo",
        update_in(payload, ["elements", Access.at(1)], &Map.delete(&1, "#{field}_y")),
        %{},
        "payload.elements[1].#{field}_y"
      )
    end
  end

  test "configured strain and scalar groups cannot fall back to components" do
    for field <- ~w(thermal_strain mechanical_strain total_strain) do
      payload = %{"nodes" => [], "elements" => [%{field => 0.1}, %{"#{field}_x" => 0.1}]}
      rejects("thermo", payload, %{}, "payload.elements[1].#{field}")

      rejects(
        "thermo",
        payload,
        %{"#{field}_field" => "requested"},
        "payload.elements[0].requested"
      )
    end
  end

  test "authoritative stress ignores unused fallback data and keeps missing versus null identities" do
    for {row, id} <- [{%{"von_mises" => 12}, "unknown"}, {%{"id" => nil, "von_mises" => 12}, nil}] do
      assert {:ok, result} =
               run("thermo", %{"nodes" => [], "elements" => [row], "max_stress" => nil})

      assert result["thermo_peak_stress_id"] == id
      assert result["thermo_stress_peak_element_id"] == nil
    end

    assert {:ok, result} = run("thermo", %{"nodes" => [], "elements" => [], "max_stress" => 12})
    assert result["thermo_peak_stress_id"] == "max_stress"
  end

  test "legacy displacement mappings work without overriding an explicit primary mapping" do
    payload = %{"nodes" => [%{"x" => 3, "y" => 4, "z" => 12}], "elements" => []}
    config = %{"ux_field" => "x", "uy_field" => "y", "uz_field" => "z"}
    assert {:ok, result} = run("thermo", payload, config)
    assert result["thermo_peak_displacement"] == 13

    rejects(
      "thermo",
      payload,
      Map.put(config, "displacement_x_field", nil),
      "config.displacement_x_field"
    )

    assert {:ok, result} =
             run("thermo", %{"nodes" => [%{"norm" => 7}], "elements" => []}, %{
               "displacement_field" => "norm"
             })

    assert result["thermo_peak_displacement"] == 7

    rejects(
      "thermo",
      payload,
      %{"temperature_delta_field" => false, "temperature_field" => "x"},
      "config.temperature_delta_field"
    )
  end

  test "scalar strain peaks follow signed maxima, distinct from absolute component peaks" do
    for field <- ~w(thermal_strain mechanical_strain total_strain) do
      payload = %{
        "nodes" => [],
        "elements" => [
          %{"id" => "compression", field => -0.8},
          %{"id" => "tension", field => 0.1}
        ]
      }

      assert {:ok, result} = run("thermo", payload)
      assert result["thermo_#{field}_peak"] == 0.1
      assert result["thermo_peak_#{field}_id"] == "tension"
    end
  end

  test "all displacement mapping aliases enforce presence-based precedence" do
    payload = %{"nodes" => [%{"x" => 3, "y" => 4, "z" => 12, "norm" => 7}], "elements" => []}

    for {primary, legacy, field} <- [
          {"displacement_x_field", "ux_field", "x"},
          {"displacement_y_field", "uy_field", "y"},
          {"displacement_z_field", "uz_field", "z"},
          {"displacement_magnitude_field", "displacement_field", "norm"}
        ],
        invalid <- [nil, false, " "] do
      base = %{"displacement_field" => "norm"}

      rejects(
        "thermo",
        payload,
        Map.merge(base, %{primary => invalid, legacy => field}),
        "config.#{primary}"
      )

      rejects("thermo", payload, Map.put(base, legacy, invalid), "config.#{legacy}")

      assert {:ok, _} =
               run("thermo", payload, Map.merge(base, %{primary => field, legacy => invalid}))

      rejects("thermo", payload, Map.put(base, legacy, "missing"), "payload.nodes[0].missing")
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
