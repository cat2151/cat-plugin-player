"""Read-only native shim path for one fresh plugin instance (no audio device/UI)."""
import argparse
import ctypes as c
import hashlib
import math
from pathlib import Path
import time

parser = argparse.ArgumentParser()
parser.add_argument("--root", type=Path, required=True)
parser.add_argument("--plugin", required=True)
parser.add_argument("--id", required=True)
parser.add_argument("--name", required=True)
parser.add_argument("--state", type=Path)
parser.add_argument("--held", action="store_true")
parser.add_argument("--shutdown", action="store_true", help="use app shutdown's 128 channel-0 offs plus 16 CC120 messages")
args = parser.parse_args()
assert not (args.held and args.shutdown)
dll = c.CDLL(str(args.root / "target/shim/out/uapmd_shim.dll"))
WAKE = c.CFUNCTYPE(None, c.c_void_p)
DONE = c.CFUNCTYPE(None, c.c_void_p, c.c_int32, c.c_char_p)
dll.uh_create.argtypes = [WAKE, c.c_void_p]
dll.uh_create.restype = c.c_void_p
for name in ("uh_destroy", "uh_pump_startup"):
    getattr(dll, name).argtypes = [c.c_void_p]
dll.uh_restore_plugin.argtypes = [c.c_void_p] + [c.c_char_p] * 5
dll.uh_restore_plugin.restype = c.c_int32
dll.uh_instance_create.argtypes = [c.c_void_p, c.c_int32, c.c_uint32, c.c_uint32, DONE, c.c_void_p]
dll.uh_state_load.argtypes = [c.c_void_p, c.c_int32, c.c_void_p, c.c_uint64, c.c_char_p, c.c_int32]
dll.uh_state_load.restype = c.c_int32
dll.uh_processor_create.argtypes = [c.c_void_p, c.c_int32, c.c_uint32, c.c_uint32]
dll.uh_processor_create.restype = c.c_void_p
dll.uh_processor_destroy.argtypes = [c.c_void_p]
dll.uh_processor_process.argtypes = [c.c_void_p, c.POINTER(c.c_uint32), c.c_int32, c.POINTER(c.c_float), c.c_int32, c.c_int32]
dll.uh_processor_process.restype = c.c_int32
result = []
@WAKE
def wake(_):
    pass
@DONE
def done(_, instance, error):
    result.append((instance, error))
host = dll.uh_create(wake, None)
assert host, "create failed"
processor = None
try:
    index = dll.uh_restore_plugin(host, b"CLAP", args.id.encode(), args.name.encode(), b"", args.plugin.encode())
    assert index >= 0
    dll.uh_instance_create(host, index, 48000, 256, done, None)
    deadline = time.monotonic() + 30
    while not result and time.monotonic() < deadline:
        dll.uh_pump_startup(host)
        time.sleep(0.005)
    assert result and result[0][0] >= 0, result
    instance = result[0][0]
    if args.state:
        state = args.state.read_bytes()
        error = c.create_string_buffer(2048)
        data = c.create_string_buffer(state)
        assert dll.uh_state_load(host, instance, data, len(state), error, len(error)) == 0, error.value
        print(f"state_bytes={len(state)} sha256={hashlib.sha256(state).hexdigest()}")
    processor = dll.uh_processor_create(host, instance, 48000, 256)
    assert processor
    bins = {name: [] for name in ("sounding", "early_tail", "late_tail")}
    statuses = {}
    for block in range(1500):
        words = ([0x20B00100] if block == 0 else [0x20903C64] if block == 1 else
                 [0x20803C64] if block == 188 and not args.held else [])
        if block == 188 and args.shutdown:
            words = [0x20800000 | (key << 8) for key in range(128)] + [0x20B07800 | (channel << 16) for channel in range(16)]
        events = (c.c_uint32 * len(words))(*words)
        output = (c.c_float * 512)()
        status = dll.uh_processor_process(processor, events, len(words), output, 2, 256)
        statuses[status] = statuses.get(status, 0) + 1
        assert status == 0, status
        assert all(math.isfinite(value) for value in output)
        key = "sounding" if 1 <= block < 188 else "early_tail" if 188 <= block < 376 else "late_tail" if block >= 1125 else None
        if key:
            bins[key].extend(output)
    print(f"shim plugin={args.name} rate=48000 block=256 key=60 velocity=100 channel=0 CC1=0 on_sample=256 off_sample=48128 held={args.held} shutdown={args.shutdown} statuses={statuses}")
    for name, values in bins.items():
        print(f"{name} rms={math.sqrt(sum(v*v for v in values)/len(values))} peak={max(abs(v) for v in values)}")
    assert max(abs(v) for v in bins["sounding"]) > 1e-6, "silent host-path note"
finally:
    if processor:
        dll.uh_processor_destroy(processor)
    dll.uh_destroy(host)
