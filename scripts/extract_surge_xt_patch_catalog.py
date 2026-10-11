"""Extract the Surge XT part of the cmrt patch catalog cache, keeping only patches present on disk.

Usage:
    python scripts/extract_surge_xt_patch_catalog.py <output.json> [<catalog.json>]

<catalog.json> defaults to %LOCALAPPDATA%\\clap-mml-render-tui\\patch-catalog\\catalog.json.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

SURGE_PLUGIN_ID = "org.surge-synth-team.surge-xt"
SURGE_PLUGIN_KEY = "id:" + SURGE_PLUGIN_ID
ALLOWED_PATH_PREFIXES = ("C:\\Program Files\\", "C:\\ProgramData\\")
DRIVE_PATH = re.compile(r"^[A-Za-z]:[\\/]")
REBUILD_COMMAND = "cmrt build-patch-catalog-cache"


def default_catalog_path():
    return Path(os.environ["LOCALAPPDATA"]) / "clap-mml-render-tui" / "patch-catalog" / "catalog.json"


def disk_key(path):
    """Compare paths the way Windows does: separator- and case-insensitive."""
    return str(path).replace("\\", "/").lower()


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from strings(item)


def winget_version():
    try:
        out = subprocess.run(["winget", "list", "--id", "SurgeSynth.SurgeXT", "--disable-interactivity"],
                             capture_output=True, text=True, encoding="utf-8", errors="replace").stdout
    except FileNotFoundError:
        return "(winget not found)"
    for line in out.splitlines():
        if "SurgeSynth.SurgeXT" in line:
            parts = line.split()
            index = parts.index("SurgeSynth.SurgeXT")
            return parts[index + 1] if index + 1 < len(parts) else line.strip()
    return "(not installed via winget)"


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    source = Path(sys.argv[2]) if len(sys.argv) == 3 else default_catalog_path()
    with open(source, encoding="utf-8") as f:
        catalog = json.load(f)

    plugins = [p for p in catalog["plugins"] if p.get("plugin_id") == SURGE_PLUGIN_ID]
    if len(plugins) != 1:
        sys.exit(f"expected exactly 1 Surge XT plugin in {source}, found {len(plugins)}")
    plugin = plugins[0]
    base = Path(plugin["base"]["Shared"])

    patches = [p for p in catalog["patches"] if p["audio"]["reference"]["plugin"] == SURGE_PLUGIN_KEY]
    kept = [p for p in patches if (base / p["audio"]["reference"]["display"]).is_file()]
    dropped = len(patches) - len(kept)

    cataloged = {disk_key(base / p["audio"]["reference"]["display"]) for p in kept}
    on_disk = [f for d in plugin["dirs"] if Path(d).is_dir() for f in Path(d).rglob("*.fxp")]
    missing = sorted(str(f) for f in on_disk if disk_key(f) not in cataloged)
    if missing:
        for path in missing[:20]:
            print(f"not in catalog: {path}")
        sys.exit(f"{len(missing)} .fxp file(s) on disk are not in the catalog; "
                 f"rebuild it with `{REBUILD_COMMAND}` and run this script again")

    displays = {p["audio"]["reference"]["display"] for p in kept}
    result = {
        "format_version": catalog["format_version"],
        "patches": kept,
        "plugins": [plugin],
        "patch_voicings": {k: v for k, v in catalog.get("patch_voicings", {}).items() if k in displays},
        "catalog_notes": [],
    }

    bad = sorted({s for s in strings(result) if DRIVE_PATH.match(s) and not s.startswith(ALLOWED_PATH_PREFIXES)})
    if bad:
        for path in bad[:20]:
            print(f"disallowed path: {path}")
        sys.exit(f"{len(bad)} path(s) outside {ALLOWED_PATH_PREFIXES}; refusing to write")
    text = json.dumps(result, indent=1, ensure_ascii=False) + "\n"

    with open(output, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print(f"wrote {output}")
    print(f"kept {len(kept)} patches, dropped {dropped} (not on disk), voicings {len(result['patch_voicings'])}")
    print(f"'Users' in output: {'Users' in text}")
    print(f"Surge XT version (winget): {winget_version()}")


if __name__ == "__main__":
    main()
