#!/usr/bin/env python3
"""Run service endpoint rotation validation with native Python orchestration."""

from __future__ import annotations

import os
import re
import json
import subprocess
import sys
import time
from pathlib import Path


def resolve_kyuubiki_cli(repo_dir: Path):
    repo_dir = repo_dir.resolve()
    candidates = [
        repo_dir / "scripts" / "kyuubiki",
        repo_dir.parent / "scripts" / "kyuubiki",
    ]
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate.parent, [str(candidate)]

    if (repo_dir / "Cargo.toml").is_file():
        return repo_dir, ["cargo", "run", "-p", "kyuubiki-script-runner", "--"]

    if (repo_dir / "workers" / "rust" / "Cargo.toml").is_file():
        return repo_dir / "workers" / "rust", ["cargo", "run", "-p", "kyuubiki-script-runner", "--"]

    raise RuntimeError(f"could not locate kyuubiki CLI in {repo_dir}")


def run_cmd(cmd_dir: Path, base_cmd, label: str, outfile: Path, args: list[str]) -> int:
    process = subprocess.run(
        args=[*base_cmd, *args],
        cwd=str(cmd_dir),
        capture_output=True,
        text=True,
    )
    outfile.with_suffix(f"{outfile.suffix}.out").write_text(process.stdout or "")
    (outfile.with_suffix(f"{outfile.suffix}.err")).write_text(process.stderr or "")
    (outfile.with_suffix(f"{outfile.suffix}.status")).write_text(str(process.returncode))
    return process.returncode


def read_json(path: Path):
    try:
        return json.loads(path.read_text())
    except Exception:
        return None


def status_cell(path: Path, expr: str):
    if not path.is_file():
        return "missing"
    try:
        data = json.loads(path.read_text())
    except Exception:
        return "error"
    if expr == ".status":
        return data.get("status", "missing")
    if expr == ".mode":
        return data.get("mode", "missing")
    return "missing"


def is_artifact_limit_failure(report_json: Path, err_log: Path) -> bool:
    data = read_json(report_json)
    if isinstance(data, dict):
        err_code = (
            (data.get("execution_summary", {}).get("failure", {}) or {}).get("error_code")
            or (data.get("error", {}) or {}).get("code")
            or ""
        )
        if err_code in {"frontend_proxy_artifact_limit", "kyuubiki.headless.transport_failure"}:
            return True

        candidates = [
            (data.get("execution_summary", {}).get("failure", {}) or {}).get("message", ""),
            (data.get("error", {}) or {}).get("message", ""),
        ]
        status = data.get("status", "")
        message = data.get("message", "")
        if message:
            candidates.append(message)
        if status == "failed" and any(
            re.search(r"frontend_proxy_artifact_limit|transport failure|artifact transport|smaller body limit|not a frontend proxy|model artifact upload failed 500|body limit|413|Payload Too Large", m or "", re.IGNORECASE)
            for m in candidates
        ):
            return True

    if err_log.is_file():
        raw = err_log.read_text()
        if re.search(
            r"frontend_proxy_artifact_limit|transport failure|artifact transport|smaller body limit|not a frontend proxy|model artifact upload failed 500|body limit|413|Payload Too Large",
            raw,
            re.IGNORECASE,
        ):
            return True
    return False


def read_error_code(report_json: Path) -> str:
    data = read_json(report_json)
    if not isinstance(data, dict):
        return "n/a"
    err_code = (
        (data.get("execution_summary", {}).get("failure", {}) or {}).get("error_code")
        or (data.get("error", {}) or {}).get("code")
        or ""
    )
    return err_code if err_code else "n/a"


def parse_transport_limit(report_json: Path) -> tuple[str, str]:
    data = read_json(report_json)
    message = ""
    if isinstance(data, dict):
        msg_candidates = [
            (data.get("execution_summary", {}).get("failure", {}) or {}).get("message", ""),
            (data.get("error", {}) or {}).get("message", ""),
            data.get("message", ""),
        ]
        message = "\n".join([m for m in msg_candidates if m])

    if not message and report_json.with_suffix(".err").is_file():
        try:
            message = report_json.with_suffix(".err").read_text()
        except Exception:
            message = ""

    size_bytes = ""
    limit_bytes = ""
    if message:
        size_match = re.search(r"size_bytes=([0-9]+)", message)
        limit_match = re.search(r"limit_bytes=([0-9]+)", message)
        if size_match:
            size_bytes = size_match.group(1)
        if limit_match:
            limit_bytes = limit_match.group(1)
    return size_bytes, limit_bytes


