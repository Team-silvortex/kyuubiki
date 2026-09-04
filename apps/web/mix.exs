defmodule KyuubikiWeb.MixProject do
  use Mix.Project

  def project do
    [
      app: :kyuubiki_web,
      version: System.get_env("KYUUBIKI_RELEASE_VERSION", "2.20.1"),
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      test_ignore_filters: [~r{^test/support/}],
      deps: deps(),
      aliases: aliases(),
      releases: releases()
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

  defp releases do
    [
      kyuubiki_web: [
        include_executables_for: [:unix, :windows]
      ]
    ]
  end

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:ecto_sql, "~> 3.13"},
      {:ecto_sqlite3, "~> 0.17"},
      {:postgrex, "~> 0.20"},
      {:plug, "~> 1.19"},
      {:bandit, "~> 1.12.5"}
    ]
  end
end
