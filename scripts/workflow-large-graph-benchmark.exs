alias KyuubikiWeb.TestSupport.WorkflowLargeGraphBenchmark

Mix.start()
Mix.env(:test)

Code.require_file(Path.expand("../apps/web/test/support/workflow_api_fixtures.exs", __DIR__))
Code.require_file(Path.expand("../apps/web/test/support/workflow_api_test_support.exs", __DIR__))
Code.require_file(Path.expand("../apps/web/test/support/workflow_large_graph_benchmark.exs", __DIR__))

config_path = Path.expand("../apps/web/config/config.exs", __DIR__)
{config, _imports} = Config.Reader.read_imports!(config_path, env: :test)
Application.put_all_env(config, persistent: true)
{:ok, _} = Application.ensure_all_started(:kyuubiki_web)

{options, positional, _invalid} =
  OptionParser.parse(System.argv(),
    strict: [output: :string],
    aliases: [o: :output]
  )

counts =
  case positional do
    [] ->
      WorkflowLargeGraphBenchmark.default_pass_through_counts()

    values ->
      Enum.map(values, fn value ->
        case Integer.parse(value) do
          {count, ""} when count > 0 -> count
          _ -> raise "invalid pass-through count: #{value}"
        end
      end)
  end

report = WorkflowLargeGraphBenchmark.benchmark_report(KyuubikiWeb.Router.init([]), counts)
json = Jason.encode_to_iodata!(report, pretty: true)

case Keyword.get(options, :output) do
  nil ->
    IO.binwrite(json)

  path ->
    File.write!(path, json)
    IO.puts("wrote workflow large graph benchmark report to #{path}")
  end
