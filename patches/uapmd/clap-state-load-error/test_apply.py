"""Regression of fail-closed generation and ordered single-source CMake replacement."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, required=True)
parser.add_argument("--work-dir", type=Path, required=True)
args = parser.parse_args()
here = Path(__file__).resolve().parent


def module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


repair = module(here / "apply.py", "state_repair")
note = module(here.parent / "clap-note-dialect/apply.py", "note_repair")
args.work_dir.mkdir(parents=True, exist_ok=True)
source = args.work_dir / "input"
shutil.copytree(args.source, source)
note_dir = args.work_dir / "note"
output = args.work_dir / "state"


def snapshot(path):
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in path.iterdir() if p.is_file()}


before = snapshot(source)
note.generate(source, note_dir)
note_before = snapshot(note_dir)
result = repair.generate(source, note_dir, output)
first = result.read_bytes()
stamp = result.stat().st_mtime_ns
repair.generate(source, note_dir, output)
assert result.read_bytes() == first and result.stat().st_mtime_ns == stamp
assert snapshot(source) == before and snapshot(note_dir) == note_before
for p in source.glob("*.*"):
    if p.suffix in (".cpp", ".hpp"):
        p.write_bytes(p.read_bytes().replace(b"\r\n", b"\n").replace(b"\n", b"\r\n"))
note.generate(source, note_dir)
repair.generate(source, note_dir, output)
assert result.read_bytes() == first
assert f'#include "{(note_dir.resolve() / "PluginFormatCLAP.hpp").as_posix()}"'.encode() in first


def rejected(path, mutation):
    original = path.read_bytes()
    path.write_bytes(mutation(original))
    try:
        repair.generate(source, note_dir, output)
    except RuntimeError as error:
        assert all(word in str(error) for word in ("clap-state-load-error", str(path.resolve()), "condition", "revision", "README.md"))
    else:
        raise AssertionError("unexpected input accepted")
    assert result.read_bytes() == first
    path.write_bytes(original)


rejected(source / repair.NAME, lambda b: b + b"// upstream change\n")
rejected(note_dir / repair.NAME, lambda b: b.replace(b'finish("")', b'finish("changed")'))
# Tiny configure fixture runs the actual note->state CMake includes.
fixture = args.work_dir / "cmake-input"
fixture.mkdir()
fake_uapmd = fixture / "uapmd"
shutil.copytree(source, fake_uapmd / "source/remidy/src/clap")
names = note.generate(source, note_dir)
source_list = "\n".join('"${UAPMD_DIR}/source/remidy/src/clap/' + name + '"' for name in names)
(fixture / "CMakeLists.txt").write_text(f'''cmake_minimum_required(VERSION 3.28)
project(state_apply_fixture LANGUAGES CXX)
set(UAPMD_DIR "{fake_uapmd.as_posix()}")
add_library(remidy STATIC {source_list})
target_include_directories(remidy PRIVATE "${{UAPMD_DIR}}/source/remidy/src/clap")
include("{(here.parent / 'clap-note-dialect/CMakeLists.cmake').as_posix()}")
include("{(here / 'CMakeLists.cmake').as_posix()}")
get_target_property(result remidy SOURCES)
file(WRITE "${{CMAKE_BINARY_DIR}}/sources.txt" "${{result}}")
''', encoding="utf-8")
build = args.work_dir / "cmake-output"
configured = subprocess.run(["cmake", "-S", str(fixture), "-B", str(build)],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
if configured.returncode:
    raise RuntimeError(configured.stdout)
sources = (build / "sources.txt").read_text().split(";")
states = [s for s in sources if s.endswith(repair.NAME)]
assert len(states) == 1 and "/generated/clap-state-load-error/" in states[0], states
assert len(sources) == len(names)
cmake_file = fixture / "CMakeLists.txt"
script = cmake_file.read_text()
script = script.replace(f'include("{(here / "CMakeLists.cmake").as_posix()}")',
                        'get_target_property(duplicates remidy SOURCES)\n'
                        'list(APPEND duplicates "${CMAKE_BINARY_DIR}/generated/clap-note-dialect/PluginInstanceCLAP.States.cpp")\n'
                        'set_property(TARGET remidy PROPERTY SOURCES "${duplicates}")\n'
                        f'include("{(here / "CMakeLists.cmake").as_posix()}")')
cmake_file.write_text(script, encoding="utf-8")
rejected_config = subprocess.run(["cmake", "-S", str(fixture), "-B", str(build)],
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
assert rejected_config.returncode and all(s in rejected_config.stdout for s in
    ("clap-state-load-error", "found 2/0", "revision", "README.md")), rejected_config.stdout
print("LF/CRLF, fresh/reapply timestamps, input preservation, drift stops, note header and ordered single-source CMake PASS")
