"""Regression of fail-closed generation of the locked data-directory publication."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import shutil

parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, required=True, help="uapmd-plugin-hosting/src/scanner")
parser.add_argument("--work-dir", type=Path, required=True)
args = parser.parse_args()
here = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("scan_repair", here / "apply.py")
repair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repair)

source = args.work_dir / "src/scanner"
shutil.copytree(args.source, source)
output = args.work_dir / "generated"
before = hashlib.sha256((source / repair.NAME).read_bytes()).hexdigest()
result = repair.generate(source, output)
first = result.read_bytes()
stamp = result.stat().st_mtime_ns
repair.generate(source, output)
assert result.read_bytes() == first and result.stat().st_mtime_ns == stamp
assert hashlib.sha256((source / repair.NAME).read_bytes()).hexdigest() == before
assert first.count(b"std::lock_guard lock{application_data_directory_mutex};") == 1
assert b"#include <mutex>" in first and b'#include "../ipc/' not in first

path = source / repair.NAME
original = path.read_bytes()
path.write_bytes(original.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n"))
repair.generate(source, output)
assert result.read_bytes() == first
path.write_bytes(original + b"// upstream change\n")
try:
    repair.generate(source, output)
except RuntimeError as error:
    assert all(word in str(error) for word in ("scan-tool-data-dir-race", str(path.resolve()), "condition", "revision", "README.md"))
else:
    raise AssertionError("unexpected input accepted")
assert result.read_bytes() == first
print("fresh/reapply timestamp, input preservation, CRLF, drift stop PASS")
