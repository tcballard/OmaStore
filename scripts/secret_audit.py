"""Scan project source for concrete credential formats; report locations, never matches."""
import json
from pathlib import Path
import re
import subprocess

rules = {
    "private_key": re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    "github_token": re.compile(rb"(?:ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{70,})"),
    "stripe_secret": re.compile(rb"sk_(?:live|test)_[A-Za-z0-9]{20,}"),
    "aws_access_key": re.compile(rb"AKIA[0-9A-Z]{16}"),
    "slack_token": re.compile(rb"xox[baprs]-[0-9A-Za-z-]{25,}"),
}
paths = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"]).split(b"\0")
findings = []
for raw in paths:
    if not raw:
        continue
    path = Path(raw.decode())
    if not path.is_file() or path.is_symlink():
        continue
    body = path.read_bytes()
    for rule, pattern in rules.items():
        for match in pattern.finditer(body):
            findings.append({"file": str(path), "line": body.count(b"\n", 0, match.start()) + 1, "rule": rule})
print(json.dumps({"patternsChecked": list(rules), "findings": findings, "scope": "Tracked and unignored project files; format scan, not a guarantee that every secret format is recognized."}, indent=2))
raise SystemExit(1 if findings else 0)
