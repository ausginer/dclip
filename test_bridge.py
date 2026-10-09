"""Container-shim checks; the Rust host has its own cargo tests."""
import importlib.util
import io
import os
from pathlib import Path
import socket
import tempfile
import threading
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


class RequestHostTest(unittest.TestCase):
    """`request_host` against a stand-in host on a real socket."""

    def exchange(self, response):
        """Runs `request_host` against a host that accepts, writes `response`
        and closes before the shim sends its request, as a host that refuses
        the connection may. The shim's connect returns only once the host has
        closed, so the order is forced rather than raced."""
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "s.sock")
            closed = threading.Event()
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(path)
                listener.listen()

                def host():
                    connection, _ = listener.accept()
                    with connection:
                        connection.sendall(response)
                    closed.set()

                class Connection(socket.socket):
                    def connect(self, address):
                        super().connect(address)
                        if not closed.wait(5):
                            raise TimeoutError("the stand-in host never closed")

                thread = threading.Thread(target=host)
                thread.start()
                try:
                    module = SimpleNamespace(
                        socket=Connection, AF_UNIX=socket.AF_UNIX, SOCK_STREAM=socket.SOCK_STREAM
                    )
                    with mock.patch.object(bridge, "socket", module), \
                         mock.patch.dict(os.environ, {"CLAUDE_CLIPBOARD_SOCKET": path}):
                        return bridge.request_host({"op": "types"})
                finally:
                    thread.join()

    def test_should_report_a_refusal_that_arrived_before_the_request_was_sent(self):
        refusal = b'{"ok":false,"error":"too many concurrent clipboard requests","size":0}\n'
        with self.assertRaisesRegex(RuntimeError, "^too many concurrent clipboard requests$"):
            self.exchange(refusal)

    def test_should_report_the_failed_send_when_no_response_arrives(self):
        with self.assertRaises(BrokenPipeError):
            self.exchange(b"")


if __name__ == "__main__":
    unittest.main()
