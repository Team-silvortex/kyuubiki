#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "sdks" / "python"))

from kyuubiki_sdk import SolverRpcClient  # noqa: E402
from kyuubiki_sdk.solver_fixtures import (  # noqa: E402
    minimal_solver_payloads,
    solver_fixture_rpc_methods,
)


SUMMARY_PREFIXES = ("max_", "total_", "final_", "peak_", "active_")
SUMMARY_NAMES = ("converged", "residual_norm")
PROFILE_METHODS = {
    "advertised": [],
    "lab-legacy-26": [
        "solve_bar_1d",
        "solve_thermal_bar_1d",
        "solve_heat_bar_1d",
        "solve_electrostatic_bar_1d",
        "solve_electrostatic_plane_triangle_2d",
        "solve_electrostatic_plane_quad_2d",
        "solve_heat_plane_triangle_2d",
        "solve_heat_plane_quad_2d",
        "solve_thermal_truss_2d",
        "solve_thermal_truss_3d",
        "solve_spring_1d",
        "solve_spring_2d",
        "solve_spring_3d",
        "solve_beam_1d",
        "solve_thermal_beam_1d",
        "solve_thermal_frame_2d",
        "solve_thermal_frame_3d",
        "solve_torsion_1d",
        "solve_truss_2d",
        "solve_truss_3d",
        "solve_frame_3d",
        "solve_plane_triangle_2d",
        "solve_thermal_plane_triangle_2d",
        "solve_plane_quad_2d",
        "solve_thermal_plane_quad_2d",
        "solve_frame_2d",
    ],
}
PROFILE_METHODS["current-40"] = sorted(solver_fixture_rpc_methods())


def main() -> int:
    parser = argparse.ArgumentParser(description="Run small solver payloads against an agent's advertised RPC capabilities.")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=5001)
    parser.add_argument("--timeout", type=float, default=15.0)
    parser.add_argument("--output", help="Optional JSON report path.")
    parser.add_argument("--profile", choices=sorted(PROFILE_METHODS), default="advertised", help="Named capability gate profile.")
    parser.add_argument("--fail-on-untested", action="store_true", help="Fail when the agent advertises solver methods without local fixtures.")
    parser.add_argument("--min-solver-methods", type=int, default=0, help="Fail when the agent advertises fewer solver methods than this count.")
    parser.add_argument("--expect-method", action="append", default=[], help="Require an advertised RPC method such as solve_bar_1d. May be repeated.")
    parser.add_argument("--expect-kind", action="append", default=[], help="Require a Python SDK solve kind such as bar_1d. May be repeated.")
    args = parser.parse_args()

    client = SolverRpcClient(args.host, args.port, timeout_s=args.timeout)
    descriptor = client.describe_agent()["result"]
    advertised = sorted(
        method
        for method in descriptor.get("protocol", {}).get("methods", [])
        if method.startswith("solve_")
    )
    method_to_kind = solver_fixture_rpc_methods()
    payloads = minimal_solver_payloads()
    profile_methods = PROFILE_METHODS[args.profile]
    min_solver_methods = max(args.min_solver_methods, len(profile_methods))
    expected_methods = sorted(
        set(profile_methods)
        | set(args.expect_method)
        | {_kind_to_method(kind) for kind in args.expect_kind}
    )
    missing_expected_methods = [method for method in expected_methods if method not in advertised]
    runnable = [method for method in advertised if method in method_to_kind]
    untested = [method for method in advertised if method not in method_to_kind]
    results = [_run_case(client, method_to_kind[method], payloads[method_to_kind[method]]) for method in runnable]
    failures = [result for result in results if not result["ok"]]
    report = {
        "agent": {
            "host": args.host,
            "port": args.port,
            "program": descriptor.get("program"),
            "runtime_mode": descriptor.get("runtime", {}).get("runtime_mode"),
            "health_score": descriptor.get("runtime", {}).get("health_score"),
        },
        "summary": {
            "advertised_solver_methods": len(advertised),
            "tested_solver_methods": len(results),
            "passed": len(results) - len(failures),
            "failed": len(failures),
            "untested_solver_methods": len(untested),
            "missing_expected_methods": len(missing_expected_methods),
            "min_solver_methods": min_solver_methods,
        },
        "expectations": {
            "profile": args.profile,
            "expected_methods": expected_methods,
            "missing_expected_methods": missing_expected_methods,
            "solver_method_count_ok": len(advertised) >= min_solver_methods,
        },
        "results": results,
        "untested_methods": untested,
    }
    encoded = json.dumps(report, indent=2, ensure_ascii=False)
    if args.output:
        Path(args.output).write_text(encoded + "\n", encoding="utf-8")
    print(encoded)
    gate_failed = (
        failures
        or (args.fail_on_untested and untested)
        or missing_expected_methods
        or len(advertised) < min_solver_methods
    )
    return 1 if gate_failed else 0


def _kind_to_method(kind: str) -> str:
    method_to_kind = solver_fixture_rpc_methods()
    for method, mapped_kind in method_to_kind.items():
        if mapped_kind == kind:
            return method
    return f"solve_{kind}"


def _run_case(client: SolverRpcClient, kind: str, payload: dict[str, Any]) -> dict[str, Any]:
    started = time.perf_counter()
    try:
        envelope = client.solve_study(kind, payload)
        elapsed_ms = (time.perf_counter() - started) * 1000
        result = envelope.get("result", envelope)
        summary = _summary_metrics(result)
        return {"kind": kind, "ok": True, "ms": round(elapsed_ms, 2), "summary": summary}
    except Exception as error:  # noqa: BLE001 - smoke reports should preserve all failure classes.
        elapsed_ms = (time.perf_counter() - started) * 1000
        return {"kind": kind, "ok": False, "ms": round(elapsed_ms, 2), "error": str(error)}


def _summary_metrics(result: Any) -> dict[str, Any]:
    if not isinstance(result, dict):
        return {}
    summary: dict[str, Any] = {}
    for key, value in result.items():
        if key in SUMMARY_NAMES or key.startswith(SUMMARY_PREFIXES):
            if isinstance(value, (bool, int, float, str)) or value is None:
                summary[key] = value
    return summary


if __name__ == "__main__":
    raise SystemExit(main())
