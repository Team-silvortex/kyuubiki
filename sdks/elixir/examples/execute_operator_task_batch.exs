alias KyuubikiSdk.Auth
alias KyuubikiSdk.ControlPlaneClient

case System.argv() do
  [batch_path] ->
    base_url = System.get_env("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")

    auth =
      case System.get_env("KYUUBIKI_TOKEN") do
        nil -> nil
        token -> Auth.access_token(token)
      end

    batch = batch_path |> File.read!() |> Jason.decode!()
    client = ControlPlaneClient.new(base_url, auth: auth)
    {:ok, result} = ControlPlaneClient.execute_operator_task_batch(client, batch)
    IO.puts(Jason.encode_to_iodata!(result, pretty: true))

  _ ->
    IO.puts(:stderr, "usage: mix run examples/execute_operator_task_batch.exs batch.json")
    System.halt(64)
end
