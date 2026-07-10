alias KyuubikiSdk.MaterialReports

{:ok, report} =
  MaterialReports.build_material_report("composite-thermo-electric-panel", [
    %{
      "electrostatic" => %{"max_electric_field" => 45.0e6},
      "heat" => %{"max_temperature" => 120.0},
      "thermal" => %{"max_stress" => 180.0e6}
    },
    %{
      "electrostatic" => %{"max_electric_field" => 52.0e6},
      "heat" => %{"max_temperature" => 98.0},
      "thermal" => %{"max_stress" => 140.0e6}
    },
    %{
      "electrostatic" => %{"max_electric_field" => 58.0e6},
      "heat" => %{"max_temperature" => 132.0},
      "thermal" => %{"max_stress" => 210.0e6}
    }
  ])

IO.puts("study=#{report["study"]}")
IO.puts("winner=#{report["winner_candidate_id"]}")
IO.puts("reliability=#{report["reliability"]["summary"]["decision"]}")
