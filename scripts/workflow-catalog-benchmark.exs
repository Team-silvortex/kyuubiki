alias KyuubikiWeb.TestSupport.WorkflowCatalogBenchmark

Mix.start()
Mix.env(:test)

Code.require_file(Path.expand("../apps/web/test/support/workflow_api_fixtures.exs", __DIR__))
Code.require_file(Path.expand("../apps/web/test/support/workflow_api_test_support.exs", __DIR__))
Code.require_file(Path.expand("../apps/web/test/support/workflow_catalog_00_benchmark_fixtures.exs", __DIR__))
Code.require_file(Path.expand("../apps/web/test/support/workflow_catalog_benchmark.exs", __DIR__))

config_path = Path.expand("../apps/web/config/config.exs", __DIR__)
{config, _imports} = Config.Reader.read_imports!(config_path, env: :test)
Application.put_all_env(config, persistent: true)
{:ok, _} = Application.ensure_all_started(:kyuubiki_web)

{options, positional, _invalid} =
  OptionParser.parse(System.argv(),
    strict: [output: :string, repeat: :integer],
    aliases: [o: :output, r: :repeat]
  )

case_ids =
  case positional do
    [] -> WorkflowCatalogBenchmark.default_case_ids()
    values -> values
  end

repeat = Keyword.get(options, :repeat, 3)
report = WorkflowCatalogBenchmark.benchmark_report(KyuubikiWeb.Router.init([]), case_ids, repeat)
json = Jason.encode_to_iodata!(report, pretty: true)

case Keyword.get(options, :output) do
  nil ->
    IO.binwrite(json)

  path ->
    File.write!(path, json)
    IO.puts("wrote workflow catalog benchmark report to #{path}")
end
