"""Container-shim checks; the Rust host has its own cargo tests."""
import importlib.util
import io
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parent
PNG = b"\x89PNG\r\n\x1a\n" + bytes(range(256))
spec = importlib.util.spec_from_file_location("bridge", ROOT / "bridge.py")
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)


class ClientTest(unittest.TestCase):
    def test_should_translate_arguments_and_write_binary_output(self):
        cases = [
            ("wl-paste", ["-l"], {"op": "types"}),
            ("wl-paste", ["--list-types"], {"op": "types"}),
            ("wl-paste", ["-n", "-t", "image/png"], {"op": "read", "type": "image/png"}),
            ("wl-paste", ["--type=image/png"], {"op": "read", "type": "image/png"}),
            ("xclip", ["-selection", "clipboard", "-t", "TARGETS", "-o"], {"op": "types"}),
            ("xclip", ["-selection", "clipboard", "-t", "image/png", "-o"], {"op": "read", "type": "image/png"}),
        ]
        for tool, args, expected in cases:
            with self.subTest(tool=tool, args=args), \
                 mock.patch.object(bridge, "request_host", return_value=PNG) as request, \
                 mock.patch.object(bridge.sys, "stdout", SimpleNamespace(buffer=io.BytesIO())) as output:
                bridge.client(tool, args)
                request.assert_called_once_with(expected)
                self.assertEqual(output.buffer.getvalue(), PNG)

    def test_should_reject_write_options(self):
        with self.assertRaises(RuntimeError):
            bridge.client("xclip", ["-selection", "clipboard", "-i"])

    def test_should_reject_non_image_reads(self):
        with self.assertRaises(RuntimeError):
            bridge.client("wl-paste", ["--type", "text/plain"])

    def test_should_reject_missing_type_argument(self):
        with self.assertRaises(RuntimeError):
            bridge.client("wl-paste", ["--type"])


if __name__ == "__main__":
    unittest.main()
