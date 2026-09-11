"""Exercise the actual service and core processes; never publish the test data."""
import json
import xml.etree.ElementTree as ET
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request

service, core, catalogue = sys.argv[1:4]
demo = len(sys.argv) > 4 and sys.argv[4] == "--demo"
with tempfile.TemporaryDirectory() as directory:
    source = Path(directory) / "catalogue.json"
    source.write_bytes(Path(catalogue).read_bytes())
    server = subprocess.Popen([service, str(source), "127.0.0.1:0", "--workspace", str(Path(directory) / "private" / "workflow.db")], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=dict(os.environ, OMASTORE_MONITORING_PAUSED="1", OMASTORE_CHECKS_PAUSED="1", OMASTORE_PUBLICATION_PAUSED="1"))
    try:
        if not select.select([server.stdout], [], [], 8)[0]:
            raise AssertionError("service startup timeout")
        line = server.stdout.readline().strip()
        if not line.startswith("LISTENING "):
            stderr = server.communicate(timeout=3)[1]
            if "Operation not permitted" in stderr or "Permission denied" in stderr:
                print("SKIP: this environment forbids listening sockets; the HTTP process test must run in CI")
                sys.exit(77)
            raise AssertionError(stderr)
        base = "http://" + line.removeprefix("LISTENING ")

        def get(path, headers=None, method="GET"):
            request = urllib.request.Request(base + path, headers=headers or {}, method=method)
            try:
                response = urllib.request.urlopen(request, timeout=5)
            except urllib.error.HTTPError as error:
                response = error
            with response:
                return response.status, response.headers, response.read()

        status, headers, body = get("/api/v1/catalogue")
        assert status == 200
        assert get("/api/v1/catalogue", {"If-None-Match": headers["ETag"]})[0] == 304
        assert get("/api/v1/apps?price=cheap")[0] == 400
        assert get("/api/v1/catalogue", method="POST")[0] == 405
        rss_status, rss_headers, rss_body = get("/feed.xml")
        assert rss_status == 200 and rss_headers["Content-Type"].startswith("application/rss+xml")
        root = ET.fromstring(rss_body)
        assert root.tag == "rss" and root.find("channel") is not None
        assert root.findall("channel/item") == []
        assert get("/feed.xml", {"If-None-Match": rss_headers["ETag"]})[0] == 304
        remote = json.loads(get("/api/v1/apps")[2])
        env = dict(os.environ, XDG_CACHE_HOME=directory)
        request = json.dumps(dict(protocol_version=1, id="query", method="apps.list")) + "\n"
        result = subprocess.run([core, "--stdio"] + (["--demo"] if demo else []), input=request, capture_output=True, text=True, env=env, timeout=5)
        local = json.loads(result.stdout)["result"]
        assert local["snapshot"] == remote["snapshot"]
        assert local["items"] == remote["items"]
        if demo:
            page = json.loads(get("/api/v1/apps?limit=1")[2])
            changed = json.loads(source.read_text())
            changed["revision"] = "changed-snapshot"
            replacement = source.with_suffix(".new")
            replacement.write_text(json.dumps(changed))
            replacement.replace(source)
            assert get("/api/v1/apps?limit=1&cursor=" + page["nextCursor"])[0] == 409
        source.write_text("interrupted invalid publication")
        assert get("/api/v1/catalogue")[0] == 503
        print("HTTP/core parity, conditional requests, errors and snapshot replacement passed")
    finally:
        server.terminate()
        server.communicate(timeout=5)
