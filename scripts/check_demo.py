"""Run the documented server and curl checks without touching an existing listener."""
import os
from pathlib import Path
import queue
import signal
import subprocess
import threading

root = Path(__file__).resolve().parents[1]
server = subprocess.Popen(
    ["cargo", "+stable", "run", "--release", "--", "127.0.0.1:0"],
    cwd=root, stdout=subprocess.PIPE, text=True, start_new_session=True,
)
lines = queue.Queue()

def read_lines():
    for line in server.stdout:
        lines.put(line.strip())

threading.Thread(target=read_lines, daemon=True).start()
try:
    while True:
        line = lines.get(timeout=15)
        if line.startswith("Listening on "):
            address = line.removeprefix("Listening on ")
            break
    for method, path, status, body, allow in [
        ("GET", "/", 200, (root / "hello.html").read_bytes(), None),
        ("GET", "/health?probe=1", 200, b"ok\n", None),
        ("POST", "/echo", 200, b"abc", None),
        ("GET", "/missing", 404, b"Not found\n", None),
        ("PUT", "/health", 405, b"Method not allowed\n", b"GET"),
        ("PATCH", "/", 501, b"Request rejected\n", None),
    ]:
        command = ["curl", "--silent", "--show-error", "--include", "--max-time", "3", "--request", method]
        if method == "POST":
            command += ["--data-binary", "abc"]
        result = subprocess.run(command + [f"http://{address}{path}"], check=True, capture_output=True, timeout=5)
        head, actual_body = result.stdout.split(b"\r\n\r\n", 1)
        assert head.startswith(f"HTTP/1.1 {status} ".encode()), head
        assert actual_body == body, actual_body
        fields = dict(line.split(b": ", 1) for line in head.split(b"\r\n")[1:])
        assert fields[b"Content-Length"] == str(len(body)).encode()
        assert fields[b"Connection"] == b"close"
        if allow:
            assert fields[b"Allow"] == allow
        print(f"{method} {path}: {status}, {len(body)} body bytes, Connection: close")
finally:
    # Only terminate the process group this script created. Signals are not graceful shutdown.
    os.killpg(server.pid, signal.SIGTERM)
    server.wait(timeout=5)
