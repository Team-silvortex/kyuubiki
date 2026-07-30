defmodule KyuubikiWeb.WorkflowDomainMetricResolverTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowDomainMetricResolver

  test "direct metric values take precedence over aliases" do
    payload = %{
      "max_stress" => 4.0,
      "peak_stress" => 8.0,
      "max_displacement" => 0.25,
      "peak_displacement" => 0.75
    }

    assert_metric(payload, "max_stress", 4.0)
    assert_metric(payload, "max_displacement", 0.25)
  end

  test "resolves dynamic frequency and transient summaries" do
    harmonic_payload = %{
      "frequencies" => [
        %{"frequency" => 10.0, "displacement_amplitude" => 0.2, "velocity_amplitude" => 1.0},
        %{"freq_hz" => 22.0, "u_peak" => 0.8, "v_peak" => 1.7},
        %{
          "response_frequency_hz" => 35.0,
          "peak_displacement" => 0.3,
          "peak_velocity" => 1.2
        }
      ]
    }

    transient_payload = %{
      "nodes" => [
        %{"ux" => -0.1, "vx" => 0.5, "ax" => 1.0},
        %{"ux" => 0.4, "vx" => -1.5, "ax" => -3.0}
      ]
    }

    assert_metric(harmonic_payload, "peak_frequency_hz", 22.0)
    assert_metric(harmonic_payload, "max_velocity", 1.7)
    assert_metric(transient_payload, "max_displacement", 0.4)
    assert_metric(transient_payload, "max_acceleration", 3.0)
  end

  test "resolves domain aliases and bounds-derived spans" do
    payload = %{
      "temperature_max" => 320.0,
      "temperature_min" => 280.0,
      "voltage_max" => 12.0,
      "voltage_min" => -3.0,
      "concentration_max" => 0.9,
      "concentration_min" => 0.2,
      "total_thermal_energy" => 42.0,
      "peak_flux_density" => 6.0,
      "electric_total_energy" => 2.0,
      "magnetic_total_energy" => 1.5,
      "current_density_total" => 2.5
    }

    assert_metric(payload, "thermo_temperature_delta_max", 40.0)
    assert_metric(payload, "electrostatic_potential_span", 15.0)
    assert_metric(payload, "transport_concentration_span", 0.7)
    assert_metric(payload, "thermal_total_energy", 42.0)
    assert_metric(payload, "electrostatic_flux_peak_magnitude", 6.0)
    assert_metric(payload, "electrostatic_total_stored_energy", 2.0)
    assert_metric(payload, "magnetostatic_total_stored_energy", 1.5)
    assert_metric(payload, "magnetostatic_current_density_sum", 2.5)
  end

  test "resolves modal modes, CFD values, and generic spans" do
    modal_payload = %{
      "modes" => [
        %{"frequency_hz" => 12.0, "participation_norm" => 1.25},
        %{"frequency_hz" => 48.0, "participation_norm" => 0.5}
      ]
    }

    cfd_payload = %{
      "vx" => 3.0,
      "vy" => 4.0,
      "p" => 9.0,
      "div_u" => 0.04,
      "re" => 120.0,
      "nu_dissipation" => 0.75,
      "cfd_pressure_min" => 2.0,
      "cfd_pressure_max" => 11.0
    }

    assert_metric(modal_payload, "min_frequency_hz", 12.0)
    assert_metric(modal_payload, "max_frequency_hz", 48.0)
    assert_metric(modal_payload, "frequency_span_hz", 36.0)
    assert_metric(modal_payload, "mode_1_participation_norm", 1.25)
    assert_metric(cfd_payload, "velocity_magnitude", 5.0)
    assert_metric(cfd_payload, "pressure", 9.0)
    assert_metric(cfd_payload, "divergence_error", 0.04)
    assert_metric(cfd_payload, "reynolds_number", 120.0)
    assert_metric(cfd_payload, "viscous_dissipation", 0.75)
    assert_metric(cfd_payload, "cfd_pressure_span", 9.0)
  end

  defp assert_metric(payload, field, expected) do
    assert_in_delta WorkflowDomainMetricResolver.metric_value(payload, field), expected, 1.0e-9
  end
end
