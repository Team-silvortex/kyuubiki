import Config

config :logger, :default_formatter,
  format: "$time $metadata[$level] $message\n",
  metadata: [:job_id, :stage]

config :kyuubiki_web, start_http_server: Mix.env() != :test

if Mix.env() == :test do
  config :logger, level: :warning
end
