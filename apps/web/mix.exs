defmodule KyuubikiWeb.MixProject do
  use Mix.Project

  def project do
    [
      app: :kyuubiki_web,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: [],
      aliases: aliases()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {KyuubikiWeb.Application, []}
    ]
  end

  def cli do
    [
      preferred_envs: [ci: :test]
    ]
  end

  defp aliases do
    [
      setup: ["compile"],
      ci: ["format --check-formatted", "test"]
    ]
  end
end
