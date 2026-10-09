"""Windows integration smoke test; briefly shows an exit overlay, without plugins/audio.
Run after cargo build: python scripts/test_shutdown_overlay.py
"""
import ctypes as c
from pathlib import Path
import time

u = c.WinDLL("user32", use_last_error=True)
g = c.WinDLL("gdi32", use_last_error=True)
u.CreateWindowExW.argtypes = [c.c_uint, c.c_wchar_p, c.c_wchar_p, c.c_uint,
                              c.c_int, c.c_int, c.c_int, c.c_int,
                              c.c_void_p, c.c_void_p, c.c_void_p, c.c_void_p]
u.CreateWindowExW.restype = c.c_void_p
u.FindWindowW.argtypes = [c.c_wchar_p, c.c_wchar_p]
u.FindWindowW.restype = c.c_void_p
u.GetDC.argtypes = [c.c_void_p]
u.GetDC.restype = c.c_void_p
u.ReleaseDC.argtypes = [c.c_void_p, c.c_void_p]
u.DestroyWindow.argtypes = [c.c_void_p]
g.GetPixel.argtypes = [c.c_void_p, c.c_int, c.c_int]
g.GetPixel.restype = c.c_uint

# Only application-owned test windows are accessed; no user windows/data.
dll = c.CDLL(str(Path(__file__).resolve().parents[1] / "target/shim/out/uapmd_shim.dll"))
dll.uh_shutdown_overlay_start.argtypes = [c.c_ssize_t]
dll.uh_shutdown_overlay_start.restype = c.c_void_p
dll.uh_shutdown_overlay_update.argtypes = [c.c_void_p, c.c_char_p, c.c_size_t, c.c_size_t]
dll.uh_shutdown_overlay_stop.argtypes = [c.c_void_p]
assert not dll.uh_shutdown_overlay_start(0)
parent = u.CreateWindowExW(0, "STATIC", "Overlay test host", 0,
                            100, 100, 1000, 600, None, None, None, None)
assert parent, c.get_last_error()
overlay = None
try:
    started = time.monotonic()
    overlay = dll.uh_shutdown_overlay_start(parent)
    assert overlay
    print(f"First paint ready in {time.monotonic() - started:.3f}s")
    window = u.FindWindowW("CatPlayerShutdownOverlay", "Saving")
    assert window

    def pixels(y):
        dc = u.GetDC(window)
        assert dc
        try:
            return tuple(g.GetPixel(dc, x, y) for x in range(24, 476))
        finally:
            u.ReleaseDC(window, dc)

    dll.uh_shutdown_overlay_update(overlay, "Saving: テスト instrument".encode(), 1, 3)
    # Deliberately block this main thread; do not pump its message queue.
    time.sleep(0.2)
    first = pixels(174)
    time.sleep(0.3)
    second = pixels(174)
    assert first != second, "animation stopped during a blocking state call"
    bright = 0x00F5B96E
    assert bright in first and bright in second, "animated dot was not painted"
    assert bright in pixels(149), "completed-step bar was not painted"
    assert u.DestroyWindow(parent)
    parent = None
    time.sleep(0.3)
    assert pixels(174) != second, "overlay stopped during parent/plugin teardown"
    dll.uh_shutdown_overlay_update(overlay, b"Closing plugins...", 3, 3)
finally:
    if overlay:
        started = time.monotonic()
        dll.uh_shutdown_overlay_stop(overlay)
        assert time.monotonic() - started < 2, "overlay teardown blocked"
    if parent:
        u.DestroyWindow(parent)
assert not u.FindWindowW("CatPlayerShutdownOverlay", "Saving")
print("PASS: animation during blocking save and parent teardown, progress, cleanup")
