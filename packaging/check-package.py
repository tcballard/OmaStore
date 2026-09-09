"""Check an Arch preview archive without installing it or running package hooks."""
import subprocess
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: check-package.py PACKAGE.pkg.tar.zst")
result = subprocess.run(["bsdtar", "-tf", sys.argv[1]], check=True, capture_output=True, text=True, timeout=30)
paths = {path.removeprefix("./") for path in result.stdout.splitlines()}
required = {
    ".PKGINFO", ".BUILDINFO", "usr/bin/omastore", "usr/bin/omastore-core",
    "usr/share/applications/io.github.tcballard.OmaStore.desktop",
    "usr/share/metainfo/io.github.tcballard.OmaStore.metainfo.xml",
    "usr/share/licenses/omastore/LICENSE",
}
if missing := required - paths:
    raise SystemExit("Package is missing: " + ", ".join(sorted(missing)))
if any(path.startswith(("home/", "root/", "etc/")) or ".." in path.split("/") for path in paths):
    raise SystemExit("Unexpected private/configuration path in package")
if any(path.endswith(("omastore-service", "native-contracts")) for path in paths):
    raise SystemExit("Desktop package contains service or test executables")
print("Desktop package contains the sibling binaries and launcher metadata")
