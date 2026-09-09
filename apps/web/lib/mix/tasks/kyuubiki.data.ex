defmodule Mix.Tasks.Kyuubiki.Data do
  use Mix.Task
  @shortdoc "Inspect, snapshot, upgrade or restore SQLite data without starting Orchestra"
  alias KyuubikiWeb.Storage.SqliteLifecycle

  @impl true
  def run(args) do
    Mix.Task.run("loadpaths")
    Mix.Task.run("compile")
    Application.ensure_all_started(:exqlite)
    Application.ensure_all_started(:crypto)

    {opts, positional, invalid} =
      OptionParser.parse(args,
        strict: [
          out: [:string, :keep],
          expect_sha256: [:string, :keep],
          receipt: [:string, :keep]
        ]
      )

    if invalid != [] or length(opts) != length(Enum.uniq_by(opts, &elem(&1, 0))), do: usage!()

    result =
      case {positional, Enum.sort(Keyword.keys(opts))} do
        {["plan", source], []} ->
          SqliteLifecycle.plan!(source)

        {["backup", source], [:out]} ->
          SqliteLifecycle.backup!(source, opts[:out])

        {["upgrade", source], [:expect_sha256, :out]} ->
          SqliteLifecycle.upgrade!(source, opts[:out], opts[:expect_sha256])

        {["restore", source], [:expect_sha256, :out]} ->
          SqliteLifecycle.restore!(source, opts[:out], opts[:expect_sha256])

        {["verify", source], [:receipt]} ->
          SqliteLifecycle.verify!(source, read_receipt!(opts[:receipt]))

        _ ->
          usage!()
      end

    Mix.shell().info(Jason.encode!(result, pretty: true))
  rescue
    error -> Mix.raise(Exception.message(error))
  end

  defp read_receipt!(path) do
    case File.lstat(path) do
      {:ok, %{type: :regular, size: size}} when size <= 1024 * 1024 ->
        File.read!(path) |> Jason.decode!()

      _ ->
        Mix.raise("receipt must be a regular JSON file no larger than 1 MiB")
    end
  end

  defp usage! do
    Mix.raise(
      "usage: mix kyuubiki.data plan INPUT | backup INPUT --out NEW | upgrade/restore SNAPSHOT --out NEW --expect-sha256 DIGEST | verify INPUT --receipt JSON"
    )
  end
end
