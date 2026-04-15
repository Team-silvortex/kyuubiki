from __future__ import annotations

import time
from typing import Any

from .auth import KyuubikiAuth
from .control_plane import ControlPlaneClient
from .errors import KyuubikiTimeoutError
from .solver_rpc import SolverRpcClient


class KyuubikiSession:
    def __init__(
        self,
        control_plane: ControlPlaneClient | None = None,
        solver_rpc: SolverRpcClient | None = None,
    ) -> None:
        self.control_plane = control_plane
        self.solver_rpc = solver_rpc

    @classmethod
    def from_control_plane(
        cls,
        base_url: str,
        token: str | None = None,
        auth: KyuubikiAuth | None = None,
    ) -> "KyuubikiSession":
        return cls(control_plane=ControlPlaneClient(base_url, token=token, auth=auth))

    @classmethod
    def from_endpoints(
        cls,
        base_url: str,
        *,
        token: str | None = None,
        auth: KyuubikiAuth | None = None,
        rpc_host: str | None = None,
        rpc_port: int | None = None,
        rpc_timeout_s: float = 15.0,
    ) -> "KyuubikiSession":
        solver_rpc = None
        if rpc_host is not None and rpc_port is not None:
            solver_rpc = SolverRpcClient(rpc_host, rpc_port, timeout_s=rpc_timeout_s)
        return cls(
            control_plane=ControlPlaneClient(base_url, token=token, auth=auth),
            solver_rpc=solver_rpc,
        )

    def submit_job(self, solve_kind: str, payload: dict[str, Any]) -> dict[str, Any]:
        if self.control_plane is None:
            raise RuntimeError("control plane client is not configured")

        normalized = solve_kind.strip().lower()
        if normalized == "bar_1d":
            return self.control_plane.create_axial_bar_job(payload)
        if normalized == "truss_2d":
            return self.control_plane.create_truss_2d_job(payload)
        if normalized == "truss_3d":
            return self.control_plane.create_truss_3d_job(payload)
        if normalized == "plane_triangle_2d":
            return self.control_plane.create_plane_triangle_2d_job(payload)
        raise ValueError(f"unsupported solve kind: {solve_kind}")

    def submit_jobs(self, jobs: list[dict[str, Any]]) -> list[dict[str, Any]]:
        return [self.submit_job(job["solve_kind"], job["payload"]) for job in jobs]

    def solve_direct(self, solve_kind: str, payload: dict[str, Any]) -> dict[str, Any]:
        if self.solver_rpc is None:
            raise RuntimeError("solver rpc client is not configured")

        normalized = solve_kind.strip().lower()
        if normalized == "bar_1d":
            return self.solver_rpc.solve_bar_1d(payload)
        if normalized == "truss_2d":
            return self.solver_rpc.solve_truss_2d(payload)
        if normalized == "truss_3d":
            return self.solver_rpc.solve_truss_3d(payload)
        if normalized == "plane_triangle_2d":
            return self.solver_rpc.solve_plane_triangle_2d(payload)
        raise ValueError(f"unsupported solve kind: {solve_kind}")

    def wait_for_job(
        self,
        job_id: str,
        *,
        poll_interval_s: float = 1.0,
        timeout_s: float = 300.0,
        terminal_statuses: tuple[str, ...] = ("completed", "failed", "cancelled"),
    ) -> dict[str, Any]:
        if self.control_plane is None:
            raise RuntimeError("control plane client is not configured")

        deadline = time.monotonic() + timeout_s
        history: list[dict[str, Any]] = []
        last_status: str | None = None
        last_progress: Any = None

        while time.monotonic() <= deadline:
            payload = self.control_plane.fetch_job(job_id)
            job = payload.get("job", {})
            status = job.get("status")
            progress = job.get("progress")
            if status != last_status or progress != last_progress:
                history.append(payload)
                last_status = status
                last_progress = progress
            if status in terminal_statuses:
                return {"terminal": payload, "history": history}
            time.sleep(poll_interval_s)

        raise KyuubikiTimeoutError(f"timed out waiting for job {job_id}")

    def submit_and_wait(
        self,
        solve_kind: str,
        payload: dict[str, Any],
        *,
        poll_interval_s: float = 1.0,
        timeout_s: float = 300.0,
    ) -> dict[str, Any]:
        submitted = self.submit_job(solve_kind, payload)
        job_id = submitted["job"]["job_id"]
        waited = self.wait_for_job(
            job_id,
            poll_interval_s=poll_interval_s,
            timeout_s=timeout_s,
        )
        return {"submitted": submitted, **waited}
