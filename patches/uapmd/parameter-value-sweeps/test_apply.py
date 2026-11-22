"""Composition, drift rejection, and idempotence; all scratch files outside the repo."""
import importlib.util
from pathlib import Path
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
spec = importlib.util.spec_from_file_location('values_repair', HERE/'apply.py')
repair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repair)
source = ROOT.parent/'uapmd/source/remidy/src'
note = ROOT/'target/shim/generated/clap-note-dialect'
state = ROOT/'target/shim/generated/clap-state-load-error'
composed = ROOT/'target/shim/generated/parameter-automation'
original = (composed/'clap/HostClasses.cpp').read_bytes()
with tempfile.TemporaryDirectory() as temp:
    work = Path(temp)
    result = repair.generate(source, note, state, composed, work/'out')
    text = result.read_text(encoding='utf-8')
    rescan = text.split('void RemidyCLAPHost::paramsRescan', 1)[1].split('void RemidyCLAPHost::paramsClear', 1)[0]
    assert 'getParameter(' not in rescan and 'notifyParameterValue' not in rescan
    assert rescan.count('refreshAllParameterMetadata()') == 3
    assert 'pluginPtr->onMainThread()' in text and 'processParamsFlush()' in text
    stamp = result.stat().st_mtime_ns
    assert repair.generate(source, note, state, composed, work/'out') == result
    assert result.stat().st_mtime_ns == stamp
    assert (composed/'clap/HostClasses.cpp').read_bytes() == original
    for anchor in ['', repair.ANCHOR * 2]:
        try:
            repair.rewrite(original.decode().replace(repair.ANCHOR, anchor))
        except ValueError:
            pass
        else:
            raise AssertionError('missing/duplicate anchor accepted')
    bad = work/'bad/clap/HostClasses.cpp'
    bad.parent.mkdir(parents=True)
    bad.write_text(original.decode() + '// unexpected drift\n', encoding='utf-8')
    try:
        repair.generate(source, note, state, bad.parent.parent, work/'bad-out')
    except RuntimeError as error:
        assert all(word in str(error) for word in ['parameter-value-sweeps', 'HostClasses.cpp', 'revision:', 'README.md'])
        assert not (work/'bad-out').exists()
    else:
        raise AssertionError('composed drift accepted')
    # read_text normalizes CRLF; an unchanged Windows checkout is valid.
    windows = original.decode().replace(composed.as_posix(), bad.parent.parent.as_posix())
    bad.write_bytes(windows.replace('\r\n', '\n').replace('\n', '\r\n').encode())
    repair.generate(source, note, state, bad.parent.parent, work/'crlf-out')
print('parameter-value-sweeps composition/metadata/callback/flush/drift/idempotence PASS')
