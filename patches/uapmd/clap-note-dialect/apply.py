"""Generate build-local CLAP sources; validate every input before writing."""
import hashlib
from pathlib import Path
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
BASELINES = {
    "PluginFormatCLAP.hpp": "116cc11fe00ce2d13b9c7458f317bbb654e39164fc6ea5a098b2d0d5547da1b6",
    "PluginInstanceCLAP.cpp": "e85a6cf32bc59ab20566cce265939aa375d2a203640b876db30e2b66f8361386",
    "PluginInstanceCLAP.Events.cpp": "cb58643e1077d3f202c202ec46cb2741ef39f46037fd15cea62abf4a774feb26",
}

def generate(source, output):
    source, output = Path(source).resolve(), Path(output).resolve()
    if source == output or source in output.parents:
        raise ValueError("output must be outside the dependency checkout")
    current = source
    try:
        texts = {}
        for name, expected in BASELINES.items():
            current = source / name
            raw = current.read_bytes().replace(b"\r\n", b"\n")
            if hashlib.sha256(raw).hexdigest() != expected:
                raise ValueError("baseline SHA256 mismatch; inspect upstream change/fix")
            texts[name] = raw.decode("utf-8").replace("\r\n", "\n")
        for path in source.glob("*.cpp"):
            text = path.read_text(encoding="utf-8")
            if '#include "PluginFormatCLAP.hpp"' in text:
                texts.setdefault(path.name, text)

        def once(name, old, new):
            if texts[name].count(old) != 1:
                raise ValueError(f"{name}: expected exactly one {old!r}")
            texts[name] = texts[name].replace(old, new)

        once("PluginFormatCLAP.hpp", "        CLAPUmpInputDispatcher ump_input_dispatcher{this};",
             "        std::vector<uint32_t> input_note_dialects;\n"
             "        bool has_note_ports{false};\n"
             "        uint32_t inputNoteDialects(uint16_t port) const {\n"
             "            if (!has_note_ports) return CLAP_NOTE_DIALECT_CLAP;\n"
             "            return port < input_note_dialects.size() ? input_note_dialects[port] : 0;\n"
             "        }\n        CLAPUmpInputDispatcher ump_input_dispatcher{this};")
        once("PluginInstanceCLAP.cpp", "            activated_ = plugin->activate(configuration.sampleRate, 1, configuration.bufferSizeInSamples);",
             "            // Main thread, before activation: never query in the audio callback.\n"
             "            has_note_ports = plugin->canUseNotePorts();\n"
             "            input_note_dialects.clear();\n"
             "            if (has_note_ports) {\n"
             "                input_note_dialects.resize(plugin->notePortsCount(true), 0);\n"
             "                for (uint32_t i = 0; i < input_note_dialects.size(); ++i) {\n"
             "                    clap_note_port_info_t port{};\n"
             "                    if (plugin->notePortsGet(i, true, &port))\n"
             "                        input_note_dialects[i] = port.supported_dialects;\n"
             "                }\n            }\n"
             "            activated_ = plugin->activate(configuration.sampleRate, 1, configuration.bufferSizeInSamples);")
        name = "PluginInstanceCLAP.Events.cpp"
        for method, call in [("onNoteOn", "note"), ("onNoteOff", "note"), ("onCC", "cc")]:
            pattern = rf"(    void PluginInstanceCLAP::CLAPUmpInputDispatcher::{method}\([^\n]+\) \{{)\n.*?\n    \}}"
            if method == "onCC":
                args = "group, channel, index, data, timestamp()"
            else:
                args = f"{'true' if method == 'onNoteOn' else 'false'}, group, channel, note, velocity, timestamp()"
            replacement = (rf"\1\n        uh::clap_dialect::{call}([&](size_t size) -> void* {{\n"
                           "            return owner->events_in->tryAllocate(alignof(void*), size);\n"
                           "        }, owner->inputNoteDialects(group), " + args + ");\n    }")
            texts[name], count = re.subn(pattern, replacement, texts[name], flags=re.S)
            if count != 1:
                raise ValueError(f"{name}: {method}: expected one method, found {count}")
        texts["PluginFormatCLAP.hpp"] += f'\n#include "{(HERE / "note_dialect.h").as_posix()}"\n'
        # Policy constants must be visible in the class declaration too.
        texts["PluginFormatCLAP.hpp"] = '#include <clap/ext/note-ports.h>\n' + texts["PluginFormatCLAP.hpp"]
        for name, text in list(texts.items()):
            def include(match):
                path = match[1]
                if path == "PluginFormatCLAP.hpp":
                    return f'#include "{(output / path).as_posix()}"'
                original = source / path
                return f'#include "{original.resolve().as_posix()}"' if original.is_file() else match[0]
            texts[name] = re.sub(r'#include "([^"\n]+)"', include, text)
        output.mkdir(parents=True, exist_ok=True)
        for name, text in texts.items():
            path = output / name
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                path.write_text(text, encoding="utf-8", newline="\n")
        return sorted(n for n in texts if n.endswith(".cpp"))
    except (OSError, ValueError) as error:
        revision = subprocess.run(["git", "-C", str(source), "rev-parse", "HEAD"],
                                  capture_output=True, text=True).stdout.strip() or "unknown"
        raise RuntimeError(f"[clap-note-dialect] NOT applied; target: {current}; "
                           f"condition: {error}; UAPMD revision: {revision}; instructions: {HERE / 'README.md'}") from error

if __name__ == "__main__":
    try:
        print(";".join(generate(sys.argv[1], sys.argv[2])))
    except (RuntimeError, ValueError) as error:
        sys.exit(str(error))
