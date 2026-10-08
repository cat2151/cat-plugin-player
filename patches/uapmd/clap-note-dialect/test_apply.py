"""Run with --source <upstream CLAP dir> --work-dir <global evidence dir>."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import shutil
import sys

sys.dont_write_bytecode = True

parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, required=True)
parser.add_argument("--work-dir", type=Path, required=True)
args = parser.parse_args()
spec = importlib.util.spec_from_file_location("repair", Path(__file__).with_name("apply.py"))
repair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repair)
args.work_dir.mkdir(parents=True, exist_ok=True)
source = args.work_dir / "input"
source.mkdir()
for path in args.source.iterdir():
    if path.is_file():
        shutil.copyfile(path, source / path.name)

def snapshot(directory):
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.iterdir() if p.is_file()}

before = snapshot(source)
output = args.work_dir / "generated"
names = repair.generate(source, output)
assert len(names) == 9, names
first = snapshot(output)
times = {p.name: p.stat().st_mtime_ns for p in output.iterdir()}
assert repair.generate(source, output) == names
assert snapshot(source) == before, "input changed"
assert snapshot(output) == first, "reapply differs"
assert {p.name: p.stat().st_mtime_ns for p in output.iterdir()} == times, "reapply rewrote identical files"
for name in repair.BASELINES:
    path = source / name
    path.write_bytes(path.read_bytes().replace(b"\n", b"\r\n"))
crlf = snapshot(source)
assert repair.generate(source, output) == names
assert snapshot(source) == crlf and snapshot(output) == first, "CRLF input was not reproducible/nonmutating"
header = (output / "PluginFormatCLAP.hpp").read_text(encoding="utf-8")
assert "inputNoteDialects" in header and "has_note_ports" in header
for name in names:
    assert str(output.resolve().as_posix()) in (output / name).read_text(encoding="utf-8")
changed = source / "PluginInstanceCLAP.Events.cpp"
changed.write_bytes(changed.read_bytes() + b"\n// upstream changed\n")
try:
    repair.generate(source, output)
except RuntimeError as error:
    message = str(error)
    assert all(text in message for text in ("clap-note-dialect", str(changed.resolve()), "baseline SHA256", "revision", "README.md")), message
else:
    raise AssertionError("upstream changed input was accepted")
assert snapshot(output) == first, "failed application wrote partial output"
print("fresh input, reapply, LF/CRLF reproducibility, unchanged source/output, upstream change stop diagnostics PASS")
