defmodule KyuubikiWeb.WorkflowThermalDiagnostics do
  @moduledoc false

  alias KyuubikiWeb.WorkflowDiagnosticSamples, as: Samples

  def thermal(payload, config) do
    Samples.extract(
      payload,
      config,
      "thermal",
      ~w(temperature heat_load gradient flux),
      fn context, config ->
        context
        |> Samples.distribution(
          :nodes,
          "temperature",
          Samples.field(config, "temperature_field", ~w(temperature)),
          node_ids: true
        )
        |> Samples.distribution(
          :nodes,
          "heat_load",
          Samples.field(config, "heat_load_field", ~w(heat_load)),
          load_peak: true
        )
        |> Samples.alias_metric("heat_load_nonzero_count", "loaded_node_count")
        |> Samples.alias_metric("heat_load_sum", "total_heat_load")
        |> Samples.vector_peak("gradient", %{
          x: Samples.field(config, "gradient_x_field", ~w(temperature_gradient_x)),
          y: Samples.field(config, "gradient_y_field", ~w(temperature_gradient_y)),
          z: Samples.field(config, "gradient_z_field", []),
          magnitude: Samples.field(config, "gradient_magnitude_field", [])
        })
        |> Samples.vector_peak("flux", %{
          x: Samples.field(config, "flux_x_field", ~w(heat_flux_x)),
          y: Samples.field(config, "flux_y_field", ~w(heat_flux_y)),
          z: Samples.field(config, "flux_z_field", []),
          magnitude: Samples.field(config, "flux_magnitude_field", ~w(heat_flux_magnitude))
        })
      end
    )
  end

  def thermo(payload, config) do
    Samples.extract(
      payload,
      config,
      "thermo",
      ~w(temperature_delta displacement stress),
      fn context, config ->
        context =
          context
          |> Map.put(:scalar_id_fallback, "unknown")
          |> Map.update!(:summary, &Map.put(&1, "diagnostic_domain", "thermo_mechanical"))
          |> Samples.distribution(
            :nodes,
            "temperature_delta",
            Samples.field(
              config,
              ~w(temperature_delta_field temperature_field),
              ~w(temperature_delta)
            ),
            node_ids: true
          )
          |> Samples.alias_metric("temperature_delta_nonzero_count", "heated_node_count")
          |> Samples.vector_peak(
            "displacement",
            %{
              x: Samples.field(config, ~w(displacement_x_field ux_field), ~w(ux displacement_x)),
              y: Samples.field(config, ~w(displacement_y_field uy_field), ~w(uy displacement_y)),
              z: Samples.field(config, ~w(displacement_z_field uz_field), []),
              magnitude:
                Samples.field(
                  config,
                  ~w(displacement_magnitude_field displacement_field),
                  ~w(displacement_magnitude)
                )
            },
            :nodes
          )
          |> Samples.scalar_peak(
            "stress",
            Samples.field(config, "stress_field", ~w(von_mises_stress von_mises))
          )

        context =
          if Samples.has_metric?(context, "stress_peak"),
            do: context,
            else: Samples.payload_scalar_peak(context, payload, "stress", "max_stress")

        context =
          if Samples.has_metric?(context, "stress_peak"),
            do: context,
            else:
              Samples.component_peak(context, "stress", ~w(stress_x stress_y stress_z stress_xy))

        Enum.reduce(~w(thermal_strain mechanical_strain total_strain), context, fn name,
                                                                                   context ->
          context =
            Samples.scalar_peak(context, name, Samples.field(config, "#{name}_field", [name]))

          if Samples.has_metric?(context, "#{name}_peak") do
            context
          else
            Samples.component_peak(context, name, Enum.map(~w(x y z xy), &"#{name}_#{&1}"))
          end
        end)
      end
    )
  end
end
