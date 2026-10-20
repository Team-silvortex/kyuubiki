defmodule KyuubikiWeb.TestSupport.BeamChild do
  @moduledoc false

  @startup_flags ~w(ERL_FLAGS ERL_AFLAGS ERL_ZFLAGS ELIXIR_ERL_OPTIONS)

  def port_env(overrides \\ %{}) do
    # The child's explicit --erl settings must not inherit a caller's VM flags.
    overrides
    |> Map.merge(Map.new(@startup_flags, &{&1, false}))
    |> Enum.map(fn {key, value} -> {String.to_charlist(key), port_value(value)} end)
  end

  defp port_value(false), do: false
  defp port_value(value) when is_binary(value), do: String.to_charlist(value)
end
