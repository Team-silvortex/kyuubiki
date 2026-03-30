defmodule KyuubikiWeb.Persistence do
  @moduledoc false

  @default_dir Path.expand("../../../../tmp/data", __DIR__)

  def data_dir do
    System.get_env("KYUUBIKI_DATA_DIR", @default_dir)
  end

  def jobs_path do
    Path.join(data_dir(), "jobs.json")
  end

  def results_path do
    Path.join(data_dir(), "results.json")
  end

  def ensure_dir! do
    File.mkdir_p!(data_dir())
  end

  def write_json!(path, payload) do
    ensure_dir!()
    tmp_path = "#{path}.tmp"
    File.write!(tmp_path, Jason.encode!(payload))
    File.rename!(tmp_path, path)
  end

  def read_json(path, default) do
    case File.read(path) do
      {:ok, contents} ->
        case Jason.decode(contents) do
          {:ok, decoded} -> decoded
          {:error, _reason} -> default
        end

      {:error, _reason} ->
        default
    end
  end

  def clear! do
    File.rm_rf!(data_dir())
    :ok
  end
end
