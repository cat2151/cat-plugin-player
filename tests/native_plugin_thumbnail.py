"""Check capture and bounded message pumping for an installed editor (Windows).

Usage: python tests/native_plugin_thumbnail.py --effect-index 0
Uses the selected effect's metadata from status.json, without changing its state.
"""
import argparse
import ctypes as c
import faulthandler
import json
import os
from pathlib import Path
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--effect-index", type=int, default=0)
args = parser.parse_args()
# A non-returning native pump otherwise prevents Python's deadline checks.
faulthandler.dump_traceback_later(40, exit=True)
root = Path(__file__).resolve().parents[1]
status = json.loads((Path(os.environ["LOCALAPPDATA"]) / "cat-plugin-player/status.json").read_text())
plugin = status["effects"][args.effect_index]
dll = c.CDLL(str(root / "target/shim/out/uapmd_shim.dll"))
WAKE = c.CFUNCTYPE(None, c.c_void_p)
DONE = c.CFUNCTYPE(None, c.c_void_p, c.c_int32, c.c_char_p)
dll.uh_create.argtypes = [WAKE, c.c_void_p]
dll.uh_create.restype = c.c_void_p
for name in ("uh_destroy", "uh_pump_startup"):
    getattr(dll, name).argtypes = [c.c_void_p]
dll.uh_restore_plugin.argtypes = [c.c_void_p] + [c.c_char_p] * 5
dll.uh_restore_plugin.restype = c.c_int32
dll.uh_instance_create.argtypes = [c.c_void_p, c.c_int32, c.c_uint32, c.c_uint32, DONE, c.c_void_p]
for name in ("uh_ui_show", "uh_ui_hide"):
    getattr(dll, name).argtypes = [c.c_void_p, c.c_int32]
dll.uh_ui_thumbnail.argtypes = [c.c_void_p, c.c_int32, c.POINTER(c.c_uint8), c.c_int32,
                               c.POINTER(c.c_int32), c.POINTER(c.c_int32)]


@WAKE
def wake(_):
    pass


results = []


@DONE
def done(_, instance, error):
    results.append((instance, error))


def pump_until(deadline):
    while time.monotonic() < deadline:
        start = time.monotonic()
        dll.uh_pump_startup(host)
        assert time.monotonic() - start < 2, "message pump did not yield promptly"
        time.sleep(0.01)


host = dll.uh_create(wake, None)
assert host
try:
    index = dll.uh_restore_plugin(host, *[plugin[k].encode() for k in
                                        ("format", "id", "name", "vendor", "bundle_path")])
    assert index >= 0, plugin
    dll.uh_instance_create(host, index, 48000, 512, done, None)
    deadline = time.monotonic() + 30
    while not results and time.monotonic() < deadline:
        pump_until(min(deadline, time.monotonic() + 0.05))
    assert results and results[0][0] >= 0, results
    instance = results[0][0]
    pixels = (c.c_uint8 * (192 * 128 * 4))()
    width, height = c.c_int32(), c.c_int32()
    for attempt in range(2):
        assert dll.uh_ui_show(host, instance) == 0
        pump_until(time.monotonic() + 2)
        assert dll.uh_ui_thumbnail(host, instance, pixels, len(pixels),
                                   c.byref(width), c.byref(height)) == 0
        assert 1 <= width.value <= 192 and 1 <= height.value <= 128
        data = bytes(pixels[:width.value * height.value * 4])
        assert all(alpha == 255 for alpha in data[3::4])
        assert len(set(zip(data[0::4], data[1::4], data[2::4]))) > 16, "blank capture"
        white = sum(all(v >= 245 for v in data[i:i+3]) for i in range(0, len(data), 4))
        assert white * 100 < width.value * height.value * 98, "white loading screen"
        print(f"{plugin['name']}: capture #{attempt + 1} {width.value}x{height.value}; pump yielded", flush=True)
        if attempt == 0:
            dll.uh_ui_hide(host, instance)
            assert dll.uh_ui_thumbnail(host, instance, pixels, len(pixels),
                                       c.byref(width), c.byref(height)) != 0
finally:
    # Destroy with the editor visible, exercising shutdown as well.
    dll.uh_destroy(host)
print("capture / hide / reopen / shutdown: passed", flush=True)
faulthandler.cancel_dump_traceback_later()
