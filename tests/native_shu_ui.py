"""Exercise Shu's editor lifecycle against the built shim (Windows)."""
import ctypes as c
from pathlib import Path
import os
import time

ROOT = Path(__file__).resolve().parents[1]
SHU = Path("C:/Program Files/Common Files/CLAP/shimmer-reverb/Shu.clap")
WAKE = c.CFUNCTYPE(None, c.c_void_p)
DONE = c.CFUNCTYPE(None, c.c_void_p, c.c_int32, c.c_char_p)
dll = c.CDLL(str(ROOT / "target/shim/out/uapmd_shim.dll"))
user32 = c.WinDLL("user32", use_last_error=True)
user32.FindWindowExW.argtypes = [c.c_void_p, c.c_void_p, c.c_wchar_p, c.c_wchar_p]
user32.FindWindowExW.restype = c.c_void_p
user32.GetWindowThreadProcessId.argtypes = [c.c_void_p, c.POINTER(c.c_ulong)]
user32.GetWindow.argtypes = [c.c_void_p, c.c_uint]
user32.GetWindow.restype = c.c_void_p
user32.IsWindowVisible.argtypes = [c.c_void_p]

user32.SetThreadDpiAwarenessContext.argtypes = [c.c_void_p]
user32.SetThreadDpiAwarenessContext.restype = c.c_void_p
# Match native placement's physical coordinate system before creating windows.
previous_dpi = user32.SetThreadDpiAwarenessContext(c.c_void_p(-4))
user32.CreateWindowExW.argtypes = [c.c_ulong, c.c_wchar_p, c.c_wchar_p, c.c_ulong,
                                  c.c_int, c.c_int, c.c_int, c.c_int] + [c.c_void_p] * 4
user32.CreateWindowExW.restype = c.c_void_p
user32.DestroyWindow.argtypes = [c.c_void_p]
user32.SetWindowPos.argtypes = [c.c_void_p, c.c_void_p, c.c_int, c.c_int, c.c_int, c.c_int, c.c_uint]
user32.MonitorFromWindow.argtypes = [c.c_void_p, c.c_ulong]
user32.MonitorFromWindow.restype = c.c_void_p

class Rect(c.Structure):
    _fields_ = [(field, c.c_long) for field in ("left", "top", "right", "bottom")]

user32.GetClientRect.argtypes = [c.c_void_p, c.POINTER(Rect)]
user32.GetWindowRect.argtypes = [c.c_void_p, c.POINTER(Rect)]
user32.MapWindowPoints.argtypes = [c.c_void_p, c.c_void_p, c.c_void_p, c.c_uint]
user32.SendMessageW.argtypes = [c.c_void_p, c.c_uint, c.c_size_t, c.c_ssize_t]
user32.SendMessageW.restype = c.c_ssize_t

class Monitor(c.Structure):
    _fields_ = [("size", c.c_ulong), ("monitor", Rect), ("work", Rect), ("flags", c.c_ulong)]

user32.GetMonitorInfoW.argtypes = [c.c_void_p, c.POINTER(Monitor)]
user32.AdjustWindowRectEx.argtypes = [c.POINTER(Rect), c.c_ulong, c.c_int, c.c_ulong]

def outer(hwnd):
    rect = Rect()
    assert user32.GetWindowRect(hwnd, c.byref(rect))
    return rect

def check_placement(main, editor, work, right_fit):
    source, target = outer(main), outer(editor)
    width, height = target.right - target.left, target.bottom - target.top
    expected = ((source.right, source.top) if right_fit else
                (max(work.left, work.right - width), max(work.top, work.bottom - height)))
    assert (target.left, target.top) == expected, (tuple(expected), target.left, target.top)
    assert work.left <= target.left < target.right <= work.right
    assert work.top <= target.top < target.bottom <= work.bottom
    print(f"placement: {expected}, right_fit={right_fit}", flush=True)

def check_editor_bounds():
    parent = None
    while True:
        parent = user32.FindWindowExW(None, parent, "RemidyContainerWindow", "Shu (CLAP)")
        assert parent, "no editor window owned by the test process"
        owner = c.c_ulong()
        user32.GetWindowThreadProcessId(parent, c.byref(owner))
        if owner.value == os.getpid():
            break
    child = user32.GetWindow(parent, 5)  # GW_CHILD
    assert child and user32.IsWindowVisible(child), "no visible embedded editor"
    client, editor = Rect(), Rect()
    assert user32.GetClientRect(parent, c.byref(client))
    assert user32.GetWindowRect(child, c.byref(editor))
    user32.MapWindowPoints(None, parent, c.byref(editor), 2)
    assert 0 <= editor.left < editor.right <= client.right, (client.right, editor.right)
    assert 0 <= editor.top < editor.bottom <= client.bottom, (client.bottom, editor.bottom)
    return parent
dll.uh_create.argtypes = [WAKE, c.c_void_p]
dll.uh_create.restype = c.c_void_p
for name in ("uh_destroy", "uh_pump_startup"):
    getattr(dll, name).argtypes = [c.c_void_p]
dll.uh_restore_plugin.argtypes = [c.c_void_p] + [c.c_char_p] * 5
dll.uh_restore_plugin.restype = c.c_int32
dll.uh_instance_create.argtypes = [c.c_void_p, c.c_int32, c.c_uint32, c.c_uint32, DONE, c.c_void_p]
for name in ("uh_ui_show", "uh_ui_hide", "uh_ui_is_visible"):
    getattr(dll, name).argtypes = [c.c_void_p, c.c_int32]
    getattr(dll, name).restype = c.c_int32 if name != "uh_ui_hide" else None

