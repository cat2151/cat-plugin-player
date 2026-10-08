"""Read-only user snapshots; measure CC1 changes in a fresh native host."""
import ctypes as c
import os
from pathlib import Path
import struct
import sys
import time
import tomllib
import xml.etree.ElementTree as ET

sys.stdout.reconfigure(encoding="utf-8")
ROOT = Path(__file__).resolve().parents[1]
DATA = Path(os.environ["LOCALAPPDATA"]) / "cat-plugin-player" / "favorites"
index = (DATA / "index.toml").read_bytes()
entries = tomllib.loads(index.decode())["entries"]
dll = c.CDLL(str(ROOT / "target/shim/out/uapmd_shim.dll"))
P, I, U, S = c.c_void_p, c.c_int32, c.c_uint32, c.c_char_p
WAKE = c.CFUNCTYPE(None, P)
DONE = c.CFUNCTYPE(None, P, I, S)
STATE = c.CFUNCTYPE(None, P, P, c.c_uint64)
for name, args, result in [
    ("uh_create", [WAKE, P], P),
    ("uh_restore_plugin", [P, S, S, S, S, S], I),
    ("uh_instance_create", [P, I, U, U, DONE, P], None),
    ("uh_state_load", [P, I, P, c.c_uint64, S, I], I),
    ("uh_state_save", [P, I, STATE, P, S, I], I),
    ("uh_chain_create", [P, I, P, I, I, U, U], P),
    ("uh_processor_process", [P, P, I, P, I, I], I),
    ("uh_processor_destroy", [P], None),
    ("uh_instance_destroy", [P, I], None),
    ("uh_destroy", [P], None),
    ("uh_pump_startup", [P], None),
    ("uh_ui_show", [P, I], I),
    ("uh_ui_hide", [P, I], None),
]:
    fn = getattr(dll, name)
    fn.argtypes, fn.restype = args, result
wake = WAKE(lambda _: None)
host = dll.uh_create(wake, None)
assert host
error = c.create_string_buffer(2048)


def save(instance):
    result = []
    callback = STATE(lambda _, data, size: result.append(c.string_at(data, size)))
    assert dll.uh_state_save(host, instance, callback, None, error, len(error)) == 0, error.value
    return result[0]


def load(instance, state):
    buffer = c.create_string_buffer(state)
    assert dll.uh_state_load(host, instance, buffer, len(state), error, len(error)) == 0, error.value


def settings(state):
    size = struct.unpack_from("<I", state, 4)[0]
    return ET.fromstring(state[8:8 + size])


try:
    for name in ["Floe", "Vaporizer2", "TyrellN6"]:
        entry = next(e for e in entries if e["plugin"]["name"] == name)
        key = entry["plugin"]
        plugin = dll.uh_restore_plugin(host, *[
            key[k].encode() for k in ["format", "id", "name", "vendor", "bundle_path"]
        ])
        assert plugin >= 0
        result = []
        callback = DONE(lambda _, instance, message: result.append((instance, message)))
        dll.uh_instance_create(host, plugin, 48000, 1024, callback, None)
        deadline = time.monotonic() + 30
        while not result:
            assert time.monotonic() < deadline
            dll.uh_pump_startup(host)
            time.sleep(.001)
        instance, message = result[0]
        assert instance >= 0 and not message, message
        try:
            original = (DATA / (entry["id"] + ".bin")).read_bytes()
            load(instance, original)
            # CLAP may apply restored state asynchronously after processing starts.
            warmup = dll.uh_chain_create(host, instance, None, 0, 0, 48000, 1024)
            assert warmup
            try:
                output = (c.c_float * 2048)()
                for _ in range(110):
                    dll.uh_pump_startup(host)
                    time.sleep(.005)
                    assert dll.uh_processor_process(warmup, None, 0, output, 2, 1024) == 0
            finally:
                dll.uh_processor_destroy(warmup)
            baseline = save(instance)
            print("PROBE", name, "baseline bytes", len(baseline), flush=True)
            for cc in [0, 32, 64, 127]:
                processor = dll.uh_chain_create(host, instance, None, 0, 0, 48000, 1024)
                assert processor
                try:
                    events = (U * 2)(0x20903C64, 0x20B00100 | cc)
                    output = (c.c_float * 2048)()
                    assert dll.uh_processor_process(processor, events, 2, output, 2, 1024) == 0
                    for _ in range(110):
                        dll.uh_pump_startup(host)
                        time.sleep(.005)
                        assert dll.uh_processor_process(processor, None, 0, output, 2, 1024) == 0
                    off = (U * 1)(0x20803C00)
                    assert dll.uh_processor_process(processor, off, 1, output, 2, 1024) == 0
                finally:
                    dll.uh_processor_destroy(processor)
                state = save(instance)
                if name == "Floe":
                    changes = [i for i, (a, b) in enumerate(zip(baseline, state)) if a != b]
                    print("CC", cc, "offsets", changes[:30], "macro", struct.unpack_from("<f", state, 4099)[0], "tail", state[-1])
                    assert abs(struct.unpack_from("<f", state, 4099)[0] - cc / 127) < 1e-6
                elif name == "Vaporizer2":
                    macro = next(e.attrib for e in settings(state) if e.attrib.get("id") == "m_fCustomModulator1")
                    print("CC", cc, "macro", macro)
                    assert abs(float(macro["text"]) - 100 * cc / 127) < .0001
                else:
                    print("CC", cc, "UI_op", next(l for l in state.decode().splitlines() if l.startswith("UI_op=")))
            if name == "TyrellN6":
                for value in [9, 10, 9]:
                    text = original.decode()
                    before = next(l for l in text.splitlines() if l.startswith("UI_op="))
                    load(instance, text.replace(before, f"UI_op={value}").encode())
                    state = save(instance)
                    print("LOAD UI_op", value, "SAVE", next(l for l in state.decode().splitlines() if l.startswith("UI_op=")))
                if "--ui" in sys.argv:
                    for _ in range(2):
                        print("SHOW", dll.uh_ui_show(host, instance))
                        for _ in range(100):
                            dll.uh_pump_startup(host)
                            time.sleep(.005)
                        state = save(instance)
                        print("UI OPEN", next(l for l in state.decode().splitlines() if l.startswith("UI_op=")))
                        dll.uh_ui_hide(host, instance)
                        state = save(instance)
                        print("UI CLOSED", next(l for l in state.decode().splitlines() if l.startswith("UI_op=")))
        finally:
            dll.uh_instance_destroy(host, instance)
finally:
    dll.uh_destroy(host)
assert (DATA / "index.toml").read_bytes() == index
print("User index unchanged")
