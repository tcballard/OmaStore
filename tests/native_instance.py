"""Exercise two real GUI invocations on an isolated session bus."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

binary = str(Path(sys.argv[1]).resolve())
if "--inside" not in sys.argv:
    runner = shutil.which("dbus-run-session")
    if not runner:
        print("SKIP: session-bus test runner unavailable")
        sys.exit(77)
    result = subprocess.run([runner, "--", sys.executable, __file__, binary, "--inside"], capture_output=True, timeout=25)
    error = result.stderr.decode(errors="replace")
    if result.returncode and ("Operation not permitted" in error or "Failed to bind socket" in error):
        print("SKIP: this environment cannot create the isolated session bus")
        sys.exit(77)
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(result.stderr)
    sys.exit(result.returncode)

with tempfile.TemporaryDirectory() as root:
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software",
               XDG_CONFIG_HOME=root + "/config", XDG_DATA_HOME=root + "/data",
               XDG_STATE_HOME=root + "/state", XDG_RUNTIME_DIR=root + "/runtime")
    Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700)
    first = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        time.sleep(0.8)
        assert first.poll() is None, "primary window did not stay running"
        second = subprocess.run([binary, "omastore://app/unavailable-app"], env=env, capture_output=True, timeout=5)
        assert second.returncode == 0, second.stderr.decode(errors="replace")
        assert first.poll() is None, "identity handoff replaced the primary process"
        invalid = subprocess.run([binary, "https://example.com"], env=env, capture_output=True, timeout=5)
        assert invalid.returncode == 2, "external origin accepted as an OmaStore handoff"
        print("PASS: second invocation forwarded its identity and exited; primary remained running")
    finally:
        first.terminate()
        try:
            first.communicate(timeout=4)
        except subprocess.TimeoutExpired:
            first.kill()
            first.communicate(timeout=2)
