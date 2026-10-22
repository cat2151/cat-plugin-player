"""Idempotence, composition and fail-closed regression. Scratch is run-owned."""
import hashlib
import importlib.util
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('parameter_patch', HERE / 'apply.py')
patch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(patch)


def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in Path(root).rglob('*') if p.is_file()}


def test(source, note, state, work):
    source, note, state, work = map(lambda p: Path(p).resolve(), (source, note, state, work))
    initial = {str(root): snapshot(root) for root in (source, note, state)}
    with tempfile.TemporaryDirectory(prefix='parameter-test-', dir=work) as temp:
        temp = Path(temp)
        output = temp / 'generated'
        names = patch.generate(source, note, state, output)
        first = snapshot(output)
        assert names == patch.generate(source, note, state, output)
        assert first == snapshot(output)
        for kind in ['clap', 'vst3']:
            header = output / kind / ('PluginFormatCLAP.hpp' if kind == 'clap' else 'PluginFormatVST3.hpp')
            for p in (output / kind).glob('*.cpp'):
                text = p.read_text(encoding='utf-8')
                if 'PluginFormat' in text and '#include' in text:
                    assert f'#include "{header.as_posix()}"' in text, p
        altered = temp / 'altered-note'
        shutil.copytree(note, altered)
        for p in altered.iterdir():
            p.write_text(p.read_text(encoding='utf-8').replace(note.as_posix(), altered.as_posix()), encoding='utf-8')
        changed = altered / 'PluginInstanceCLAP.Parameters.cpp'
        changed.write_text(changed.read_text(encoding='utf-8') + '\n// unexpected input\n', encoding='utf-8')
        try:
            patch.generate(source, altered, state, output)
        except RuntimeError as error:
            assert all(s in str(error) for s in ['parameter-automation', 'unexpected composed input', 'revision', 'README.md']), str(error)
        else:
            raise AssertionError('tampered composed source accepted')
        assert first == snapshot(output), 'failed validation changed generated output'
        altered_source = temp / 'source'
        shutil.copytree(source / 'vst3', altered_source / 'vst3')
        shutil.copytree(source / 'clap', altered_source / 'clap')
        changed = altered_source / 'vst3' / 'PluginInstanceVST3.Parameters.cpp'
        changed.write_text(changed.read_text(encoding='utf-8') + '\n// changed upstream\n', encoding='utf-8')
        try:
            patch.generate(altered_source, note, state, output)
        except RuntimeError as error:
            assert 'baseline SHA256 mismatch' in str(error)
        else:
            raise AssertionError('unknown upstream accepted')
        assert first == snapshot(output)
    assert initial == {str(root): snapshot(root) for root in (source, note, state)}, 'input modified'
    print('parameter-automation composition/idempotence/fail-closed/input preservation PASS')


if __name__ == '__main__':
    test(*sys.argv[1:])
