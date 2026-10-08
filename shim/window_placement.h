#pragma once
#if _WIN32
#include <Windows.h>
#include <algorithm>

namespace uh {
// All rect queries and moves use physical pixels, independent of caller DPI.
class PhysicalCoordinates {
    DPI_AWARENESS_CONTEXT previous_;
public:
    PhysicalCoordinates() : previous_(SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)) {}
    ~PhysicalCoordinates() { if (previous_) SetThreadDpiAwarenessContext(previous_); }
};

inline POINT editorPosition(const RECT& main, const RECT& work, LONG width, LONG height) {
    if (main.right >= work.left && main.top >= work.top &&
        main.right + width <= work.right && main.top + height <= work.bottom)
        return {main.right, main.top};
    // Oversized editors retain their dimensions and their accessible top/left.
    return {std::max(work.left, work.right - width),
            std::max(work.top, work.bottom - height)};
}

inline bool placeEditor(HWND main, HWND editor) {
    if (!IsWindow(main) || !IsWindow(editor)) return false;
    DWORD process = 0;
    GetWindowThreadProcessId(main, &process);
    if (process != GetCurrentProcessId()) return false;
    PhysicalCoordinates coordinates;
    MONITORINFO monitor{sizeof(MONITORINFO)};
    if (!GetMonitorInfoW(MonitorFromWindow(main, MONITOR_DEFAULTTONEAREST), &monitor)) return false;
    // A cross-monitor move can synchronously change editor DPI and outer size.
    // Re-read its real frame after the move before choosing the final position.
    for (int pass = 0; pass < 2; ++pass) {
        RECT mainRect{}, editorRect{};
        if (!GetWindowRect(main, &mainRect) || !GetWindowRect(editor, &editorRect)) return false;
        const auto position = editorPosition(mainRect, monitor.rcWork,
            editorRect.right - editorRect.left, editorRect.bottom - editorRect.top);
        if (!SetWindowPos(editor, nullptr, position.x, position.y, 0, 0,
                          SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE)) return false;
    }
    return true;
}
} // namespace uh
#endif
