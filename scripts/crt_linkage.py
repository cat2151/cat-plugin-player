"""Check that built binaries link the C/C++ runtime statically, from `dumpbin /dependents`.

Usage:
    python scripts/crt_linkage.py <file>...   # no file may import a VC++ or UCRT DLL
"""

import re
import subprocess
import sys
from pathlib import Path

VSWHERE = Path(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe")
VC_RUNTIME = re.compile(r"^(vcruntime|msvcp|concrt)\d+.*\.dll$", re.IGNORECASE)
# With /MT the UCRT is linked statically too, so its DLLs must be absent as well.
UCRT = re.compile(r"^(api-ms-win-crt-.*|ucrtbase)\.dll$", re.IGNORECASE)


def dumpbin():
    out = subprocess.run(
        [str(VSWHERE), "-latest", "-products", "*", "-find",
         "VC/Tools/MSVC/**/bin/Hostx64/x64/dumpbin.exe"],
        capture_output=True, text=True, check=True).stdout.splitlines()
    if not out:
        sys.exit("dumpbin.exe not found (MSVC x64 tools are required)")
    return out[-1]


def dependents(tool, path):
    out = subprocess.run([tool, "/nologo", "/dependents", str(path)],
                         capture_output=True, text=True, check=True).stdout
    return sorted({line.strip() for line in out.splitlines() if line.strip().lower().endswith(".dll")})


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    tool = dumpbin()
    failed = False
    for path in map(Path, sys.argv[1:]):
        crt = [d for d in dependents(tool, path) if VC_RUNTIME.match(d) or UCRT.match(d)]
        print(f"[{'NG' if crt else 'OK'}] static: {path.name} -> CRT DLLs: {', '.join(crt) or '(none)'}")
        failed |= bool(crt)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
