#!/usr/bin/env node

import { readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_LOG_PATH = path.resolve("tmp/workflow-mesh-regression/latest/run.log");

function parseArgs(argv) {
  const options = {
    logPath: DEFAULT_LOG_PATH,
    outputDir: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];

    if (arg === "--log" && next) {
      options.logPath = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--output-dir" && next) {
      options.outputDir = path.resolve(next);
      index += 1;
    }
  }

  return options;
}

function parseLog(logText) {
  const lines = logText.split(/\r?\n/);
  const tests = [];
  let current = null;
  let totalPass = 0;
  let totalFail = 0;
  let completed = false;

  for (const line of lines) {
    const testMatch = line.match(/^==> running (.+)$/);
    if (testMatch) {
      current = {
        test_file: testMatch[1],
        subtest: null,
        pass: 0,
        fail: 0,
        duration_ms: null,
        status: "running",
      };
      tests.push(current);
      continue;
    }

    if (!current) {
      if (line.trim() === "workflow mesh regression completed") {
        completed = true;
      }
      continue;
    }

    const subtestMatch = line.match(/^# Subtest: (.+)$/);
    if (subtestMatch) {
      current.subtest = subtestMatch[1];
      continue;
    }

    const specPassMatch = line.match(/^✔ (.+) \(([0-9.]+)ms\)$/);
    if (specPassMatch) {
      current.subtest = specPassMatch[1];
      current.pass = 1;
      current.fail = 0;
      current.duration_ms = Number(specPassMatch[2]);
      current.status = "passed";
      totalPass += 1;
      continue;
    }

    const specFailMatch = line.match(/^✖ (.+) \(([0-9.]+)ms\)$/);
    if (specFailMatch) {
      current.subtest = specFailMatch[1];
      current.pass = 0;
      current.fail = 1;
      current.duration_ms = Number(specFailMatch[2]);
      current.status = "failed";
      totalFail += 1;
      continue;
    }

    const passMatch = line.match(/^# pass (\d+)$/);
    if (passMatch) {
      current.pass = Number(passMatch[1]);
      if (current.status !== "passed") {
        totalPass += Number(passMatch[1]);
      }
      continue;
    }

    const failMatch = line.match(/^# fail (\d+)$/);
    if (failMatch) {
      current.fail = Number(failMatch[1]);
      if (current.status !== "failed") {
        totalFail += Number(failMatch[1]);
      }
      current.status = Number(failMatch[1]) > 0 ? "failed" : current.status;
      continue;
    }

    const infoPassMatch = line.match(/^ℹ pass (\d+)$/);
    if (infoPassMatch) {
      current.pass = Number(infoPassMatch[1]);
      if (current.status !== "passed") {
        totalPass += Number(infoPassMatch[1]);
      }
      continue;
    }

    const infoFailMatch = line.match(/^ℹ fail (\d+)$/);
    if (infoFailMatch) {
      current.fail = Number(infoFailMatch[1]);
      if (current.status !== "failed") {
        totalFail += Number(infoFailMatch[1]);
      }
      current.status = Number(infoFailMatch[1]) > 0 ? "failed" : current.status;
      continue;
    }

    const durationMatch = line.match(/^# duration_ms ([0-9.]+)$/);
    if (durationMatch) {
      current.duration_ms = Number(durationMatch[1]);
      if (current.status !== "failed") {
        current.status = current.pass > 0 ? "passed" : "completed";
      }
      current = null;
      continue;
    }

    const infoDurationMatch = line.match(/^ℹ duration_ms ([0-9.]+)$/);
    if (infoDurationMatch) {
      current.duration_ms = Number(infoDurationMatch[1]);
      if (current.status !== "failed") {
        current.status = current.pass > 0 ? "passed" : "completed";
      }
      current = null;
      continue;
    }

    if (line.trim() === "workflow mesh regression completed") {
      completed = true;
    }
  }

  const totalDurationMs = tests.reduce(
    (sum, test) => sum + (Number.isFinite(test.duration_ms) ? test.duration_ms : 0),
    0,
  );

  return {
    completed,
    total_pass: totalPass,
    total_fail: totalFail,
    total_duration_ms: totalDurationMs,
    tests,
  };
}

function buildReadme(summary, logPath) {
  const lines = [
    "# Workflow Mesh Regression",
    "",
    `- Status: \`${summary.status}\``,
    `- Completed marker: \`${summary.completed}\``,
    `- Total tests: \`${summary.total_tests}\``,
    `- Total pass: \`${summary.total_pass}\``,
    `- Total fail: \`${summary.total_fail}\``,
    `- Total duration ms: \`${summary.total_duration_ms.toFixed(3)}\``,
    `- Log: \`${logPath}\``,
    "",
    "## Cases",
    "",
  ];

  for (const test of summary.tests) {
    lines.push(`- \`${test.test_file}\``);
    lines.push(`  Status: \`${test.status}\``);
    lines.push(`  Subtest: ${test.subtest ? `\`${test.subtest}\`` : "`n/a`"}`);
    lines.push(
      `  Pass/fail: \`${test.pass}\` / \`${test.fail}\`, duration ms: \`${(test.duration_ms ?? 0).toFixed(3)}\``,
    );
  }

  lines.push("");
  return `${lines.join("\n").trimEnd()}\n`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const outputDir = options.outputDir ?? path.dirname(options.logPath);
  const logText = await readFile(options.logPath, "utf8");
  const parsed = parseLog(logText);
  const info = await stat(options.logPath);
  const summary = {
    schema_version: "kyuubiki.workflow-mesh-regression-summary/v1",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    log_path: path.relative(process.cwd(), options.logPath) || path.basename(options.logPath),
    completed: parsed.completed,
    status: parsed.completed && parsed.total_fail === 0 ? "passed" : "failed",
    total_tests: parsed.tests.length,
    total_pass: parsed.total_pass,
    total_fail: parsed.total_fail,
    total_duration_ms: parsed.total_duration_ms,
    log_mtime_unix_s: Math.floor(info.mtimeMs / 1000),
    tests: parsed.tests,
  };

  await writeFile(
    path.join(outputDir, "summary.json"),
    `${JSON.stringify(summary, null, 2)}\n`,
  );
  await writeFile(
    path.join(outputDir, "README.md"),
    buildReadme(summary, summary.log_path),
  );

  process.stdout.write(`${path.relative(process.cwd(), outputDir) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
