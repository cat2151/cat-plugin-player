"""Generate the state-load repair after clap-note-dialect, without editing inputs."""
import hashlib
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
NAME = "PluginInstanceCLAP.States.cpp"
BASELINE = "4addbb9213ae857edb7019fff4e365e9d48e11c62c543b1f75376c5d9d522c43"
HELPER = '''    // Private implementation only: caller is already on the main thread.
    static std::string loadClapState(CLAPPluginProxy* plugin,
                                     clap_plugin_state_context_t* extension,
                                     std::vector<uint8_t>& state,
                                     PluginStateSupport::StateContextType type) {
        if (!plugin) return "CLAP state load: owner or plugin missing";
        ClapStreamReadContext context{&state, 0};
        clap_istream_t stream{&context, remidy_clap_stream_read};
        if (extension) {
            if (!extension->load) return "CLAP state load: context load missing";
            if (!extension->load(plugin->clapPlugin(), &stream, remidyContextTypeToCLAPContextType(type)))
                return "CLAP state load rejected (with context)";
        } else {
            if (!plugin->canUseState()) return "CLAP state load: state extension missing";
            // Proxy::stateLoad is noexcept: call the extension on this main
            // thread so a plugin exception reaches the queued error boundary.
            const auto raw = plugin->clapPlugin();
            const auto normal = static_cast<const clap_plugin_state_t*>(raw->get_extension(raw, CLAP_EXT_STATE));
            if (!normal || !normal->load) return "CLAP state load: state extension missing";
            if (!normal->load(raw, &stream)) return "CLAP state load rejected";
        }
        return "";
    }

'''


def generate(upstream, note_input, output):
    upstream, note_input, output = map(lambda p: Path(p).resolve(), (upstream, note_input, output))
    current = upstream / NAME
    try:
        if output == upstream or upstream in output.parents or output == note_input or note_input in output.parents:
            raise ValueError("output must be outside dependency checkout and note input")
        raw = current.read_bytes().replace(b"\r\n", b"\n")
        if hashlib.sha256(raw).hexdigest() != BASELINE:
            raise ValueError("upstream baseline SHA256 mismatch; inspect upstream fix/change")
        original = raw.decode("utf-8")
        current = note_input / NAME
        text = current.read_text(encoding="utf-8")
        expected = original.replace('#include "PluginFormatCLAP.hpp"',
                                    f'#include "{(note_input / "PluginFormatCLAP.hpp").as_posix()}"')
        if text != expected or not (note_input / "PluginFormatCLAP.hpp").is_file():
            raise ValueError("expected unchanged note-generated state source and generated header")
        text = text.replace('#include <vector>', '#include <vector>\n#include <exception>')
        text = text.replace(': owner(owner) {', ': owner(owner), state_context_ext(nullptr) {', 1)
        text = text.replace('if (!owner->plugin)', 'if (!owner || !owner->plugin)', 1)
        start = text.index('    void PluginInstanceCLAP::PluginStatesCLAP::setState(')
        end = text.index('    void PluginInstanceCLAP::PluginStatesCLAP::requestState(', start)
        text = text[:start] + HELPER + '''    void PluginInstanceCLAP::PluginStatesCLAP::setState(std::vector<uint8_t>& state,
            PluginStateSupport::StateContextType stateContextType, bool includeUiState) {
        EventLoop::runTaskOnMainThread([&] {
            try {
                const auto error = loadClapState(owner ? owner->plugin.get() : nullptr,
                                                state_context_ext, state, stateContextType);
                if (!error.empty()) std::cerr << error << std::endl;
            } catch (const std::exception& error) {
                std::cerr << "CLAP state load: " << error.what() << std::endl;
            } catch (...) {
                std::cerr << "CLAP state load: unknown exception" << std::endl;
            }
        });
    }

''' + text[end:]
        old = '                                   setState(state, stateContextType, includeUiState);\n                                   finish("");'
        if text.count(old) != 1:
            raise ValueError("expected exactly one queued load completion")
        text = text.replace(old, '''                                   std::string error;
                                   try {
                                       error = loadClapState(owner ? owner->plugin.get() : nullptr,
                                                             state_context_ext, state, stateContextType);
                                   } catch (const std::exception& exception) {
                                       error = std::string("CLAP state load: ") + exception.what();
                                   } catch (...) {
                                       error = "CLAP state load: unknown exception";
                                   }
                                   finish(std::move(error));''')
        output.mkdir(parents=True, exist_ok=True)
        path = output / NAME
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            path.write_text(text, encoding="utf-8", newline="\n")
        return path
    except (OSError, ValueError) as error:
        revision = subprocess.run(["git", "-C", str(upstream), "rev-parse", "HEAD"],
                                  capture_output=True, text=True).stdout.strip() or "unknown"
        raise RuntimeError(f"[clap-state-load-error] NOT applied; target: {current}; condition: {error}; "
                           f"UAPMD revision: {revision}; instructions: {HERE / 'README.md'}") from error


if __name__ == "__main__":
    try:
        print(generate(*sys.argv[1:]))
    except (RuntimeError, ValueError) as error:
        sys.exit(str(error))
