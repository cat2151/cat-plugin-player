"""Generate PluginScanTool.cpp whose first-time data-directory publication is locked."""
import hashlib
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
NAME = "PluginScanTool.cpp"
BASELINE = "586fa326f95ecf72878f49fee2c5798b0e87f071662cb9d0dd12403e33733713"
RACE = '''    if (auto& supplied = applicationDataDirectory(); !supplied.empty())
        dir = supplied;
    else if (!dir.empty())
        applicationDataDirectory(dir);
'''
LOCKED = '''    {
        // Hosts created on several threads at once must not publish the process-global path concurrently.
        static std::mutex application_data_directory_mutex;
        std::lock_guard lock{application_data_directory_mutex};
        if (auto& supplied = applicationDataDirectory(); !supplied.empty())
            dir = supplied;
        else if (!dir.empty())
            applicationDataDirectory(dir);
    }
'''


def generate(upstream, output):
    upstream, output = Path(upstream).resolve(), Path(output).resolve()
    current = upstream / NAME
    try:
        if output == upstream or upstream in output.parents:
            raise ValueError("output must be outside dependency checkout")
        raw = current.read_bytes().replace(b"\r\n", b"\n")
        if hashlib.sha256(raw).hexdigest() != BASELINE:
            raise ValueError("upstream baseline SHA256 mismatch; inspect upstream fix/change")
        text = raw.decode("utf-8")
        if text.count(RACE) != 1:
            raise ValueError("expected exactly one data-directory publication")
        text = text.replace(RACE, LOCKED)
        text = text.replace("#include <cstring>", "#include <cstring>\n#include <mutex>", 1)
        # The copy lives outside src/scanner, so its relative includes become absolute.
        ipc = (upstream.parent / "ipc").as_posix()
        text = text.replace('#include "../ipc/', f'#include "{ipc}/')
        output.mkdir(parents=True, exist_ok=True)
        path = output / NAME
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            path.write_text(text, encoding="utf-8", newline="\n")
        return path
    except (OSError, ValueError) as error:
        revision = subprocess.run(["git", "-C", str(upstream), "rev-parse", "HEAD"],
                                  capture_output=True, text=True).stdout.strip() or "unknown"
        raise RuntimeError(f"[scan-tool-data-dir-race] NOT applied; target: {current}; condition: {error}; "
                           f"UAPMD revision: {revision}; instructions: {HERE / 'README.md'}") from error


if __name__ == "__main__":
    try:
        print(generate(*sys.argv[1:]))
    except (RuntimeError, ValueError) as error:
        sys.exit(str(error))
