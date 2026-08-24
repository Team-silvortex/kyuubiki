from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class KyuubikiAuth:
    header_name: str
    header_value: str = field(repr=False)

    @classmethod
    def access_token(cls, token: str) -> "KyuubikiAuth":
        return cls(header_name="x-kyuubiki-token", header_value=token)

    def validate(self) -> None:
        valid_name = (
            0 < len(self.header_name) <= 128
            and all(character.isascii() and (character.isalnum() or character == "-") for character in self.header_name)
        )
        if not valid_name:
            raise ValueError("invalid authentication header name")
        valid_value = (
            0 < len(self.header_value.encode("utf-8")) <= 8 * 1024
            and all(character.isascii() and character.isprintable() and not character.isspace() for character in self.header_value)
        )
        if not valid_value:
            raise ValueError("invalid authentication header value")

    def apply(self, headers: dict[str, str]) -> dict[str, str]:
        self.validate()
        headers[self.header_name] = self.header_value
        return headers
