defmodule KyuubikiWeb.AxialBarInput do
  @moduledoc false

  @max_elements 4_294_967_295
  @max_finite 1.7976931348623157e308
  @unit_roundoff 8.881784197001252e-16
  @numeric_text ~r/\A-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?\z/

  def normalize(params) when is_map(params) do
    with {:ok, length} <- required_number(params, "length", :length),
         {:ok, area} <- required_number(params, "area", :area),
         {:ok, elements} <- required_number(params, "elements", :elements),
         {:ok, force} <- required_number(params, "tip_force", :tip_force),
         {:ok, pa} <- optional_number(params, "youngs_modulus", :youngs_modulus),
         {:ok, gpa} <- optional_number(params, "youngs_modulus_gpa", :youngs_modulus_gpa),
         {:ok, modulus} <- modulus(pa, gpa),
         true <- length > 0.0 and area > 0.0,
         true <- elements >= 1.0 and elements <= @max_elements and elements == trunc(elements) do
      {:ok,
       %{
         "length" => length,
         "area" => area,
         "elements" => trunc(elements),
         "tip_force" => force,
         "youngs_modulus" => modulus
       }}
    else
      _ -> {:error, :invalid_axial_bar_model}
    end
  end

  def normalize(_), do: {:error, :invalid_axial_bar_model}

  defp required_number(params, string_key, atom_key) do
    case optional_number(params, string_key, atom_key) do
      {:ok, nil} -> {:error, :missing_parameter}
      other -> other
    end
  end

  defp optional_number(params, string_key, atom_key) do
    case Map.fetch(params, string_key) do
      {:ok, value} ->
        number(value)

      :error ->
        case Map.fetch(params, atom_key) do
          {:ok, value} -> number(value)
          :error -> {:ok, nil}
        end
    end
  end

  defp number(value) when is_number(value) do
    converted = value * 1.0
    if abs(converted) <= @max_finite, do: {:ok, converted}, else: {:error, :invalid_parameter}
  rescue
    ArithmeticError -> {:error, :invalid_parameter}
  end

  defp number(value) when is_binary(value) and byte_size(value) <= 256 do
    with true <- Regex.match?(@numeric_text, value),
         {parsed, ""} <- Float.parse(value) do
      number(parsed)
    else
      _ -> {:error, :invalid_parameter}
    end
  rescue
    ArgumentError -> {:error, :invalid_parameter}
  end

  defp number(_), do: {:error, :invalid_parameter}

  defp modulus(nil, nil), do: {:error, :missing_parameter}
  defp modulus(pa, nil) when pa > 0.0, do: {:ok, pa}

  defp modulus(pa, gpa) when is_number(gpa) and gpa > 0.0 do
    with {:ok, scaled} <- number(gpa * 1.0e9),
         true <- scaled > 0.0,
         true <-
           is_nil(pa) or
             (pa > 0.0 and abs(pa - scaled) <= @unit_roundoff * max(abs(pa), abs(scaled))) do
      {:ok, scaled}
    else
      _ -> {:error, :invalid_parameter}
    end
  rescue
    ArithmeticError -> {:error, :invalid_parameter}
  end

  defp modulus(_, _), do: {:error, :invalid_parameter}
end
