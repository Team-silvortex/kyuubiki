from __future__ import annotations

import json
import socket
import struct
import uuid
from typing import Any

from .errors import KyuubikiRpcError, KyuubikiTransportError

_SOLVER_METHODS: dict[str, str] = {
    "bar_1d": "solve_bar_1d",
    "thermal_bar_1d": "solve_thermal_bar_1d",
    "heat_bar_1d": "solve_heat_bar_1d",
    "electrostatic_bar_1d": "solve_electrostatic_bar_1d",
    "beam_1d": "solve_beam_1d",
    "thermal_beam_1d": "solve_thermal_beam_1d",
    "torsion_1d": "solve_torsion_1d",
    "spring_1d": "solve_spring_1d",
    "spring_2d": "solve_spring_2d",
    "spring_3d": "solve_spring_3d",
    "truss_2d": "solve_truss_2d",
    "thermal_truss_2d": "solve_thermal_truss_2d",
    "frame_2d": "solve_frame_2d",
    "thermal_frame_2d": "solve_thermal_frame_2d",
    "plane_triangle_2d": "solve_plane_triangle_2d",
    "heat_plane_triangle_2d": "solve_heat_plane_triangle_2d",
    "thermal_plane_triangle_2d": "solve_thermal_plane_triangle_2d",
    "electrostatic_plane_triangle_2d": "solve_electrostatic_plane_triangle_2d",
    "plane_quad_2d": "solve_plane_quad_2d",
    "heat_plane_quad_2d": "solve_heat_plane_quad_2d",
    "thermal_plane_quad_2d": "solve_thermal_plane_quad_2d",
    "electrostatic_plane_quad_2d": "solve_electrostatic_plane_quad_2d",
    "truss_3d": "solve_truss_3d",
    "thermal_truss_3d": "solve_thermal_truss_3d",
    "frame_3d": "solve_frame_3d",
    "thermal_frame_3d": "solve_thermal_frame_3d",
}

_SOLVE_KIND_ALIASES: dict[str, str] = {
    "axial_bar_1d": "bar_1d",
}


class SolverRpcClient:
    def __init__(self, host: str, port: int, timeout_s: float = 15.0) -> None:
        self.host = host
        self.port = port
        self.timeout_s = timeout_s

    def _call(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        request_id = str(uuid.uuid4())
        payload = json.dumps(
            {
                "rpc_version": 1,
                "id": request_id,
                "method": method,
                "params": params or {},
            }
        ).encode("utf-8")

        try:
            sock = socket.create_connection((self.host, self.port), timeout=self.timeout_s)
        except OSError as error:
            raise KyuubikiTransportError(str(error)) from error

        with sock:
            sock.sendall(struct.pack(">I", len(payload)) + payload)
            progress_frames: list[dict[str, Any]] = []

            while True:
                header = self._recv_exact(sock, 4)
                size = struct.unpack(">I", header)[0]
                frame = json.loads(self._recv_exact(sock, size).decode("utf-8"))
                if "event" in frame:
                    progress_frames.append(frame)
                    continue
                if frame.get("ok") is True:
                    return {
                        "result": frame.get("result"),
                        "progress_frames": progress_frames,
                    }
                error = frame.get("error", {})
                raise KyuubikiRpcError(error.get("message", "rpc failed"), code=error.get("code"))

    def _recv_exact(self, sock: socket.socket, size: int) -> bytes:
        chunks = bytearray()
        while len(chunks) < size:
            chunk = sock.recv(size - len(chunks))
            if not chunk:
                raise KyuubikiTransportError("rpc connection closed before frame completed")
            chunks.extend(chunk)
        return bytes(chunks)

    def ping(self) -> dict[str, Any]:
        return self._call("ping")

    def describe_agent(self) -> dict[str, Any]:
        return self._call("describe_agent")

    def solve_study(self, solve_kind: str, payload: dict[str, Any]) -> dict[str, Any]:
        normalized = normalize_solve_kind(solve_kind)
        method = _SOLVER_METHODS.get(normalized)
        if method is None:
            raise ValueError(f"unsupported solve kind: {solve_kind}")
        return self._call(method, payload)

    def solve_bar_1d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.solve_study("bar_1d", payload)

    def solve_truss_2d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.solve_study("truss_2d", payload)

    def solve_truss_3d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.solve_study("truss_3d", payload)

    def solve_plane_triangle_2d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.solve_study("plane_triangle_2d", payload)

    def cancel_job(self, job_id: str) -> dict[str, Any]:
        return self._call("cancel_job", {"job_id": job_id})


def normalize_solve_kind(solve_kind: str) -> str:
    normalized = solve_kind.strip().lower()
    return _SOLVE_KIND_ALIASES.get(normalized, normalized)
