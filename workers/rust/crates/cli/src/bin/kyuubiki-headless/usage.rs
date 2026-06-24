pub(super) fn print_usage() {
    println!(
        "kyuubiki headless (Rust)\n\nUsage:\n  kyuubiki-headless help\n  kyuubiki-headless templates [--runtime service_only|browser_only|hybrid] [--category name] [--tag label] [--query text] [--json]\n  kyuubiki-headless suggest <query> [--json]\n  kyuubiki-headless init [--template <id>] [--runtime-style service_only|browser_only|hybrid] [--category name] [--tag label] [--query text] [--workflow-id workflow.id] [--out output.json] [--json]\n  kyuubiki-headless inspect <input> [--json]\n  kyuubiki-headless validate <input> [--json]\n  kyuubiki-headless plan <input> [--json] [--out plan.json]\n  kyuubiki-headless run <input> [--json] [--report-out report.json] [--material-report study] [--material-report-out report.json] [--allow-sensitive] [--allow-destructive] [--execute] [--executor mock|service|hybrid] [--api-base-url http://127.0.0.1:3000] [--api-token token]"
    );
}
