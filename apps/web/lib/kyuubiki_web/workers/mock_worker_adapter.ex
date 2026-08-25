defmodule KyuubikiWeb.Workers.MockWorkerAdapter do
  @moduledoc """
  Bridges the Elixir orchestration layer to the Rust mock worker CLI.
  """

  alias KyuubikiWeb.Jobs.{Job, ProgressEvent, Store}

  @worker_id "rust-cli"

  @type runner :: (Job.t(), keyword() -> {String.t(), integer()})

  @spec run_job(Job.t(), keyword()) :: {:ok, [map()]} | {:error, term()}
  def run_job(%Job{} = job, opts \\ []) do
    if adapter_enabled?() do
      runner = Keyword.get(opts, :runner, &default_runner/2)

      with {output, 0} <- runner.(job, opts),
           {:ok, events} <- parse_output(output, job.job_id),
           {:ok, _job} <- Store.assign_worker(job.job_id, @worker_id),
           {:ok, applied_events} <- persist_events(events) do
        {:ok, applied_events}
      else
        {output, status} when is_integer(status) ->
          {:error, {:worker_command_failed, status, String.trim(output)}}

        error ->
          error
      end
    else
      {:error, :transitional_worker_adapter_disabled}
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

  defp adapter_enabled? do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:enabled?, false)
  end

  defp worker_dir do
    Path.expand("../../../../../workers/rust", __DIR__)
  end

  defp parse_output(output, expected_job_id)
       when is_binary(output) and is_binary(expected_job_id) do
    event_lines =
      output
      |> String.split("\n", trim: true)
      |> Enum.filter(&String.starts_with?(&1, "event|"))

    with :ok <- require_event_lines(event_lines),
         {:ok, reversed_events} <- parse_event_lines(event_lines, expected_job_id),
         events = Enum.reverse(reversed_events),
         :ok <- validate_event_sequence(events) do
      {:ok, events}
    end
  end

  defp require_event_lines([]), do: {:error, :worker_output_missing_progress}
  defp require_event_lines(_event_lines), do: :ok

  defp parse_event_lines(lines, expected_job_id) do
    Enum.reduce_while(lines, {:ok, []}, fn line, {:ok, events} ->
      case parse_line(line, expected_job_id) do
        {:ok, event} -> {:cont, {:ok, [event | events]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  defp parse_line(line, expected_job_id) do
    case String.split(String.trim(line), "|", parts: 8) do
      ["event", job_id, stage, progress, iteration, residual, peak_memory, message] ->
        with :ok <- validate_job_id(job_id, expected_job_id),
             {:ok, progress} <- parse_progress(progress),
             {:ok, iteration} <- parse_optional_integer(iteration),
             {:ok, residual} <- parse_optional_non_negative_float(residual),
             {:ok, peak_memory} <- parse_optional_integer(peak_memory) do
          event = %{
            job_id: job_id,
            stage: stage,
            progress: progress,
            iteration: iteration,
            residual: residual,
            peak_memory: peak_memory,
            message: parse_string(message)
          }

          case ProgressEvent.new(event) do
            {:ok, _validated} -> {:ok, event}
            {:error, reason} -> {:error, {:invalid_worker_event, reason}}
          end
        else
          {:error, {:worker_job_mismatch, _, _} = reason} -> {:error, reason}
          {:error, reason} -> {:error, {:invalid_worker_output, reason, line}}
        end

      _ ->
        {:error, {:invalid_worker_output, line}}
    end
  end

  defp validate_job_id(job_id, job_id), do: :ok

  defp validate_job_id(job_id, expected_job_id),
    do: {:error, {:worker_job_mismatch, expected_job_id, job_id}}

  defp validate_event_sequence(events) do
    with :ok <- validate_monotonic_progress(events),
         %{stage: stage} <- List.last(events),
         true <- stage in ["completed", "failed", "cancelled"] do
      :ok
    else
      false -> {:error, :worker_output_missing_terminal_event}
      _ -> {:error, :worker_output_missing_terminal_event}
    end
  end

  defp validate_monotonic_progress(events) do
    events
    |> Enum.reduce_while(-1.0, fn event, previous ->
      if event.progress < previous,
        do: {:halt, {:error, {:worker_progress_regression, previous, event.progress}}},
        else: {:cont, event.progress}
    end)
    |> case do
      {:error, _reason} = error -> error
      _progress -> :ok
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

  defp parse_progress(value) do
    case Float.parse(value) do
      {number, ""} when number >= 0.0 and number <= 1.0 -> {:ok, number}
      _ -> {:error, {:invalid_progress, value}}
    end
  end

  defp parse_optional_non_negative_float(""), do: {:ok, nil}

  defp parse_optional_non_negative_float(value) do
    case Float.parse(value) do
      {number, ""} when number >= 0.0 -> {:ok, number}
      _ -> {:error, {:invalid_non_negative_float, value}}
    end
  end

  defp parse_optional_integer(""), do: {:ok, nil}

  defp parse_optional_integer(value) do
    case Integer.parse(value) do
      {number, ""} when number >= 0 -> {:ok, number}
      _ -> {:error, {:invalid_non_negative_integer, value}}
    end
  end

  defp parse_string(value) do
    case String.trim(value) do
      "" -> nil
      trimmed -> trimmed
    end
  end
end
