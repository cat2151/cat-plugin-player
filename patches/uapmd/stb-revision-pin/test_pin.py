"""Real FetchContent regression; all generated files stay in --work-dir."""
import argparse
import os
from pathlib import Path
import subprocess

PIN = "2c980bb59875b0d32144a71867fbdebb2f77cd20"
PATCH = Path(__file__).resolve().with_name("CMakeLists.cmake").as_posix()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--stb-source", type=Path, required=True)
    args = parser.parse_args()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="url." + args.stb_source.resolve().as_uri() + ".insteadOf",
               GIT_CONFIG_VALUE_0="https://github.com/nothings/stb")
    base = f'''cmake_minimum_required(VERSION 3.24)
project(pin_fixture NONE)
set(UAPMD_DIR "${{CMAKE_CURRENT_SOURCE_DIR}}")
include("{PATCH}")
# Later upstream declaration must lose, even with an invalid repository/tag.
FetchContent_Declare(stb GIT_REPOSITORY https://invalid.example/stb GIT_TAG invalid)
FetchContent_Populate(stb)
add_library(stb INTERFACE)
add_library(stb::stb ALIAS stb)
target_include_directories(stb INTERFACE "$<BUILD_INTERFACE:${{stb_SOURCE_DIR}}>")
uh_stb_verify()
'''

    def configure(name, text=base, extra=(), failure=None):
        source = work / name / "source"
        build = work / name / "build"
        source.mkdir(parents=True, exist_ok=True)
        (source / "CMakeLists.txt").write_text(text, encoding="utf-8")
        result = subprocess.run(["cmake", "-S", str(source), "-B", str(build), *extra],
                                env=env, capture_output=True, text=True)
        (work / (name + ".log")).write_text(result.stdout + result.stderr, encoding="utf-8")
        if failure:
            assert result.returncode != 0 and failure in result.stdout + result.stderr, result.stdout + result.stderr
        else:
            assert result.returncode == 0, result.stdout + result.stderr
        return build

    build = configure("precedence")
    actual = subprocess.check_output(["git", "-C", str(build / "_deps/stb-src"), "rev-parse", "HEAD"], text=True).strip()
    assert actual == PIN
    configure("precedence")  # clean existing cache is safe to reuse
    configure("override", extra=["-DFETCHCONTENT_SOURCE_DIR_STB=" + args.stb_source.resolve().as_posix()],
              failure="source override")
    configure("missing", base.replace("uh_stb_verify()", 'file(REMOVE "${stb_SOURCE_DIR}/stb_sprintf.h")\nuh_stb_verify()'),
              failure="required root stb_sprintf.h missing")
    configure("target", base.replace(f'include("{PATCH}")', f'add_library(stb INTERFACE)\ninclude("{PATCH}")'),
              failure="pre-existing target")
    configure("structure", base.replace('target_include_directories(stb INTERFACE "$<BUILD_INTERFACE:${stb_SOURCE_DIR}>")', ""),
              failure="no longer exports")
    old_build = configure("old-revision")
    old_source = old_build / "_deps/stb-src"
    subprocess.run(["git", "-C", str(old_source), "checkout", "--detach", "HEAD^"],
                   check=True, capture_output=True)
    before = subprocess.check_output(["git", "-C", str(old_source), "rev-parse", "HEAD"])
    configure("old-revision", failure="expected clean checkout")
    assert subprocess.check_output(["git", "-C", str(old_source), "rev-parse", "HEAD"]) == before
    cache = build / "_deps/stb-src"
    (cache / "unexpected.txt").write_text("preserve me", encoding="utf-8")
    configure("precedence", failure="expected clean checkout")
    assert (cache / "unexpected.txt").read_text() == "preserve me"
    print("PASS: first declaration, clean cache, override, missing header, target, structure, wrong-SHA and dirty-cache preservation")


if __name__ == "__main__":
    main()
