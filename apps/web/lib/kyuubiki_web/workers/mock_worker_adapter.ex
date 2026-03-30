defmodule KyuubikiWeb.Workers.MockWorkerAdapter do
  @moduledoc """
  Bridges the Elixir orchestration layer to the Rust mock worker CLI.
  """

  alias KyuubikiWeb.Jobs.{Job, Store}

  @worker_id "rust-cli"

  @type runner :: (Job.t(), keyword() -> {String.t(), integer()})

  @spec run_job(Job.t(), keyword()) :: {:ok, [map()]} | {:error, term()}
  def run_job(%Job{} = job, opts \\ []) do
    runner = Keyword.get(opts, :runner, &default_runner/2)

    with {:ok, _job} <- Store.assign_worker(job.job_id, @worker_id),
         {output, 0} <- runner.(job, opts),
         {:ok, events} <- parse_output(output),
         {:ok, applied_events} <- persist_events(events) do
      {:ok, applied_events}
    else
      {output, status} when is_integer(status) ->
        {:error, {:worker_command_failed, status, String.trim(output)}}

      error ->
        error
    end
  end

  defp default_runner(job, opts) do
    worker_dir = Keyword.get(opts, :worker_dir, worker_dir())

    args = [
      "run",
      "-p",
      "kyuubiki-cli",
      "--",
      "--job-id",
      job.job_id,
      "--project-id",
      job.project_id,
      "--case-id",
      job.simulation_case_id
    ]

    System.cmd("cargo", args, stderr_to_stdout: true, cd: worker_dir)
  end

  defp worker_dir do
    Path.expand("../../../../../workers/rust", __DIR__)
  end

  defp parse_output(output) when is_binary(output) do
    output
    |> String.split("\n", trim: true)
    |> Enum.reject(&(not String.starts_with?(&1, "event|")))
    |> Enum.reduce_while({:ok, []}, fn line, {:ok, events} ->
      case parse_line(line) do
        {:ok, event} -> {:cont, {:ok, events ++ [event]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  defp parse_line(line) do
    case String.split(String.trim(line), "|", parts: 8) do
      ["event", job_id, stage, progress, iteration, residual, peak_memory, message] ->
        {:ok,
         %{
           job_id: job_id,
           stage: stage,
           progress: parse_float(progress),
           iteration: parse_integer(iteration),
           residual: parse_float_or_nil(residual),
           peak_memory: parse_integer(peak_memory),
           message: parse_string(message)
         }}

      _ ->
        {:error, {:invalid_worker_output, line}}
    end
  end

  defp persist_events(events) do
    Enum.reduce_while(events, {:ok, []}, fn event, {:ok, applied_events} ->
      case Store.apply_progress(event) do
        {:ok, _job} -> {:cont, {:ok, applied_events ++ [event]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  defp parse_float(value) do
    {number, ""} = Float.parse(value)
    number
  end

  defp parse_float_or_nil(""), do: nil
  defp parse_float_or_nil(value), do: parse_float(value)

  defp parse_integer(""), do: nil

  defp parse_integer(value) do
    {number, ""} = Integer.parse(value)
    number
  end

  defp parse_string(value) do
    case String.trim(value) do
      "" -> nil
      trimmed -> trimmed
    end
  end
end
