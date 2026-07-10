defmodule KyuubikiSdk.MaterialReportCatalog do
  @moduledoc false

  @study_order ~w(
    material_heat_spreader_screening
    material_dielectric_screening
    material_thermo_shield_screening
    material_structural_panel_screening
    material_composite_thermo_electric_panel
  )

  @study_aliases %{
    "material_heat_spreader_screening" => "material.heat_spreader_screening.v1",
    "material_dielectric_screening" => "material.dielectric_screening.v1",
    "material_thermo_shield_screening" => "material.thermo_shield_screening.v1",
    "material_structural_panel_screening" => "material.structural_panel_screening.v1",
    "material_composite_thermo_electric_panel" => "material.composite_thermo_electric_panel.v1"
  }

  @descriptors %{
    "material_heat_spreader_screening" => %{
      "id" => "material_heat_spreader_screening",
      "title" => "Heat Spreader Material Screening",
      "domain" => "thermal",
      "objective" =>
        "rank heat spreader candidates by peak temperature, mass, and conductivity efficiency",
      "aliases" => ["heat-spreader", "heat_spreader", "material.heat_spreader_screening.v1"],
      "schema_version" => "kyuubiki.material-research-report/v1",
      "template_id" => "material_heat_spreader_screening"
    },
    "material_dielectric_screening" => %{
      "id" => "material_dielectric_screening",
      "title" => "Dielectric Material Screening",
      "domain" => "electromagnetic",
      "objective" =>
        "rank dielectric candidates by electric-field margin, low loss, and low mass",
      "aliases" => [
        "dielectric-screening",
        "dielectric_screening",
        "material.dielectric_screening.v1"
      ],
      "schema_version" => "kyuubiki.dielectric-material-report/v1",
      "template_id" => "material_dielectric_screening"
    },
    "material_thermo_shield_screening" => %{
      "id" => "material_thermo_shield_screening",
      "title" => "Thermo-Mechanical Shield Screening",
      "domain" => "thermo_mechanical",
      "objective" =>
        "rank thermo-mechanical shield candidates by stress, displacement, thermal expansion, and mass",
      "aliases" => ["thermo-shield", "thermo_shield", "material.thermo_shield_screening.v1"],
      "schema_version" => "kyuubiki.thermo-material-report/v1",
      "template_id" => "material_thermo_shield_screening"
    },
    "material_structural_panel_screening" => %{
      "id" => "material_structural_panel_screening",
      "title" => "Structural Panel Material Screening",
      "domain" => "structural",
      "objective" =>
        "rank structural panel candidates by stress, deflection, mass, stiffness, and yield margin",
      "aliases" => [
        "structural-panel",
        "structural_panel",
        "material.structural_panel_screening.v1"
      ],
      "schema_version" => "kyuubiki.structural-material-report/v1",
      "template_id" => "material_structural_panel_screening"
    },
    "material_composite_thermo_electric_panel" => %{
      "id" => "material_composite_thermo_electric_panel",
      "title" => "Composite Thermo-Electric Panel",
      "domain" => "multiphysics_materials",
      "objective" =>
        "rank mixed-material panel stacks across electric field, heat, thermal stress, interface risk, and mass",
      "aliases" => [
        "composite-thermo-electric-panel",
        "composite_thermo_electric_panel",
        "material.composite_thermo_electric_panel.v1"
      ],
      "schema_version" => "kyuubiki.composite-panel-report/v1",
      "template_id" => "material_composite_thermo_electric_panel"
    }
  }

  def study_order, do: @study_order
  def study_alias(study_id), do: @study_aliases[study_id]
  def descriptor(study_id), do: @descriptors[study_id]

  def catalog_entry(study_id),
    do: Map.put(descriptor(study_id), "metric_specs", metric_specs(study_id))

  def metric_specs("material_heat_spreader_screening") do
    [
      metric(
        "peak_temperature_c",
        "Peak temperature",
        "C",
        "minimize",
        0.45,
        "solver.result.max_temperature"
      ),
      metric(
        "peak_heat_flux_w_m2",
        "Peak heat flux",
        "W/m^2",
        "observe",
        0.0,
        "solver.result.max_heat_flux"
      ),
      metric(
        "areal_mass_kg_m2",
        "Areal mass",
        "kg/m^2",
        "minimize",
        0.25,
        "candidate.density_kg_m3 * model.thickness"
      ),
      metric(
        "conductivity_density_ratio",
        "Conductivity-density ratio",
        "W*m^2/(kg*K)",
        "maximize",
        0.3,
        "candidate.thermal_conductivity_w_mk / candidate.density_kg_m3"
      )
    ]
  end

  def metric_specs("material_dielectric_screening") do
    [
      metric(
        "max_electric_field_v_m",
        "Max electric field",
        "V/m",
        "minimize",
        0.25,
        "solver.result.max_electric_field"
      ),
      metric(
        "breakdown_safety_factor",
        "Breakdown safety factor",
        "ratio",
        "maximize",
        0.4,
        "candidate.breakdown_field_v_m / solver.result.max_electric_field"
      ),
      metric(
        "dielectric_loss_proxy",
        "Dielectric loss proxy",
        "relative",
        "minimize",
        0.2,
        "candidate.relative_permittivity * candidate.dissipation_factor"
      ),
      metric(
        "areal_mass_kg_m2",
        "Areal mass",
        "kg/m^2",
        "minimize",
        0.15,
        "candidate.density_kg_m3 * model.thickness"
      ),
      metric(
        "max_flux_density_c_m2",
        "Max electric flux density",
        "C/m^2",
        "observe",
        0.0,
        "solver.result.max_flux_density"
      )
    ]
  end

  def metric_specs("material_thermo_shield_screening") do
    [
      metric(
        "max_stress_pa",
        "Max von Mises stress",
        "Pa",
        "minimize",
        0.45,
        "solver.result.max_stress"
      ),
      metric(
        "max_displacement_m",
        "Max displacement",
        "m",
        "minimize",
        0.25,
        "solver.result.max_displacement"
      ),
      metric(
        "areal_mass_kg_m2",
        "Areal mass",
        "kg/m^2",
        "minimize",
        0.25,
        "candidate.density_kg_m3 * model.thickness"
      ),
      metric(
        "max_temperature_delta_k",
        "Max temperature delta",
        "K",
        "observe",
        0.0,
        "solver.result.max_temperature_delta"
      ),
      metric(
        "thermal_expansion_1_k",
        "Thermal expansion",
        "1/K",
        "minimize",
        0.05,
        "candidate.thermal_expansion_1_k"
      )
    ]
  end

  def metric_specs("material_structural_panel_screening") do
    [
      metric(
        "max_stress_pa",
        "Max equivalent stress",
        "Pa",
        "minimize",
        0.3,
        "solver.result.max_stress"
      ),
      metric(
        "max_displacement_m",
        "Max displacement",
        "m",
        "minimize",
        0.25,
        "solver.result.max_displacement"
      ),
      metric(
        "areal_mass_kg_m2",
        "Areal mass",
        "kg/m^2",
        "minimize",
        0.2,
        "candidate.density_kg_m3 * model.thickness"
      ),
      metric(
        "specific_stiffness_m2_s2",
        "Specific stiffness",
        "m^2/s^2",
        "maximize",
        0.15,
        "candidate.youngs_modulus_pa / candidate.density_kg_m3"
      ),
      metric(
        "yield_safety_factor",
        "Yield safety factor",
        "ratio",
        "maximize",
        0.1,
        "candidate.yield_strength_pa / solver.result.max_stress"
      )
    ]
  end

  def metric_specs("material_composite_thermo_electric_panel") do
    [
      metric(
        "max_electric_field_v_m",
        "Max electric field",
        "V/m",
        "minimize",
        0.23,
        "electrostatic.max_electric_field"
      ),
      metric(
        "max_temperature_c",
        "Max temperature",
        "C",
        "minimize",
        0.23,
        "heat.max_temperature"
      ),
      metric(
        "max_thermal_stress_pa",
        "Max thermal stress",
        "Pa",
        "minimize",
        0.22,
        "thermal.max_stress"
      ),
      metric(
        "breakdown_safety_factor",
        "Breakdown safety factor",
        "ratio",
        "maximize",
        0.15,
        "candidate.breakdown_field / electrostatic.max_electric_field"
      ),
      metric(
        "interface_risk_score",
        "Interface risk score",
        "0..1",
        "minimize",
        0.12,
        "candidate.interface_compatibility"
      ),
      metric(
        "areal_mass_kg_m2",
        "Areal mass",
        "kg/m^2",
        "minimize",
        0.05,
        "candidate.stack_areal_mass"
      )
    ]
  end

  def candidates("material_heat_spreader_screening") do
    [
      %{
        "id" => "aluminum_6061",
        "label" => "Aluminum 6061",
        "density_kg_m3" => 2700.0,
        "thermal_conductivity_w_mk" => 167.0
      },
      %{
        "id" => "copper_c110",
        "label" => "Copper C110",
        "density_kg_m3" => 8960.0,
        "thermal_conductivity_w_mk" => 385.0
      },
      %{
        "id" => "pyrolytic_graphite_in_plane",
        "label" => "Pyrolytic graphite, in-plane",
        "density_kg_m3" => 2200.0,
        "thermal_conductivity_w_mk" => 1500.0
      }
    ]
  end

  def candidates("material_dielectric_screening") do
    [
      %{
        "id" => "polyimide_film",
        "label" => "Polyimide film",
        "relative_permittivity" => 3.4,
        "breakdown_field_v_m" => 300.0e6,
        "dissipation_factor" => 0.002,
        "density_kg_m3" => 1420.0
      },
      %{
        "id" => "alumina_96",
        "label" => "Alumina 96%",
        "relative_permittivity" => 9.8,
        "breakdown_field_v_m" => 130.0e6,
        "dissipation_factor" => 0.0002,
        "density_kg_m3" => 3720.0
      },
      %{
        "id" => "ptfe",
        "label" => "PTFE",
        "relative_permittivity" => 2.1,
        "breakdown_field_v_m" => 60.0e6,
        "dissipation_factor" => 0.0002,
        "density_kg_m3" => 2200.0
      }
    ]
  end

  def candidates("material_thermo_shield_screening") do
    [
      %{
        "id" => "aluminum_6061_t6",
        "label" => "Aluminum 6061-T6",
        "density_kg_m3" => 2700.0,
        "thermal_expansion_1_k" => 23.6e-6
      },
      %{
        "id" => "titanium_grade_5",
        "label" => "Titanium Grade 5",
        "density_kg_m3" => 4430.0,
        "thermal_expansion_1_k" => 8.6e-6
      },
      %{
        "id" => "invar_36",
        "label" => "Invar 36",
        "density_kg_m3" => 8050.0,
        "thermal_expansion_1_k" => 1.2e-6
      }
    ]
  end

  def candidates("material_structural_panel_screening") do
    [
      %{
        "id" => "aluminum_7075_t6",
        "label" => "Aluminum 7075-T6",
        "density_kg_m3" => 2810.0,
        "youngs_modulus_pa" => 71.7e9,
        "yield_strength_pa" => 503.0e6
      },
      %{
        "id" => "steel_4130_normalized",
        "label" => "Steel 4130 normalized",
        "density_kg_m3" => 7850.0,
        "youngs_modulus_pa" => 205.0e9,
        "yield_strength_pa" => 435.0e6
      },
      %{
        "id" => "carbon_fiber_quasi_iso",
        "label" => "Carbon fiber quasi-isotropic",
        "density_kg_m3" => 1600.0,
        "youngs_modulus_pa" => 70.0e9,
        "yield_strength_pa" => 600.0e6
      }
    ]
  end

  def candidates("material_composite_thermo_electric_panel") do
    [
      %{
        "id" => "copper_polyimide_aluminum",
        "label" => "Copper / Polyimide / Aluminum",
        "breakdown_field_v_m" => 300.0e6,
        "areal_mass_kg_m2" => 2.85,
        "interface_risk_score" => 0.63
      },
      %{
        "id" => "aluminum_alumina_aluminum",
        "label" => "Aluminum / Alumina / Aluminum",
        "breakdown_field_v_m" => 130.0e6,
        "areal_mass_kg_m2" => 3.7,
        "interface_risk_score" => 0.42
      },
      %{
        "id" => "copper_ptfe_glass_epoxy",
        "label" => "Copper / PTFE / Glass epoxy",
        "breakdown_field_v_m" => 60.0e6,
        "areal_mass_kg_m2" => 2.25,
        "interface_risk_score" => 0.91
      }
    ]
  end

  defp metric(id, label, unit, direction, default_weight, source) do
    %{
      "id" => id,
      "label" => label,
      "unit" => unit,
      "direction" => direction,
      "default_weight" => default_weight,
      "source" => source
    }
  end
end
