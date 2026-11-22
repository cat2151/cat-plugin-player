"""Run the opt-in browser-to-memory native test without a window or audio device.

Requires installed Six Sines and its factory Templates/INIT Sine.sxsnp and INIT EP.
Supply --bundle, --factory and an external --output directory. Builds the default
target's Rust test executable; the native shim uses its normal Release build.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--factory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if output.is_relative_to(root):
        parser.error('--output must be outside the repository')
    if not args.bundle.is_file():
        parser.error('--bundle must name the installed CLAP file')
    for name in ['INIT Sine', 'INIT EP']:
        if not (args.factory / 'Templates' / f'{name}.sxsnp').is_file():
            parser.error(f'factory template missing: {name}')
    output.mkdir(parents=True, exist_ok=True)
    (output / 'browser.json').unlink(missing_ok=True)
    command = ['cargo', 'test', '--bin', 'cat-plugin-player', '--no-run', '--message-format=json']
    with (output / 'build.log').open('wb') as log:
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=log, check=True)
    messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith(b'{')]
    binary = next(m['executable'] for m in messages
                  if m.get('reason') == 'compiler-artifact' and m.get('executable')
                  and m['target']['name'] == 'cat-plugin-player' and m['profile']['test'])
    env = os.environ.copy()
    env.update({
        'PATCH_LATENCY_BUNDLE': str(args.bundle.resolve()),
        'PATCH_LATENCY_FACTORY': str(args.factory.resolve()),
        'PATCH_LATENCY_JSON': str(output / 'browser.json'),
        'S06_NATIVE_WORK_DIR': str(output / 'session'),
        'CAT_NATIVE_TEST_ROOT': str(output / 'cache'),
    })
    case = 'patch_preparation::tests::native::latency::native_browser_memory_latency_and_restart'
    with (output / 'test.log').open('wb') as log:
        subprocess.run([binary, case, '--exact', '--ignored', '--nocapture'],
                       cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT,
                       timeout=180, check=True)
    print(output / 'browser.json')


if __name__ == '__main__':
    main()
