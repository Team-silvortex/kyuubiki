defmodule KyuubikiWeb.WorkflowReportingChecks do
  @moduledoc false

  @max_u64 18_446_744_073_709_551_615

  # Only contract failures are translated; programming errors keep their own traces.
  def run(operator, work) do
    {:ok, work.()}
  catch
    {:workflow_reporting_invalid, reason} -> {:error, "#{operator}: #{reason}"}
  end

  def fail(reason), do: throw({:workflow_reporting_invalid, reason})

  def object(value, path) when is_map(value) do
    if Enum.all?(Map.keys(value), &is_binary/1),
      do: value,
      else: fail("#{path} must be an object with string keys")
  end

  def object(_value, path), do: fail("#{path} must be an object")
  def config(nil), do: %{}
  def config(value), do: object(value, "config")

  def list(value, _path) when is_list(value), do: value
  def list(_value, path), do: fail("#{path} must be an array")

  def text(object, key, path, default \\ nil) do
    case Map.fetch(object, key) do
      :error ->
        default

      {:ok, value} when is_binary(value) ->
        if String.trim(value) == "", do: fail("#{path}.#{key} must be a nonblank string")
        String.trim(value)

      _ ->
        fail("#{path}.#{key} must be a nonblank string")
    end
  end

  def boolean(config, key, default) do
    case Map.fetch(config, key) do
      :error -> default
      {:ok, value} when is_boolean(value) -> value
      _ -> fail("config.#{key} must be a boolean")
    end
  end

  def number(value, path) when is_number(value), do: calculate(path, fn -> value * 1.0 end)
  def number(_value, path), do: fail("#{path} must be a finite number")

  def optional_number(object, key, path) do
    case Map.fetch(object, key) do
      :error -> nil
      {:ok, value} -> number(value, "#{path}.#{key}")
    end
  end

  def unsigned(value, _path) when is_integer(value) and value >= 0 and value <= @max_u64,
    do: value

  def unsigned(_value, path), do: fail("#{path} must be a nonnegative u64 integer")

  # BEAM raises on floating-point overflow rather than returning IEEE infinity.
  def calculate(path, work) do
    work.()
  rescue
    ArithmeticError -> fail("#{path} produced a non-finite value")
  end

  def converged(object, path) do
    marker(object, path)

    case Map.fetch(object, "stability_result") do
      :error ->
        :ok

      {:ok, value} ->
        value |> object("#{path}.stability_result") |> marker("#{path}.stability_result")
    end
  end

  defp marker(object, path) do
    case Map.fetch(object, "converged") do
      :error -> :ok
      {:ok, true} -> :ok
      {:ok, false} -> fail("rejects nonconverged result: #{path}.converged=false")
      _ -> fail("#{path}.converged must be a boolean")
    end
  end

  def choice(config, key, default, choices, path \\ "config") do
    selected = text(config, key, path, default)

    if selected not in choices,
      do: fail("#{path}.#{key} must be one of #{Enum.join(choices, ", ")}")

    selected
  end

  def hypot(left, right, path) do
    {large, small} = {max(abs(left), abs(right)), min(abs(left), abs(right))}

    if large == 0 do
      0.0
    else
      calculate(path, fn ->
        ratio = small / large
        large * :math.sqrt(1.0 + ratio * ratio)
      end)
    end
  end
end
