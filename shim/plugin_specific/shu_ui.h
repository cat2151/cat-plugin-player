#pragma once

#if defined(_WIN32)
#include <Windows.h>
#include <string_view>

namespace uh {

// Shu's Windows CLAP build creates and embeds its editor in set_parent(),
// but returns false from gui.show(). Accept that failure only when its actual
// editor is already visible. Do not mask failed creation or other plugins.
inline bool shuEmbeddedEditorVisible(std::string_view format, std::string_view id,
                                     HWND parent) {
    if (format != "CLAP" || id != "audio.mikey.Shu" || !IsWindowVisible(parent))
        return false;
    for (HWND child = GetWindow(parent, GW_CHILD); child;
         child = GetWindow(child, GW_HWNDNEXT)) {
        RECT bounds{};
        if (IsWindowVisible(child) && GetClientRect(child, &bounds) &&
            bounds.right > bounds.left && bounds.bottom > bounds.top)
            return true;
    }
    return false;
}

} // namespace uh
#endif