def main():
    repo_dir = Path(os.environ.get("KYUUBIKI_REPO_DIR", "/Users/Shared/chroot/dev/kyuubiki"))
    if not repo_dir.is_dir():
        raise SystemExit(f"KYUUBIKI_REPO_DIR does not exist: {repo_dir}")

    workspace_dir = Path(
        os.environ.get(
            "WORKSPACE_DIR",
            "/Users/Shared/chroot/research/kyuubiki-em-material-lab",
        )
    )
    if not workspace_dir.is_dir():
        workspace_dir = Path(__file__).resolve().parent.parent
    if not workspace_dir.is_dir():
        raise SystemExit(f"WORKSPACE_DIR does not exist: {workspace_dir}")

    ts = os.environ.get("SERVICE_MATRIX_TS", time.strftime("%Y%m%d-%H%M%S"))
    out_dir = workspace_dir / f"results/service-matrix-port-rotation-{ts}"
    report_path = workspace_dir / f"reports/service-matrix-port-rotation-{ts}.md"
    out_dir.mkdir(parents=True, exist_ok=True)

    cli_dir, cli = resolve_kyuubiki_cli(repo_dir)

    report_path.write_text(
        "# Service Port Rotation Regression\n\n"
        f"- 时间: {time.strftime('%Y-%m-%d %H:%M:%S')}\n"
        f"- 仓库: {repo_dir}\n"
        f"- 工作目录: {workspace_dir}\n"
        "- 说明: 对同一批输入分别执行 3000 与 4000，检查 3000 的 payload 限制退化是否可被 4000 回放。\n"
    )

    primary_api = os.environ.get("SERVICE_PRIMARY_API_BASE_URL", "http://127.0.0.1:3000")
    fallback_api_default = os.environ.get("SERVICE_FALLBACK_API_BASE_URL", "http://127.0.0.1:4000")
    control_plane_api = os.environ.get("SERVICE_CONTROL_PLANE_API_BASE_URL", "http://127.0.0.1:4000")

    test_cases = [
        ("large_700x700", workspace_dir / "results/sdk-large-mesh-1m/input_700x700.json"),
        ("large_1000x1000_noids", workspace_dir / "results/sdk-large-mesh-1m/input_1000x1000_noids.json"),
        (
            "large_3000x1000_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_3000x1000_noids_jobwait_1200000.json",
        ),
        (
            "large_2000x1000_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_2000x1000_noids_jobwait_1200000.json",
        ),
        (
            "large_2000x2000_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_2000x2000_noids_jobwait_1200000.json",
        ),
        (
            "large_2500x1600_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_2500x1600_noids_jobwait_1200000.json",
        ),
        (
            "large_1800x2200_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_1800x2200_noids_jobwait_1200000.json",
        ),
        (
            "large_3000x900_noids",
            workspace_dir / "results/sdk-large-mesh-1m/input_3000x900_noids_jobwait_1200000.json",
        ),
        ("small_direct_heat_triangle", workspace_dir / "results/headless-all-dryrun-20260804-123750/direct_heat_triangle/input.json"),
    ]

    lines = []
    force_fallback = os.environ.get("FORCE_FALLBACK_4000", "0") == "1"

    for name, input_path in test_cases:
        if not input_path.is_file():
            lines.append(f"## {name}\n- input: {input_path}\n- error: input not found\n")
            continue

        case_dir = out_dir / name
        case_dir.mkdir(parents=True, exist_ok=True)
        print(f"==case:{name}")

        validate_status = run_cmd(
            cli_dir,
            cli,
            f"{name}-validate",
            case_dir / "validate",
            ["headless", "validate", str(input_path), "--json"],
        )
        _ = validate_status  # keep behavior parity; status is always logged in file

        primary_rc = run_cmd(
            cli_dir,
            cli,
            f"{name}-run-3000",
            case_dir / "service_3000",
            [
                "headless",
                "run",
                str(input_path),
                "--json",
                "--report-out",
                str(case_dir / "service_3000.json"),
                "--execute",
                "--executor",
                "service",
                "--api-base-url",
                primary_api,
            ],
        )

        transport_signature = is_artifact_limit_failure(
            case_dir / "service_3000.json", case_dir / "service_3000.err"
        )
        service_3000_error_code = read_error_code(case_dir / "service_3000.json")
        fallback_api = fallback_api_default
        if transport_signature:
            fallback_api = control_plane_api

        fallback_triggered = force_fallback or primary_rc != 0 or transport_signature
        fallback_reason = (
            "force_fallback"
            if force_fallback
            else "transport_or_artifact_signature"
            if transport_signature
            else "primary_exit_nonzero"
            if primary_rc != 0
            else "-"
        )
        if fallback_triggered:
            run_cmd(
                cli_dir,
                cli,
                f"{name}-run-4000",
                case_dir / "service_4000",
                [
                    "headless",
                    "run",
                    str(input_path),
                    "--json",
                    "--report-out",
                    str(case_dir / "service_4000.json"),
                    "--execute",
                    "--executor",
                    "service",
                    "--allow-sensitive",
                    "--api-base-url",
                    fallback_api,
                ],
            )
            (case_dir / "service_4000.url").write_text(fallback_api)
        else:
            (case_dir / "service_4000.status").write_text(f"fallback skipped: case={name}")
            (case_dir / "service_4000.url").write_text(fallback_api)

        s3000_status = (case_dir / "service_3000.status").read_text().strip()
        s4000_status = (case_dir / "service_4000.status").read_text().strip()
        s3000_run_status = status_cell(case_dir / "service_3000.json", ".status")
        s3000_mode = status_cell(case_dir / "service_3000.json", ".mode")
        s4000_run_status = status_cell(case_dir / "service_4000.json", ".status")
        s4000_mode = status_cell(case_dir / "service_4000.json", ".mode")
        err_body_limit = "1" if transport_signature else "0"
        service_3000_error_code = service_3000_error_code
        service_4000_error_code = read_error_code(case_dir / "service_4000.json")
        (size_bytes, limit_bytes) = parse_transport_limit(case_dir / "service_4000.json")
        fallback_url = (case_dir / "service_4000.url").read_text().strip()

        lines.extend(
            [
                f"## {name}\n",
                f"- input: {input_path}\n",
                f"- validate.status: {(case_dir / 'validate.status').read_text().strip()}\n",
                f"- service_3000.shell-status: {s3000_status}\n",
                f"- service_3000.report.status: {s3000_run_status}\n",
                f"- service_3000.mode: {s3000_mode}\n",
                f"- service_3000.error_code: {service_3000_error_code}\n",
                f"- service_3000.body-limit-signature: {err_body_limit}\n",
                f"- service_4000.fallback-triggered: {1 if fallback_triggered else 0}\n",
                f"- service_4000.fallback-reason: {fallback_reason}\n",
                f"- service_4000.fallback-url: {fallback_url}\n",
                f"- service_4000.shell-status: {s4000_status}\n",
                f"- service_4000.report.status: {s4000_run_status}\n",
                f"- service_4000.error_code: {service_4000_error_code}\n",
                f"- service_4000.size_bytes: {size_bytes if size_bytes else 'n/a'}\n",
                f"- service_4000.limit_bytes: {limit_bytes if limit_bytes else 'n/a'}\n",
                f"- service_4000.mode: {s4000_mode}\n",
                "\n",
            ]
        )

    lines.extend(
        [
            "\n---\n\n## 汇总（Port Matrix）\n",
            "- 文件: `service_3000` 与 `service_4000` 的 shell status 与 report status 已逐项记录。\n",
            "- 若 `service_3000` 出现 `frontend_proxy_artifact_limit` 或类似 body/transport 限制特征，需路由到 4000（控制面）。\n",
        ]
    )
    report_path.write_text(report_path.read_text() + "\n" + "".join(lines))
    print(f"report: {report_path}")


if __name__ == "__main__":
    main()
