from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Any

from .errors import KyuubikiSdkError, classify_error
from .session import KyuubikiSession


@dataclass(frozen=True)
class KyuubikiRetryPolicy:
    max_attempts: int = 3
    retry_on: tuple[str, ...] = ("timeout", "transport")
    backoff_s: float = 1.0
    backoff_multiplier: float = 2.0


class KyuubikiAgentClient:
    def __init__(self, session: KyuubikiSession) -> None:
        self.session = session

    def run_study(
        self,
        solve_kind: str,
        payload: dict[str, Any],
        *,
        poll_interval_s: float = 1.0,
        timeout_s: float = 300.0,
        include_result: bool = True,
    ) -> dict[str, Any]:
        outcome = self.session.submit_and_wait(
            solve_kind,
            payload,
            poll_interval_s=poll_interval_s,
            timeout_s=timeout_s,
        )
        terminal = outcome["terminal"]
        job = terminal.get("job", {})
        result = None
        if include_result and job.get("status") == "completed" and self.session.control_plane is not None:
            result = self.session.control_plane.fetch_result(job["job_id"])
        return {
            "submitted": outcome["submitted"],
            "terminal": terminal,
            "history": outcome["history"],
            "result": result,
        }

    def fetch_job_bundle(self, job_id: str, *, include_result: bool = True) -> dict[str, Any]:
        if self.session.control_plane is None:
            raise RuntimeError("control plane client is not configured")
        job = self.session.control_plane.fetch_job(job_id)
        result = self.session.control_plane.fetch_result(job_id) if include_result else None
        return {"job": job, "result": result}

    def run_study_with_retry(
        self,
        solve_kind: str,
        payload: dict[str, Any],
        *,
        retry_policy: KyuubikiRetryPolicy | None = None,
        poll_interval_s: float = 1.0,
        timeout_s: float = 300.0,
        include_result: bool = True,
    ) -> dict[str, Any]:
        policy = retry_policy or KyuubikiRetryPolicy()
        attempts: list[dict[str, Any]] = []
        delay_s = policy.backoff_s

        for attempt_index in range(1, policy.max_attempts + 1):
            try:
                outcome = self.run_study(
                    solve_kind,
                    payload,
                    poll_interval_s=poll_interval_s,
                    timeout_s=timeout_s,
                    include_result=include_result,
                )
                return {
                    **outcome,
                    "attempt_count": attempt_index,
                    "attempts": attempts,
                }
            except Exception as error:
                classification = self.classify_failure(error=error)
                attempts.append(
                    {
                        "attempt": attempt_index,
                        "classification": classification,
                        "message": str(error),
                    }
                )
                if attempt_index >= policy.max_attempts or classification not in policy.retry_on:
                    raise
                time.sleep(delay_s)
                delay_s *= policy.backoff_multiplier

        raise RuntimeError("retry loop exited unexpectedly")

    def browse_result_chunks(
        self,
        job_id: str,
        kind: str,
        *,
        offset: int = 0,
        limit: int = 500,
    ) -> dict[str, Any]:
        if self.session.control_plane is None:
            raise RuntimeError("control plane client is not configured")
        return self.session.control_plane.fetch_result_chunk(
            job_id,
            kind,
            offset=offset,
            limit=limit,
        )

    def iter_result_chunks(
        self,
        job_id: str,
        kind: str,
        *,
        page_size: int = 500,
        start_offset: int = 0,
        max_pages: int | None = None,
    ):
        offset = start_offset
        pages = 0

        while True:
            page = self.browse_result_chunks(job_id, kind, offset=offset, limit=page_size)
            yield page
            pages += 1

            returned = int(page.get("returned", 0))
            total = int(page.get("total", 0))
            if returned <= 0 or offset + returned >= total:
                return
            if max_pages is not None and pages >= max_pages:
                return
            offset += returned

    def classify_failure(
        self,
        *,
        error: Exception | None = None,
        terminal: dict[str, Any] | None = None,
    ) -> str:
        if error is not None:
            return classify_error(error)

        if terminal is not None:
            status = terminal.get("job", {}).get("status")
            if status == "completed":
                return "completed"
            if status == "failed":
                return "failed"
            if status == "cancelled":
                return "cancelled"
            return "pending"

        return "unknown"
