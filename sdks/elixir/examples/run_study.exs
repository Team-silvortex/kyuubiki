alias KyuubikiSdk.AgentClient
alias KyuubikiSdk.Auth
alias KyuubikiSdk

base_url = System.get_env("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")

auth =
  case System.get_env("KYUUBIKI_TOKEN") do
    nil -> nil
    token -> Auth.access_token(token)
  end

payload = %{
  "nodes" => [
    %{
      "id" => "n0",
      "x" => 0.0,
      "y" => 0.0,
      "fix_x" => true,
      "fix_y" => true,
      "load_x" => 0.0,
      "load_y" => 0.0
    },
    %{
      "id" => "n1",
      "x" => 1.0,
      "y" => 0.0,
      "fix_x" => false,
      "fix_y" => true,
      "load_x" => 0.0,
      "load_y" => 0.0
    },
    %{
      "id" => "n2",
      "x" => 0.5,
      "y" => 0.75,
      "fix_x" => false,
      "fix_y" => false,
      "load_x" => 0.0,
      "load_y" => -1000.0
    }
  ],
  "elements" => [
    %{"id" => "e0", "node_i" => 0, "node_j" => 1, "area" => 0.01, "youngs_modulus" => 7.0e10},
    %{"id" => "e1", "node_i" => 1, "node_j" => 2, "area" => 0.01, "youngs_modulus" => 7.0e10},
    %{"id" => "e2", "node_i" => 2, "node_j" => 0, "area" => 0.01, "youngs_modulus" => 7.0e10}
  ]
}

session = Kyuubiki.new_session(base_url: base_url, auth: auth)

{:ok, outcome} = AgentClient.run_study(session, "truss_2d", payload, timeout: 60_000)
IO.puts("terminal:")
IO.puts(Jason.encode_to_iodata!(outcome.terminal, pretty: true))

job_id = get_in(outcome, [:terminal, "job", "job_id"])

{:ok, first_page} =
  AgentClient.browse_result_chunks(session, job_id, "nodes", offset: 0, limit: 2)

IO.puts("\nfirst nodes page:")
IO.puts(Jason.encode_to_iodata!(first_page, pretty: true))

IO.puts("\niterating element pages:")

session
|> AgentClient.stream_result_chunks(job_id, "elements", page_size: 2)
|> Enum.each(fn page ->
  IO.puts(Jason.encode_to_iodata!(page, pretty: true))
end)
