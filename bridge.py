#!/usr/bin/env python3
"""Container-only image clipboard shims for the Rust host bridge (stdlib only)."""
import json
import os
from pathlib import Path
import socket
import sys

VERSION = "2.0"
MAX_BYTES = 64 * 1024 * 1024
IMAGE_TYPES = ("image/png", "image/jpeg", "image/jpg", "image/gif", "image/webp")
ROOT = Path(__file__).resolve().parent


def socket_path():
    return os.environ.get("CLAUDE_CLIPBOARD_SOCKET", str(ROOT / "clipboard.sock"))


def request_host(request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(12)
        connection.connect(socket_path())
        # A host that refuses the connection writes its response and closes
        # without reading the request, so the send can fail with the refusal
        # already waiting to be read. Read it whatever became of the send.
        failed_send = None
        try:
            connection.sendall(json.dumps(request).encode() + b"\n")
        except OSError as error:
            failed_send = error
        with connection.makefile("rb") as stream:
            try:
                header = json.loads(stream.readline(4097))
            except (OSError, ValueError):
                if failed_send is None:
                    raise
                raise failed_send from None
            if not header.get("ok"):
                raise RuntimeError(header.get("error", "host bridge failed"))
            size = header.get("size")
            if not isinstance(size, int) or not 0 <= size <= MAX_BYTES:
                raise RuntimeError("invalid response size")
            data = stream.read(size)
            if len(data) != size:
                raise RuntimeError("incomplete clipboard response")
            return data



def client(tool, args):
    trace = os.environ.get("CLAUDE_CLIPBOARD_TRACE")
    if trace:
        with open(trace, "a", encoding="utf-8") as log:
            log.write(json.dumps({"tool": tool, "args": args}) + "\n")
    if "--version" in args or "-version" in args:
        print(f"{tool} (host image bridge {VERSION})")
        return
    if "--help" in args or "-h" in args:
        print("Host image bridge: list image MIME types, or read image bytes.")
        return
    listing, mime = False, None
    index = 0
    while index < len(args):
        arg = args[index]
        if arg in ("-l", "--list-types") and tool == "wl-paste":
            listing = True
        elif arg in ("-t", "--type", "-target"):
            index += 1
            if index >= len(args):
                raise RuntimeError("missing clipboard type")
            mime = args[index]
        elif arg.startswith("--type="):
            mime = arg.split("=", 1)[1]
        elif arg in ("-selection", "-sel") and tool == "xclip":
            index += 1
            if index >= len(args) or args[index].lower() not in ("clipboard", "c"):
                raise RuntimeError("only the CLIPBOARD selection is supported")
        elif arg in ("-o", "-out", "--out", "-n", "--no-newline"):
            pass
        else:
            raise RuntimeError(f"unsupported {tool} option: {arg}")
        index += 1
    listing = listing or mime == "TARGETS"
    if listing:
        data = request_host({"op": "types"})
    else:
        if mime not in (None, "image", *IMAGE_TYPES):
            raise RuntimeError("this bridge exports images only")
        data = request_host({"op": "read", "type": mime})
    sys.stdout.buffer.write(data)



def main():
    tool = Path(sys.argv[0]).name
    try:
        if tool not in ("wl-paste", "xclip"):
            raise RuntimeError("invoke this container shim as wl-paste or xclip")
        client(tool, sys.argv[1:])
    except (OSError, ValueError, RuntimeError) as error:
        print(f"{tool}: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
