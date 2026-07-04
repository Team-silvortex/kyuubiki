alias KyuubikiSdk.Auth
alias KyuubikiSdk.ControlPlaneClient

base_url = System.get_env("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")

auth =
  case System.get_env("KYUUBIKI_TOKEN") do
    nil -> nil
    token -> Auth.access_token(token)
  end

client = ControlPlaneClient.new(base_url, auth: auth)
request = KyuubikiSdk.material_study_envelope_catalog_request()

{:ok, job} =
  ControlPlaneClient.submit_workflow_catalog_job(
    client,
    request["workflow_id"],
    request["input_artifacts"]
  )

IO.puts(Jason.encode_to_iodata!(job, pretty: true))
