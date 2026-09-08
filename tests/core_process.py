"""Exercise the actual core executable and its streamed transport."""
import json
import subprocess
import sys
import unittest

BINARY = sys.argv.pop(1)


class CoreProcessTests(unittest.TestCase):
    def run_core(self, payload):
        result = subprocess.run([BINARY, "--stdio"], input=payload, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        return [json.loads(line) for line in result.stdout.splitlines()]

    def test_replies_keep_request_identity(self):
        payload = b"\n".join(json.dumps({"protocol_version": 1, "id": ident, "method": "core.info"}).encode()
                             for ident in ["first", "second"]) + b"\n"
        replies = self.run_core(payload)
        self.assertEqual([r["id"] for r in replies], ["first", "second"])
        self.assertTrue(all(r["ok"] for r in replies))
        self.assertIn("apps.list", replies[0]["result"]["capabilities"])
        self.assertNotIn("install", replies[0]["result"]["capabilities"])

    def test_public_catalogue_and_unknown_filters(self):
        requests = [
            {"protocol_version": 1, "id": "catalogue", "method": "catalogue.info"},
            {"protocol_version": 1, "id": "apps", "method": "apps.list", "params": {"price": "cheap"}},
            {"protocol_version": 1, "id": "next", "method": "apps.list"},
        ]
        replies = self.run_core(b"\n".join(json.dumps(r).encode() for r in requests) + b"\n")
        self.assertFalse(replies[0]["result"]["demo"])
        self.assertEqual(replies[1]["error"]["code"], "invalid_filter")
        self.assertTrue(replies[2]["ok"])

    def test_invalid_protocol_and_unknown_fields_are_rejected(self):
        requests = [
            {"protocol_version": 99, "id": "wrong", "method": "core.info"},
            {"protocol_version": 1, "id": "command", "method": "core.info", "command": "touch /tmp/should-not-run"},
        ]
        replies = self.run_core(b"\n".join(json.dumps(r).encode() for r in requests) + b"\n")
        self.assertEqual([r["error"]["code"] for r in replies], ["unsupported_protocol", "invalid_request"])

    def test_oversized_input_is_bounded_and_terminates(self):
        replies = self.run_core(b"x" * (256 * 1024 + 1))
        self.assertEqual(len(replies), 1)
        self.assertEqual(replies[0]["error"]["code"], "message_too_large")


if __name__ == "__main__":
    unittest.main()
