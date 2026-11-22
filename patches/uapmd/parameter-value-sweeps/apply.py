"""Compose the VALUES-only repair after parameter-automation, without editing inputs."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ANCHOR = '''        if ((flags & CLAP_PARAM_RESCAN_VALUES) && !definitionsRebuilt) {
            // Parameter values changed - refresh all parameter values
            Logger::global()->logDiagnostic("CLAP: Parameter values changed, refreshing");
            auto& paramDefs = params->parameters();
            for (size_t i = 0; i < paramDefs.size(); ++i) {
                double value = 0.0;
                if (params->getParameter(static_cast<uint32_t>(i), &value) == StatusCode::OK) {
                    auto paramId = params->getParameterId(static_cast<uint32_t>(i));
                    params->notifyParameterValue(paramId, value);
                }
            }
        }
'''
REPLACEMENT = '''        // parameter-value-sweeps: consumers read current values on demand.
        // Keep metadata rescans above, callback scheduling and parameter flushes.
        (void) definitionsRebuilt;
'''


def rewrite(text):
    if text.count(ANCHOR) != 1:
        raise ValueError(f'expected exactly one VALUES sweep, found {text.count(ANCHOR)}')
    return text.replace(ANCHOR, REPLACEMENT)


def generate(source, note, state, composed, output):
    source, note, state, composed, output = map(
        lambda p: Path(p).resolve(), (source, note, state, composed, output))
    target = composed / 'clap/HostClasses.cpp'
    try:
        if any(output == p or p in output.parents for p in (source, note, state, composed)):
            raise ValueError('output must be outside input directories')
        spec = importlib.util.spec_from_file_location('parameter_automation', HERE.parent/'parameter-automation/apply.py')
        automation = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(automation)
        # Validate the entire composed input against the existing repair, including
        # its upstream checks. Absolute includes must continue to use its headers.
        with tempfile.TemporaryDirectory(prefix='parameter-value-validation-') as temp:
            expected = Path(temp).resolve()
            automation.generate(source, note, state, expected)
            text = target.read_text(encoding='utf-8')
            wanted = (expected/'clap/HostClasses.cpp').read_text(encoding='utf-8')
            wanted = wanted.replace(expected.as_posix(), composed.as_posix())
            if text != wanted:
                raise ValueError('composed HostClasses.cpp differs from parameter-automation output')
        text = rewrite(text)
        output.mkdir(parents=True, exist_ok=True)
        result = output / 'HostClasses.cpp'
        if not result.exists() or result.read_text(encoding='utf-8') != text:
            result.write_text(text, encoding='utf-8', newline='\n')
        return result
    except (OSError, ValueError, RuntimeError) as error:
        revision = subprocess.run(['git', '-C', str(source), 'rev-parse', 'HEAD'],
                                  capture_output=True, text=True).stdout.strip() or 'unknown'
        raise RuntimeError(f'[parameter-value-sweeps] NOT applied; target: {target}; '
                           f'condition: {error}; UAPMD revision: {revision}; instructions: {HERE/"README.md"}') from error


if __name__ == '__main__':
    try:
        print(generate(*sys.argv[1:]))
    except RuntimeError as error:
        sys.exit(str(error))
