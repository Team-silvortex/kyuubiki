defmodule KyuubikiWeb.MixProject do
  use Mix.Project

  def project do
    [
      app: :kyuubiki_web,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      aliases: aliases()
    ]
  end

  def application do
    [
      extra_applications: [:logger, :ecto_sql],
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

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:ecto_sql, "~> 3.13"},
      {:ecto_sqlite3, "~> 0.17"},
      {:postgrex, "~> 0.20"},
      {:plug, "~> 1.19"},
      {:plug_cowboy, "~> 2.8"}
    ]
  end
end
