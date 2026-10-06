"""Windows integration check with an installed instrument (Python 3.11+).

Run after cargo build --release. Uses the saved plugin without changing history.
Renders in memory, opens/closes its editor, and scans the existing plugin cache.
"""

import ctypes as c
import os
from pathlib import Path
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
WAKE = c.CFUNCTYPE(None, c.c_void_p)
DONE = c.CFUNCTYPE(None, c.c_void_p, c.c_char_p)
CREATED = c.CFUNCTYPE(None, c.c_void_p, c.c_int32, c.c_char_p)
dll = c.CDLL(str(ROOT / "target/release/uapmd_shim.dll"))


def bind(name, result, *args):
    fn = getattr(dll, name)
    fn.restype = result
    fn.argtypes = args
    return fn


ptr, i32, u32, string = c.c_void_p, c.c_int32, c.c_uint32, c.c_char_p
create = bind("uh_create", ptr, WAKE, ptr)
destroy = bind("uh_destroy", None, ptr)
pump = bind("uh_pump_startup", None, ptr)
restore = bind("uh_restore_plugin", i32, ptr, string, string, string, string, string)
instantiate = bind("uh_instance_create", None, ptr, i32, u32, u32, CREATED, ptr)
remove = bind("uh_instance_destroy", None, ptr, i32)
scan = bind("uh_scan_async", i32, ptr, i32, DONE, ptr)
count = bind("uh_plugin_count", i32, ptr)
info = bind("uh_plugin_info", i32, ptr, i32, i32, ptr, i32)
processor_create = bind("uh_processor_create", ptr, ptr, i32, u32, u32)
processor_destroy = bind("uh_processor_destroy", None, ptr)
process = bind("uh_processor_process", i32, ptr, c.POINTER(u32), i32,
               c.POINTER(c.c_float), i32, i32)
show = bind("uh_ui_show", i32, ptr, i32)
hide = bind("uh_ui_hide", None, ptr, i32)

config = Path(os.environ["LOCALAPPDATA"]) / "cat-plugin-player/config.toml"
key = tomllib.loads(config.read_text(encoding="utf-8"))["last_played"]
args = [key[field].encode("utf-8") for field in
        ("format", "id", "name", "vendor", "bundle_path")]
wake = WAKE(lambda _: None)
host = create(wake, None)
assert host
processors = []
instances = []


def wait_for(condition, tick=lambda: None):
    deadline = time.monotonic() + 20
    while not condition():
        assert time.monotonic() < deadline, "callback timed out"
        pump(host)
        tick()
        time.sleep(0.001)


def load(index, fail=False):
    result = []
    callback = CREATED(lambda _, instance, error: result.append((instance, error)))
    instantiate(host, index, 48000, 1024, callback, None)
    wait_for(lambda: bool(result))
    instance, error = result[0]
    if fail:
        assert instance < 0 and error, result
        return
    assert instance >= 0 and not error, result
    instances.append(instance)
    processor = processor_create(host, instance, 48000, 1024)
    assert processor
    processors.append(processor)
    return instance, processor


samples = (c.c_float * 2048)()
note = (u32 * 1)(0x20903064)


def render(processor, start=False):
    assert process(processor, note if start else None, int(start), samples, 2, 1024) == 0
    return any(abs(value) > 0.000001 for value in samples)


def field(index, number):
    length = info(host, index, number, None, 0)
    assert length >= 0
    buffer = c.create_string_buffer(length + 1)
    info(host, index, number, buffer, len(buffer))
    return buffer.value


try:
    assert restore(host, *args[:-1], b"X:/missing/plugin.clap") == -1
    bad = restore(host, b"unknown-format", *args[1:])
    assert bad >= 0
    load(bad, fail=True)
    index = restore(host, *args)
    instance, processor = load(index)
    signal = render(processor, True)
    signal = any([render(processor) for _ in range(20)]) or signal
    assert signal, "saved instrument did not generate a signal"

    # Replacing the discovery catalog twice must not invalidate the live instance.
    for _ in range(2):
        completed = []
        callback = DONE(lambda _, error: completed.append(error))
        assert scan(host, 0, callback, None) == 0
        wait_for(lambda: bool(completed), lambda: render(processor))
        assert completed == [None], completed
        assert render(processor, True)
    matching = [index for index in range(count(host))
                if field(index, 2) == args[0] and field(index, 3) == args[1]]
    assert matching, "saved plugin missing from scan"
    second, second_processor = load(matching[0])
    assert render(second_processor, True)
    assert show(host, instance) == 0, "editor could not be opened"
    pump(host)
    hide(host, instance)

    for processor in processors:
        processor_destroy(processor)
    processors.clear()
    for instance in instances:
        remove(host, instance)
    instances.clear()

    # Shutdown also has to drain a scan that has not delivered its callback yet.
    completed = []
    callback = DONE(lambda _, error: completed.append(error))
    assert scan(host, 0, callback, None) == 0
    destroy(host)
    host = None
    assert completed == [None], completed
    print("PASS: direct load, missing path, creation failure, scan while rendering, "
          "reload, editor, remove, shutdown during scan")
finally:
    if host:
        for processor in processors:
            processor_destroy(processor)
        destroy(host)
