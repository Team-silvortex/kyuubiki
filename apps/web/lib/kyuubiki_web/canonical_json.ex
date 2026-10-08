defmodule KyuubikiWeb.CanonicalJson do
  @moduledoc false
  import Bitwise

  @decimal_scale 1_000_000_000_000_000

  @spec encode!(term()) :: String.t()
  def encode!(value) when is_map(value) do
    entries =
      value
      |> Map.to_list()
      |> Enum.sort_by(fn {key, _value} -> to_string(key) end)
      |> Enum.map(fn {key, nested} -> Jason.encode!(to_string(key)) <> ":" <> encode!(nested) end)

    "{" <> Enum.join(entries, ",") <> "}"
  end

  def encode!(values) when is_list(values) do
    "[" <> (values |> Enum.map(&encode!/1) |> Enum.join(",")) <> "]"
  end

  def encode!(value) when is_binary(value), do: Jason.encode!(value)
  def encode!(value) when is_integer(value), do: Integer.to_string(value)

  def encode!(value) when is_float(value) do
    # Match Rust's fixed-15, ties-to-even rounding on the exact IEEE value.
    # The runtime formatter can round the last digit differently for solver outputs.
    <<sign::1, exponent::11, fraction::52>> = <<value::float-64>>

    {mantissa, power} =
      if exponent == 0, do: {fraction, -1074}, else: {fraction + (1 <<< 52), exponent - 1075}

    numerator = mantissa * @decimal_scale

    scaled =
      if power >= 0 do
        numerator <<< power
      else
        divisor = 1 <<< -power
        quotient = div(numerator, divisor)
        remainder = rem(numerator, divisor)

        if remainder * 2 > divisor or (remainder * 2 == divisor and rem(quotient, 2) == 1),
          do: quotient + 1,
          else: quotient
      end

    whole = Integer.to_string(div(scaled, @decimal_scale))

    decimals =
      scaled
      |> rem(@decimal_scale)
      |> Integer.to_string()
      |> String.pad_leading(15, "0")
      |> String.trim_trailing("0")

    prefix = if sign == 1, do: "-", else: ""
    prefix <> whole <> "." <> if(decimals == "", do: "0", else: decimals)
  end

  def encode!(value) when is_boolean(value), do: Jason.encode!(value)
  def encode!(nil), do: "null"
end
