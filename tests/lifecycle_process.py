"""A real independent worker must survive loss of its coordinating pipe process."""
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

binary = str(Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory() as directory:
    env = dict(os.environ, XDG_STATE_HOME=directory + "/state", XDG_DATA_HOME=directory + "/data")
    processes = []

    def start():
        process = subprocess.Popen([binary, "--stdio", "--demo"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        processes.append(process)
        return process

    def request(process, method, params):
        process.stdin.write((json.dumps({"protocol_version": 1, "id": "test", "method": method, "params": params}) + "\n").encode())
        process.stdin.flush()
        assert select.select([process.stdout], [], [], 5)[0], "core response timed out"
        line = process.stdout.readline()
        assert line, "core exited before replying"
        return json.loads(line)

    try:
        core = start()
        plan = request(core, "system.plan", {"kind": "app", "id": "demo-fieldnotes"})["result"]
        consent = {"id": plan["digest"], "digest": plan["digest"], "accepted": False}
        assert not request(core, "operations.confirm", consent)["ok"]
        assert request(core, "library.list", {})["result"]["items"] == []
        consent["accepted"] = True
        accepted = request(core, "operations.confirm", consent)
        assert accepted["ok"] and accepted["result"]["state"] in ("awaiting_user", "running"), accepted
        core.kill()
        core.communicate(timeout=3)
        recovered = start()
        deadline = time.monotonic() + 7
        state = None
        while time.monotonic() < deadline:
            state = request(recovered, "operations.status", {"id": plan["digest"]})["result"]
            if state["state"] in ("succeeded", "failed", "unknown"):
                break
            time.sleep(0.1)
        assert state["state"] == "succeeded", state
        library = request(recovered, "library.list", {})["result"]
        assert library["items"][0]["present"] and not library["items"][0]["preexisting"]
        replay = request(recovered, "operations.confirm", consent)
        assert replay["ok"] and replay["result"]["state"] == "succeeded", replay
        assert request(recovered, "library.list", {})["result"]["lastSequence"] == library["lastSequence"]
        print("PASS: explicit consent, worker survival, durable version observation and replay without a second install")
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
                process.communicate(timeout=3)
