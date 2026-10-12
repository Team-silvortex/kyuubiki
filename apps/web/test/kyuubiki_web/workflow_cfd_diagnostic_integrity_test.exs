defmodule KyuubikiWeb.WorkflowCfdDiagnosticIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCfdRuntime, as: Runtime

  @node %{"velocity_magnitude" => 0, "pressure" => 0}
  @element %{"divergence_error" => 0, "reynolds_number" => 0, "viscous_dissipation" => 0}
  @valid %{"nodes" => [@node], "elements" => [@element]}

  for {source, fields} <- [
        {"nodes", ~w(velocity_magnitude pressure)},
        {"elements", ~w(divergence_error reynolds_number viscous_dissipation)}
      ] do
    @source source
    @fields fields
    test "#{source}: missing collections cannot manufacture zero diagnostics" do
      for missing <- [nil, [], %{}, false] do
        assert_error(Map.put(@valid, @source, missing), "payload.#{@source}")
      end

      assert_error(Map.delete(@valid, @source), "payload.#{@source}")
    end

    test "#{source}: every selected metric is required on every row" do
      good = hd(@valid[@source])

      for index <- 0..2, field <- @fields, bad <- [nil, "0", false] do
        rows = List.replace_at(List.duplicate(good, 3), index, Map.put(good, field, bad))
        assert_error(Map.put(@valid, @source, rows), "payload.#{@source}[#{index}].#{field}")
      end

      for index <- 0..2, field <- @fields do
        rows = List.replace_at(List.duplicate(good, 3), index, Map.delete(good, field))

        assert_error(
          Map.put(@valid, @source, rows),
          "payload.#{@source}[#{index}].#{field}"
        )
      end

      assert_error(Map.put(@valid, @source, [good, nil]), "payload.#{@source}[1]")
    end
  end

  test "explicit nonconvergence is rejected before diagnostics" do
    for marker <- [
          %{"converged" => false},
          %{"converged" => "true"},
          %{"stability_result" => %{"converged" => false}}
        ] do
      assert_error(Map.merge(@valid, marker), "converged")
    end
  end

  test "mixed aliases participate in both peaks and means, with last ties matching native" do
    input = %{
      "nodes" => [%{"vx" => 3, "vy" => 4, "p" => -2}, %{"speed" => 0, "static_pressure" => 2}],
      "elements" => [
        %{"id" => "a", "div_u" => -3, "re" => 10, "nu_dissipation" => 2},
        %{"id" => "b", "divergence" => 3, "reynolds" => 20, "dissipation" => 4}
      ]
    }

    assert {:ok, result} =
             Runtime.extract_stokes_flow_result_diagnostics(input, %{"output_prefix" => "flow"})

    assert result["flow_velocity_mean"] == 2.5
    assert result["flow_velocity_span"] == 5.0
    assert result["flow_pressure_span"] == 4.0
    assert result["flow_divergence_error_peak"] == 3.0
    assert result["flow_divergence_error_peak_element_id"] == "b"
    assert result["flow_divergence_error_mean"] == 0.0
    assert result["flow_viscous_dissipation_total"] == 6.0
  end

  test "bad canonical and earlier aliases cannot be replaced by healthy samples" do
    assert_error(
      %{@valid | "nodes" => [%{"velocity_magnitude" => nil, "speed" => 0, "pressure" => 0}]},
      "nodes[0].velocity_magnitude"
    )

    assert_error(
      %{
        @valid
        | "elements" => [
            %{
              "div_u" => nil,
              "divergence" => 0,
              "reynolds_number" => 0,
              "viscous_dissipation" => 0
            }
          ]
      },
      "elements[0].div_u"
    )
  end

  test "valid large means need not overflow an unused sum" do
    input = %{@valid | "nodes" => List.duplicate(%{@node | "pressure" => 1.0e308}, 4)}
    assert {:ok, result} = Runtime.extract_stokes_flow_result_diagnostics(input, %{})
    assert_in_delta result["cfd_pressure_mean"] / 1.0e308, 1.0, 1.0e-14
    assert result["cfd_pressure_span"] == 0.0
  end

  test "required overflowing sums and spans reject instead of raising" do
    input = %{
      @valid
      | "elements" => List.duplicate(%{@element | "viscous_dissipation" => 1.0e308}, 2)
    }

    assert_error(input, "non-finite")

    assert_error(
      %{
        @valid
        | "nodes" => [%{@node | "pressure" => -1.0e308}, %{@node | "pressure" => 1.0e308}]
      },
      "non-finite"
    )
  end

  test "zero is retained only when complete zero-valued evidence exists" do
    assert {:ok, result} = Runtime.extract_stokes_flow_result_diagnostics(@valid, %{})
    assert result["cfd_velocity_mean"] == 0.0
    assert result["cfd_divergence_error_peak"] == 0.0
    assert result["cfd_viscous_dissipation_total"] == 0.0
    assert result["diagnostic_node_count"] == 1
    assert result["diagnostic_element_count"] == 1
  end

  defp assert_error(payload, path) do
    assert {:error, reason} = Runtime.extract_stokes_flow_result_diagnostics(payload, %{})
    assert is_binary(reason)
    assert reason =~ path
  end
end
