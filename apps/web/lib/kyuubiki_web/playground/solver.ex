defmodule KyuubikiWeb.Playground.Solver do
  @moduledoc """
  Small 1D axial-bar FEM solver used by the browser playground and orchestration API.
  """

  @spec solve(map()) :: {:ok, map()} | {:error, term()}
  def solve(params) when is_map(params) do
    with {:ok, length} <- positive_number(params, [:length, "length"]),
         {:ok, area} <- positive_number(params, [:area, "area"]),
         {:ok, youngs_modulus} <- positive_number(params, [:youngs_modulus, "youngs_modulus"]),
         {:ok, elements} <- positive_integer(params, [:elements, "elements"]),
         {:ok, tip_force} <- finite_number(params, [:tip_force, "tip_force"]) do
      {:ok, solve_bar(length, area, youngs_modulus, elements, tip_force)}
    end
  end

  defp solve_bar(length, area, youngs_modulus, elements, tip_force) do
    node_count = elements + 1
    element_length = length / elements
    stiffness = youngs_modulus * area / element_length
    global_stiffness = zero_matrix(node_count)
    force_vector = List.duplicate(0.0, node_count) |> List.replace_at(node_count - 1, tip_force)

    assembled =
      Enum.reduce(0..(elements - 1), global_stiffness, fn index, matrix ->
        matrix
        |> add_at(index, index, stiffness)
        |> add_at(index, index + 1, -stiffness)
        |> add_at(index + 1, index, -stiffness)
        |> add_at(index + 1, index + 1, stiffness)
      end)

    reduced_stiffness = assembled |> Enum.drop(1) |> Enum.map(&Enum.drop(&1, 1))
    reduced_force = Enum.drop(force_vector, 1)
    reduced_displacements = solve_linear_system(reduced_stiffness, reduced_force)
    displacements = [0.0 | reduced_displacements]

    nodes =
      Enum.with_index(displacements)
      |> Enum.map(fn {displacement, index} ->
        %{
          "index" => index,
          "x" => length * index / elements,
          "displacement" => displacement
        }
      end)

    elements_result =
      Enum.map(0..(elements - 1), fn index ->
        left = Enum.at(nodes, index)
        right = Enum.at(nodes, index + 1)
        strain = (right["displacement"] - left["displacement"]) / element_length
        stress = youngs_modulus * strain

        %{
          "index" => index,
          "x1" => left["x"],
          "x2" => right["x"],
          "strain" => strain,
          "stress" => stress,
          "axial_force" => stress * area
        }
      end)

    reaction_force =
      assembled
      |> Enum.at(0)
      |> Enum.zip(displacements)
      |> Enum.reduce(0.0, fn {stiffness_ij, displacement}, sum ->
        sum + stiffness_ij * displacement
      end)

    %{
      "input" => %{
        "length" => length,
        "area" => area,
        "youngs_modulus" => youngs_modulus,
        "elements" => elements,
        "tip_force" => tip_force
      },
      "nodes" => nodes,
      "elements" => elements_result,
      "tip_displacement" => List.last(displacements),
      "reaction_force" => reaction_force,
      "max_displacement" => Enum.max_by(nodes, &abs(&1["displacement"]))["displacement"] |> abs(),
      "max_stress" => Enum.max_by(elements_result, &abs(&1["stress"]))["stress"] |> abs()
    }
  end

  defp zero_matrix(size), do: Enum.map(1..size, fn _ -> List.duplicate(0.0, size) end)

  defp add_at(matrix, row, column, value) do
    List.update_at(matrix, row, fn current_row ->
      List.update_at(current_row, column, &(&1 + value))
    end)
  end

  defp solve_linear_system(matrix, vector) do
    size = length(vector)
    augmented = Enum.zip(matrix, vector) |> Enum.map(fn {row, value} -> row ++ [value] end)

    reduced =
      Enum.reduce(0..(size - 1), augmented, fn pivot, acc ->
        max_row =
          pivot..(size - 1)
          |> Enum.max_by(fn row -> abs(cell(acc, row, pivot)) end)

        acc
        |> swap_rows(pivot, max_row)
        |> normalize_row(pivot, size)
        |> eliminate_column(pivot, size)
      end)

    Enum.map(reduced, &Enum.at(&1, size))
  end

  defp swap_rows(matrix, row_a, row_b) when row_a == row_b, do: matrix

  defp swap_rows(matrix, row_a, row_b) do
    value_a = Enum.at(matrix, row_a)
    value_b = Enum.at(matrix, row_b)

    matrix
    |> List.replace_at(row_a, value_b)
    |> List.replace_at(row_b, value_a)
  end

  defp normalize_row(matrix, pivot, size) do
    pivot_value = cell(matrix, pivot, pivot)

    if abs(pivot_value) < 1.0e-12 do
      raise ArgumentError, "system is singular"
    end

    List.update_at(matrix, pivot, fn row ->
      Enum.map(0..size, fn column -> Enum.at(row, column) / pivot_value end)
    end)
  end

  defp eliminate_column(matrix, pivot, size) do
    pivot_row = Enum.at(matrix, pivot)

    Enum.with_index(matrix)
    |> Enum.map(fn
      {row, ^pivot} ->
        row

      {row, row_index} ->
        factor = Enum.at(row, pivot)

        Enum.map(0..size, fn column ->
          Enum.at(row, column) - factor * Enum.at(pivot_row, column)
        end)
        |> then(fn reduced_row -> {reduced_row, row_index} end)
    end)
    |> Enum.map(fn
      {row, _row_index} -> row
      row -> row
    end)
  end

  defp cell(matrix, row, column), do: matrix |> Enum.at(row) |> Enum.at(column)

  defp positive_number(params, keys) do
    with {:ok, value} <- finite_number(params, keys),
         true <- value > 0 do
      {:ok, value}
    else
      false -> {:error, {:invalid_positive_number, keys}}
      error -> error
    end
  end

  defp positive_integer(params, keys) do
    case fetch_number(params, keys) do
      {:ok, value} when value > 0 and trunc(value) == value -> {:ok, trunc(value)}
      _ -> {:error, {:invalid_positive_integer, keys}}
    end
  end

  defp finite_number(params, keys) do
    with {:ok, value} <- fetch_number(params, keys),
         true <- is_float(value) or is_integer(value) do
      {:ok, value * 1.0}
    else
      false -> {:error, {:invalid_number, keys}}
      error -> error
    end
  end

  defp fetch_number(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} -> cast_number(value)
      :error -> fetch_number(params, rest)
    end
  end

  defp fetch_number(_params, []), do: {:error, :missing}

  defp cast_number(value) when is_integer(value), do: {:ok, value}
  defp cast_number(value) when is_float(value), do: {:ok, value}

  defp cast_number(value) when is_binary(value) do
    case Float.parse(value) do
      {number, ""} -> {:ok, number}
      _ -> {:error, :invalid}
    end
  end

  defp cast_number(_value), do: {:error, :invalid}
end
