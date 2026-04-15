defmodule KyuubikiSdk.MixProject do
  use Mix.Project

  def project do
    [
      app: :kyuubiki_sdk,
      version: "0.1.0",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger, :inets, :ssl, :public_key]
    ]
  end

  defp deps do
    [
      {:jason, "~> 1.4"}
    ]
  end
end
