from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class KyuubikiAuth:
    header_name: str
    header_value: str

    @classmethod
    def access_token(cls, token: str) -> "KyuubikiAuth":
        return cls(header_name="x-kyuubiki-token", header_value=token)

    def apply(self, headers: dict[str, str]) -> dict[str, str]:
        headers[self.header_name] = self.header_value
        return headers
