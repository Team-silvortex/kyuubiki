from __future__ import annotations

import unittest

from kyuubiki_sdk import KyuubikiAuth


class HeadlessSdkSecurityTest(unittest.TestCase):
    def test_auth_repr_redacts_and_rejects_header_injection(self) -> None:
        token = "private-python-sdk-token"
        auth = KyuubikiAuth.access_token(token)

        self.assertNotIn(token, repr(auth))
        auth.validate()
        with self.assertRaisesRegex(ValueError, "invalid authentication header value"):
            KyuubikiAuth.access_token("token\r\nX-Injected: yes").apply({})

    def test_auth_rejects_unsafe_custom_header_names_and_empty_values(self) -> None:
        with self.assertRaisesRegex(ValueError, "invalid authentication header name"):
            KyuubikiAuth("X-Test\r\nInjected", "token").validate()
        with self.assertRaisesRegex(ValueError, "invalid authentication header value"):
            KyuubikiAuth.access_token("").validate()


if __name__ == "__main__":
    unittest.main()
