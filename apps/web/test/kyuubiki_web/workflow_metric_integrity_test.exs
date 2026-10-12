defmodule KyuubikiWeb.WorkflowMetricIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowDomainMetricResolver, as: Metrics
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @corrupt [
    {"canonical cannot fall back", "max_stress", %{"max_stress" => nil, "peak_stress" => 0},
     "max_stress"},
    {"earlier alias cannot fall back", "max_stress",
     %{"peak_stress" => nil, "von_mises_peak" => 0}, "peak_stress"},
    {"dynamic alias cannot fall back", "max_displacement",
     %{"peak_displacement" => false, "u_max" => 0}, "peak_displacement"},
    {"invalid collection cannot use nodes", "max_velocity",
     %{"frequencies" => nil, "nodes" => [%{"vx" => 0}]}, "frequencies"},
    {"malformed frequency row", "max_velocity", %{"frequencies" => [%{"v_peak" => 0}, nil]},
     "frequencies[1]"},
    {"missing frequency metric", "max_velocity", %{"frequencies" => [%{"v_peak" => 0}, %{}]},
     "frequencies[1].max_velocity"},
    {"invalid frequency alias", "max_velocity",
     %{"frequencies" => [%{"v_peak" => 0}, %{"peak_velocity" => nil, "v_peak" => 0}]},
     "frequencies[1].peak_velocity"},
    {"invalid peak coordinate", "peak_frequency_hz",
     %{
       "frequencies" => [
         %{"frequency_hz" => 1, "u_peak" => 2},
         %{"frequency_hz" => "3", "u_peak" => 0}
       ]
     }, "frequencies[1].frequency_hz"},
    {"missing peak amplitude", "peak_frequency_hz",
     %{"frequencies" => [%{"frequency_hz" => 1, "u_peak" => 2}, %{"frequency_hz" => 3}]},
     "frequencies[1].max_displacement"},
    {"invalid mode cannot shrink span", "frequency_span_hz",
     %{"modes" => [%{"frequency_hz" => 1}, %{"frequency_hz" => nil}]}, "modes[1].frequency_hz"},
    {"invalid bound cannot use modes", "frequency_span_hz",
     %{"min_frequency_hz" => nil, "modes" => [%{"frequency_hz" => 1}]}, "min_frequency_hz"},
    {"invalid mode shape", "min_frequency_hz", %{"modes" => [%{"frequency_hz" => 1}, false]},
     "modes[1]"},
    {"invalid mode collection", "max_frequency_hz", %{"modes" => %{}}, "modes"},
    {"invalid participation", "mode_1_participation_norm",
     %{"modes" => [%{"participation_norm" => "0"}]}, "modes[0].participation_norm"},
    {"invalid transient node", "max_acceleration", %{"nodes" => [%{"ax" => 0}, %{"ax" => nil}]},
     "nodes[1].ax"},
    {"missing transient sample", "max_displacement", %{"nodes" => [%{"ux" => 0}, %{}]},
     "nodes[1].ux"},
    {"invalid thermal bound alias", "thermo_temperature_delta_max",
     %{"temperature_max" => nil, "max_temperature" => 1, "temperature_min" => 0},
     "temperature_max"},
    {"invalid voltage bound alias", "electrostatic_potential_span",
     %{"potential_max" => false, "max_voltage" => 1, "potential_min" => 0}, "potential_max"},
    {"invalid concentration bound alias", "transport_concentration_span",
     %{"concentration_min" => nil, "min_concentration" => 0, "concentration_max" => 1},
     "concentration_min"},
    {"invalid speed cannot use vector", "velocity_magnitude",
     %{"speed" => nil, "vx" => 0, "vy" => 0}, "speed"},
    {"invalid vector component", "velocity_magnitude", %{"vx" => 0, "vy" => "0"}, "vy"},
    {"invalid generic bound", "custom_span", %{"custom_min" => 0, "custom_max" => false},
     "custom_max"}
  ]

  for {name, field, payload, bad_path} <- @corrupt do
    @field field
    @payload payload
    @bad_path bad_path
    test "#{name} blocks guard and pair decisions" do
      assert {:error, resolution_error} = Metrics.checked_metric_value(@payload, @field)
      assert resolution_error =~ "payload.#{@bad_path}"
      assert Metrics.fetch_metric(@payload, @field) == :error
      assert {:error, reason} = guard(@payload, @field)
      assert reason =~ @bad_path
      assert reason =~ "config.rules[0]"

      for side <- ~w(left right) do
        pair = Map.put(%{"left" => %{@field => 0}, "right" => %{@field => 0}}, side, @payload)

        assert {:error, reason} =
                 Runtime.run_transform_operator(
                   "transform.benchmark_structural_pair",
                   pair,
                   %{"criteria" => [%{"field" => @field}]}
                 )

        assert reason =~ "payload.#{side}.#{@bad_path}"
        assert reason =~ "config.criteria[0]"
      end

      assert Metrics.metric_value(@payload, @field) == nil
    end
  end

  test "empty selected samples cannot fall through to an unrelated source" do
    payload = %{"frequencies" => [], "nodes" => [%{"vx" => 0}]}
    assert Metrics.checked_metric_value(payload, "max_velocity") == {:ok, nil}
    assert {:error, _} = guard(payload, "max_velocity")
    assert Metrics.metric_value(payload, "max_velocity") == nil
  end

  test "selected metrics tolerate unrelated bad fields but never coerce numeric strings" do
    assert Metrics.metric_value(%{"max_stress" => 0, "peak_stress" => nil}, "max_stress") == 0.0

    assert Metrics.metric_value(%{"von_mises_peak" => 0, "stress_peak" => false}, "max_stress") ==
             0.0

    assert {:error, _} = guard(%{"peak_stress" => "0"}, "max_stress")
  end

  test "valid derived zeros and stable large vector norms are preserved" do
    assert Metrics.metric_value(%{"modes" => [%{"frequency_hz" => 0}]}, "frequency_span_hz") ==
             0.0

    assert Metrics.metric_value(%{"nodes" => [%{"ux" => -3}, %{"ux" => 2}]}, "max_displacement") ==
             3.0

    norm = Metrics.metric_value(%{"vx" => 1.0e308, "vy" => 1.0e308}, "velocity_magnitude")
    assert_in_delta norm / 1.0e308, :math.sqrt(2), 1.0e-14
  end

  test "peak ties follow the native last-sample rule" do
    assert Metrics.metric_value(
             %{
               "frequencies" => [
                 %{"frequency_hz" => 10, "u_peak" => 2},
                 %{"frequency_hz" => 20, "u_peak" => 2}
               ]
             },
             "peak_frequency_hz"
           ) == 20.0
  end

  defp guard(payload, field),
    do:
      Runtime.run_transform_operator(
        "transform.evaluate_structural_guard",
        payload,
        %{"rules" => [%{"field" => field, "threshold" => 1}]}
      )
end