dll.uh_ui_set_main_window.argtypes = [c.c_void_p, c.c_void_p]
dll.uh_ui_place_window.argtypes = [c.c_void_p, c.c_void_p]
dll.uh_ui_place_window.restype = c.c_int32

dll.uh_ui_thumbnail.argtypes = [c.c_void_p, c.c_int32, c.POINTER(c.c_uint8), c.c_int32,
                               c.POINTER(c.c_int32), c.POINTER(c.c_int32)]
dll.uh_ui_thumbnail.restype = c.c_int32

def check_thumbnail(host, instance, visible):
    pixels = (c.c_uint8 * (192 * 128 * 4))()
    width, height = c.c_int32(), c.c_int32()
    code = dll.uh_ui_thumbnail(host, instance, pixels, len(pixels), c.byref(width), c.byref(height))
    if not visible:
        assert code != 0, "hidden UI must not be captured"
        return
    assert code == 0, f"capture failed: {code}"
    assert 1 <= width.value <= 192 and 1 <= height.value <= 128
    client = Rect()
    assert user32.GetClientRect(check_editor_bounds(), c.byref(client))
    scale = min(1, 192 / client.right, 128 / client.bottom)
    assert (width.value, height.value) == (max(1, int(client.right * scale)),
                                           max(1, int(client.bottom * scale)))
    data = bytes(pixels[:width.value * height.value * 4])
    assert all(alpha == 255 for alpha in data[3::4]), "non-opaque RGBA"
    assert len(set(zip(data[0::4], data[1::4], data[2::4]))) > 16, "blank capture"
    print(f"thumbnail: {width.value}x{height.value}, varied opaque RGBA", flush=True)

@WAKE
def wake(_):
    pass

result = []
capture_failures = []

@DONE
def done(_, instance, error):
    result.append((instance, error))

host = dll.uh_create(wake, None)
assert host, "host creation failed"
main = None
try:
    main_frame = Rect(0, 0, 900, 600)
    assert user32.AdjustWindowRectEx(c.byref(main_frame), 0x00CF0000, False, 0)
    main_width, main_height = main_frame.right - main_frame.left, main_frame.bottom - main_frame.top
    main = user32.CreateWindowExW(0, "STATIC", "placement test main", 0x00CF0000,
                                  0, 0, main_width, main_height, None, None, None, None)
    assert main
    monitor = Monitor()
    monitor.size = c.sizeof(Monitor)
    assert user32.GetMonitorInfoW(user32.MonitorFromWindow(main, 2), c.byref(monitor))
    work = monitor.work
    # Outer-frame equivalent of the actual app: framed window, measured via HWND.
    assert user32.SetWindowPos(main, None, work.left, work.top, main_width, main_height, 0x14)
    dll.uh_ui_set_main_window(host, main)
    index = dll.uh_restore_plugin(host, b"CLAP", b"audio.mikey.Shu", b"Shu", b"Mikey Audio", str(SHU).encode())
    assert index >= 0, "Shu not installed"
    dll.uh_instance_create(host, index, 48000, 512, done, None)
    deadline = time.monotonic() + 30
    while not result and time.monotonic() < deadline:
        dll.uh_pump_startup(host)
        time.sleep(0.01)
    assert result and result[0][0] >= 0, result
    instance = result[0][0]
    for attempt in range(3):
        # Show again after moving the main window: only Show should reposition.
        x = work.right - main_width if attempt == 1 else work.left
        assert user32.SetWindowPos(main, None, x, work.top, 0, 0, 0x15)
        code = dll.uh_ui_show(host, instance)
        print(f"show #{attempt + 1}: {code}", flush=True)
        assert code == 0, f"uh_ui_show failed: {code}"
        parent = check_editor_bounds()
        check_placement(main, parent, work, attempt != 1)
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            dll.uh_pump_startup(host)
            assert dll.uh_ui_is_visible(host, instance), "editor disappeared"
            time.sleep(0.01)
        try:
            check_thumbnail(host, instance, True)
        except AssertionError as error:
            # Keep independent placement/lifecycle evidence when capture fails.
            capture_failures.append(str(error))
        if attempt == 1:
            user32.SendMessageW(parent, 0x10, 0, 0)  # WM_CLOSE
        else:
            dll.uh_ui_hide(host, instance)
        assert not dll.uh_ui_is_visible(host, instance), "editor did not hide"
        check_thumbnail(host, instance, False)
    # Exercise the same native positioning entry used by a plugin resize,
    # without requesting unsupported resizing from Shu or altering its state.
    resized = user32.CreateWindowExW(0, "STATIC", "placement resize test", 0x00CF0000,
                                     0, 0, 100, 100, None, None, None, None)
    assert resized
    try:
        assert user32.SetWindowPos(main, None, work.left, work.top, 0, 0, 0x15)
        assert dll.uh_ui_place_window(host, resized) == 0
        check_placement(main, resized, work, True)
        width, height = work.right - work.left + 100, work.bottom - work.top + 100
        assert user32.SetWindowPos(resized, None, 0, 0, width, height, 0x14)
        assert dll.uh_ui_place_window(host, resized) == 0
        large = outer(resized)
        assert (large.left, large.top) == (work.left, work.top)
        assert (large.right - large.left, large.bottom - large.top) == (width, height)
    finally:
        user32.DestroyWindow(resized)
    print("Shu editor placement / visibility / bounds / hide / close / reopen: passed", flush=True)
    assert not capture_failures, f"thumbnail failures: {capture_failures}"
finally:
    dll.uh_destroy(host)
    if main:
        user32.DestroyWindow(main)
    if previous_dpi:
        user32.SetThreadDpiAwarenessContext(previous_dpi)
