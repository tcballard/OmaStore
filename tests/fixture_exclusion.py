"""Ordinary core binaries neither load nor embed the synthetic catalogue."""
from pathlib import Path
import subprocess
import sys

binary = sys.argv[1]
result = subprocess.run([binary, "--stdio", "--demo"], input=b"", capture_output=True, timeout=5)
assert result.returncode == 2, "production binary accepted development mode"
assert b"Example Workshop" not in Path(binary).read_bytes(), "fixture catalogue was embedded"
print("Production fixture exclusion passed")
