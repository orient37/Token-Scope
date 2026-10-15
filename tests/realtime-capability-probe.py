"""Read-only probe: never starts/resumes threads or records message contents."""
import json
import pathlib
import queue
import shutil
import subprocess
import threading
import time

exe = shutil.which("codex")
process = subprocess.Popen(
    [exe, "app-server", "--listen", "stdio://", "-c", "analytics.enabled=false"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW,
)
messages = queue.Queue()

def read():
    for line in process.stdout:
        try:
            messages.put(json.loads(line))
        except ValueError:
            pass

threading.Thread(target=read, daemon=True).start()

def send(value):
    process.stdin.write(json.dumps(value) + "\n")
    process.stdin.flush()

def response(request_id):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        value = messages.get(timeout=max(0.1, deadline - time.monotonic()))
        if value.get("id") == request_id:
            return value
    raise TimeoutError("App Server response timeout")

try:
    send({"id": 1, "method": "initialize", "params": {
        "clientInfo": {"name": "tokenscope", "version": "0.5.16"}}})
    if "error" in response(1):
        raise RuntimeError("initialize rejected")
    send({"method": "initialized"})
    send({"id": 2, "method": "thread/loaded/list", "params": {}})
    result = response(2)
    report = {
        "probe": "separate_app_server_loaded_threads", "read_only": True,
        "success": "result" in result,
        "loaded_count": len(result.get("result", {}).get("data", [])),
        "started_or_resumed_threads": False,
    }
    pathlib.Path(".local/realtime-capability-v0516.json").write_text(
        json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report))
finally:
    process.terminate()
    process.wait(timeout=10)
