alias KyuubikiSdk.MaterialResearchBundle

default_fixture =
  __DIR__
  |> Path.join("../../../schemas/examples.material-research-bundle.json")
  |> Path.expand()

fixture = System.argv() |> List.first() |> Kernel.||(default_fixture)

{:ok, bundle} =
  fixture
  |> File.read!()
  |> Jason.decode!()
  |> MaterialResearchBundle.validate()

IO.puts("schema=#{bundle["schema_version"]}")
IO.puts("study=#{bundle["study"]}")
IO.puts("winner=#{bundle["summary"]["winner_candidate_id"]}")
IO.puts("reliability=#{bundle["summary"]["reliability_decision"]}")
