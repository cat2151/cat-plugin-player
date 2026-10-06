"""Check reapplication and visible failures without editing the UAPMD checkout."""
from pathlib import Path
import hashlib
import shutil
import subprocess
import tempfile

PATCH = Path(__file__).resolve().parent
ROOT = PATCH.parents[2]
BASE = ROOT / "target" / "shim" / "patch-tests"
BASE.mkdir(parents=True, exist_ok=True)
WORK = Path(tempfile.mkdtemp(prefix="vst3-", dir=BASE))
SOURCE = '''#include "ClassModuleInfo.hpp"
#include "../utils.hpp"
        if (!is_directory(pluginPath)) // self-contained plugin DLL
            return pluginPath;
        for (auto& entry : std::filesystem::directory_iterator(binDir))
            return entry.path();
        return {};
'''


def apply(source, output):
    driver = WORK / "apply-test.cmake"
    driver.write_text(
        f'include("{(PATCH / "apply.cmake").as_posix()}")\n'
        f'uh_patch_windows_vst3_loader("{source.as_posix()}" "{output.as_posix()}")\n',
        encoding="utf-8",
    )
    return subprocess.run(["cmake", "-P", str(driver)], capture_output=True, text=True)


try:
    # Separate checkout directories emulate a fresh clone, with no generated cache.
    for name in ("checkout-one", "checkout-two"):
        source = WORK / name / "ClassModuleInfo.cpp"
        source.parent.mkdir()
        source.write_bytes(SOURCE.replace("\n", "\r\n").encode())
        original = hashlib.sha256(source.read_bytes()).digest()
        output = WORK / "generated" / name / "ClassModuleInfo.cpp"
        result = apply(source, output)
        assert result.returncode == 0, result.stdout + result.stderr
        assert "uh::windowsVst3Binary" in output.read_text()
        assert hashlib.sha256(source.read_bytes()).digest() == original
        first = output.read_bytes()
        result = apply(source, output)
        assert result.returncode == 0, result.stdout + result.stderr
        assert output.read_bytes() == first

    for label, content in (
        ("upstream-changed", SOURCE.replace("return entry.path();", "return other.path();")),
        ("duplicate-selector", SOURCE + SOURCE[SOURCE.index("        for (auto&"):]),
        ("changed-include", SOURCE.replace('"../utils.hpp"', '"different.hpp"')),
        ("missing-source", None),
    ):
        source = WORK / label / "ClassModuleInfo.cpp"
        source.parent.mkdir()
        if content is not None:
            source.write_text(content, encoding="utf-8")
        output = WORK / "generated" / label / "ClassModuleInfo.cpp"
        result = apply(source, output)
        assert result.returncode != 0, label
        diagnostic = result.stdout + result.stderr
        for expected in ("windows-vst3-loader", source.as_posix(), "Failed condition:",
                         "UAPMD revision:", "Source SHA256:", "README.md", "Do not bypass"):
            assert expected in diagnostic, (label, expected, diagnostic)
        assert not output.exists(), label
    print("PASS: fresh checkout, repeat application, checkout unchanged, drift/duplicate/missing diagnostics")
finally:
    shutil.rmtree(WORK)
