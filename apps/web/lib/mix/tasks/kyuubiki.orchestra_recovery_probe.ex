defmodule Mix.Tasks.Kyuubiki.OrchestraRecoveryProbe do
  use Mix.Task

  alias KyuubikiWeb.Orchestra.DistributedRecoveryProbe

  @shortdoc "Runs Orchestra process-loss fault injection and writes JSON evidence"

  @impl Mix.Task
  def run(args) do
    {opts, rest, invalid} = OptionParser.parse(args, strict: [out: :string])

    if rest != [] or invalid != [] or not is_binary(opts[:out]) do
      Mix.raise("usage: mix kyuubiki.orchestra_recovery_probe --out <path>")
    end

    Mix.Task.run("app.start")
    report = DistributedRecoveryProbe.run!()
    path = Path.expand(opts[:out])
    File.mkdir_p!(Path.dirname(path))
    File.write!(path, Jason.encode_to_iodata!(report, pretty: true))
    Mix.shell().info("Orchestra process-loss report written: #{path}")
  end
end
