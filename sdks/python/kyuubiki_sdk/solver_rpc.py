from __future__ import annotations

import json
import socket
import struct
import uuid
from typing import Any


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

        with socket.create_connection((self.host, self.port), timeout=self.timeout_s) as sock:
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
                raise RuntimeError(frame.get("error", {}).get("message", "rpc failed"))

    def _recv_exact(self, sock: socket.socket, size: int) -> bytes:
        chunks = bytearray()
        while len(chunks) < size:
            chunk = sock.recv(size - len(chunks))
            if not chunk:
                raise RuntimeError("rpc connection closed before frame completed")
            chunks.extend(chunk)
        return bytes(chunks)

    def ping(self) -> dict[str, Any]:
        return self._call("ping")

    def describe_agent(self) -> dict[str, Any]:
        return self._call("describe_agent")

    def solve_bar_1d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._call("solve_bar_1d", payload)

    def solve_truss_2d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._call("solve_truss_2d", payload)

    def solve_truss_3d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._call("solve_truss_3d", payload)

    def solve_plane_triangle_2d(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._call("solve_plane_triangle_2d", payload)

    def cancel_job(self, job_id: str) -> dict[str, Any]:
        return self._call("cancel_job", {"job_id": job_id})
