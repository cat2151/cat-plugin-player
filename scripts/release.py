"""Publish target/release/cat-plugin-player.exe as a GitHub Release (prototype: dry-run only).

Usage:
    python scripts/release.py            # dry-run: run read-only checks and print the plan

The build depends on sibling checkouts (ADR 0012), so the release is built locally,
not in CI. Read-only checks run for real; build, packaging and `gh release create`
are only printed.
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = ROOT / "target" / "release" / "cat-plugin-player.exe"
SIBLINGS = ["uapmd", "clap-mml-render-tui", "clap-mml-play-server"]
BRANCH = "main"


def run(args, cwd=ROOT):
    p = subprocess.run(args, cwd=cwd, capture_output=True, text=True, encoding="utf-8")
    return p.returncode, (p.stdout + p.stderr).strip()


class Report:
    def __init__(self):
        self.failures = []

    def check(self, ok, label, detail=""):
        print(f"  [{'OK' if ok else 'NG'}] {label}" + (f": {detail}" if detail else ""))
        if not ok:
            self.failures.append(label)


def repo_state(path):
    """(HEAD sha, number of uncommitted changes to tracked files) of a git checkout."""
    _, sha = run(["git", "rev-parse", "HEAD"], cwd=path)
    _, status = run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=path)
    return sha, len(status.splitlines())


def leak_prefixes():
    """Local path prefixes that must not appear in a published binary."""
    prefixes = {Path.home(), ROOT.parent}
    for var in ["CARGO_HOME", "RUSTUP_HOME"]:
        if os.environ.get(var):
            prefixes.add(Path(os.environ[var]))
    # Drop prefixes nested in another one; they would be counted twice.
    return sorted(str(p) for p in prefixes if not any(q != p and q in p.parents for q in prefixes))


# MSVC `assert` in C sources embeds file names as UTF-16LE, so ASCII alone misses them.
ENCODINGS = {"ASCII": "utf-8", "UTF-16LE": "utf-16-le"}


def find_leaks(data, prefix):
    """{encoding label: sorted (start, end) of `prefix` in `data`}, either separator, any case."""
    variants = {prefix, prefix.replace("\\", "/")}
    return {
        label: sorted(m.span() for v in variants
                      for m in re.finditer(re.escape(v.encode(enc)), data, re.IGNORECASE))
        for label, enc in ENCODINGS.items()
    }


def leak_context(data, span, label, width=24):
    """Decoded text around one match, with the prefix itself masked."""
    enc = ENCODINGS[label]
    unit = len("a".encode(enc))
    start, end = span

    def text(b):
        # Bytes next to a match need not be in the same encoding; show only printable ASCII.
        return "".join(c if " " <= c <= "~" else "." for c in b.decode(enc, "replace"))

    return (text(data[max(0, start - width * unit):start]) + "<prefix>"
            + text(data[end:end + width * unit]))


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.parse_args()

    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    tag = f"v{version}"
    asset = f"cat-plugin-player-{tag}-windows-x64.zip"
    r = Report()

    print(f"== release {tag} (dry-run) ==\n")
    print("1. 事前確認（実際に実行）")
    code, out = run(["gh", "auth", "status"])
    r.check(code == 0, "gh にログイン済み")

    _, branch = run(["git", "branch", "--show-current"])
    r.check(branch == BRANCH, f"branch が {BRANCH}", branch)

    head, dirty = repo_state(ROOT)
    r.check(dirty == 0, "cat-plugin-player に未commit変更なし", f"{dirty} 件")
    # origin/main may be ahead (README auto-translation); HEAD only has to be in it.
    _, status = run(["gh", "api", f"repos/{{owner}}/{{repo}}/compare/{head}...{BRANCH}",
                     "--jq", ".status"])
    r.check(status in ("identical", "ahead"), f"HEAD {head[:8]} が GitHub の {BRANCH} に含まれる",
            status)

    code, _ = run(["git", "ls-remote", "--exit-code", "--tags", "origin", f"refs/tags/{tag}"])
    r.check(code != 0, f"tag {tag} が origin に未作成")
    code, _ = run(["gh", "release", "view", tag])
    r.check(code != 0, f"release {tag} が未作成")

    states = {"cat-plugin-player": (head, dirty)}
    for name in SIBLINGS:
        path = ROOT.parent / name
        if not (path / ".git").exists():
            r.check(False, f"兄弟 checkout {name} が存在", str(path))
            continue
        states[name] = repo_state(path)
        r.check(states[name][1] == 0, f"{name} に未commit変更なし", f"{states[name][1]} 件")

    r.check(EXE.exists(), "release exe が存在（既存ビルドを検査）")
    if EXE.exists():
        data = EXE.read_bytes()
        for prefix in leak_prefixes():
            # The prefix itself is a local path; print only its drive and depth.
            shown = f"{Path(prefix).drive}\\…（{len(Path(prefix).parts) - 1} 階層）"
            for label, spans in find_leaks(data, prefix).items():
                r.check(not spans, f"exe に個人パス {shown}（{label}）が埋め込まれていない",
                        f"{len(spans)} 箇所")
                for span in spans[:3]:
                    print(f"       {leak_context(data, span, label)}")

    print("\n2. 実行予定（dry-run のため実行しない）")
    print("   cargo build --release")
    print("   （ビルド後の exe で上記の個人パス検査をやり直す）")
    print(f"   zip {asset} <- cat-plugin-player.exe, LICENSE   （.pdb は含めない）")
    print(f"   gh release create {tag} {asset} --target {head} "
          f"--title {tag} --notes-file <下記ノート>")

    print("\n3. release ノート案")
    print(f"   cat-plugin-player {tag} (Windows x64)")
    print("   - 動作要件: Microsoft Visual C++ 再頒布可能パッケージ（VCRUNTIME140 / MSVCP140）")
    print("   - ビルド時の各repo:")
    for name, (sha, n) in states.items():
        print(f"     - {name} {sha[:12]}" + (f"（未commit {n} 件）" if n else ""))
    if EXE.exists():
        print(f"   - 既存 exe の sha256（参考・再ビルド後に変わる）: "
              f"{hashlib.sha256(EXE.read_bytes()).hexdigest()}")

    print()
    if r.failures:
        print(f"NG {len(r.failures)} 件: 本番実行ならここで停止する")
        return 1
    print("全確認 OK（本番実行は未実装）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
