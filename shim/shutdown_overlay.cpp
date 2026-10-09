// Independent Windows UI thread; never calls a plugin or the host.
#include "uapmd_shim.h"
#include <cstddef>
#include <memory>
#ifdef _WIN32
#include <windows.h>
#include <algorithm>
#include <future>
#include <mutex>
#include <string>
#include <thread>

namespace {
struct Overlay {
    std::thread thread;
    HWND window = nullptr;
    std::mutex mutex;
    std::wstring text = L"Preparing to save...";
    size_t done = 0, total = 1;
    ULONGLONG started = GetTickCount64();
};

void paint(HWND window, Overlay& overlay) {
    PAINTSTRUCT ps;
    HDC dc = BeginPaint(window, &ps);
    RECT rect;
    GetClientRect(window, &rect);
    HDC buffer = CreateCompatibleDC(dc);
    HBITMAP bitmap = CreateCompatibleBitmap(dc, rect.right, rect.bottom);
    auto oldBitmap = SelectObject(buffer, bitmap);
    HBRUSH background = CreateSolidBrush(RGB(28, 30, 36));
    FillRect(buffer, &rect, background);
    DeleteObject(background);
    SetBkMode(buffer, TRANSPARENT);
    SetTextColor(buffer, RGB(240, 240, 245));
    auto font = CreateFontW(-18, 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY, DEFAULT_PITCH, L"Segoe UI");
    auto oldFont = SelectObject(buffer, font);
    RECT title{24, 18, rect.right - 24, 48};
    DrawTextW(buffer, L"Saving before exit", -1, &title, DT_CENTER | DT_SINGLELINE);
    std::wstring text;
    size_t done, total;
    {
        std::lock_guard lock(overlay.mutex);
        text = overlay.text;
        done = overlay.done;
        total = overlay.total;
    }
    RECT detail{24, 58, rect.right - 24, 90};
    DrawTextW(buffer, text.c_str(), -1, &detail, DT_CENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
    auto elapsed = (GetTickCount64() - overlay.started) / 1000;
    auto count = std::to_wstring(done) + L" / " + std::to_wstring(total)
        + L" save steps complete   |   " + std::to_wstring(elapsed) + L"s elapsed";
    RECT caption{24, 105, rect.right - 24, 133};
    DrawTextW(buffer, count.c_str(), -1, &caption, DT_CENTER | DT_SINGLELINE);
    RECT track{32, 144, rect.right - 32, 154};
    HBRUSH dim = CreateSolidBrush(RGB(65, 70, 80));
    FillRect(buffer, &track, dim);
    DeleteObject(dim);
    RECT progress = track;
    progress.right = progress.left + static_cast<LONG>((track.right - track.left)
        * std::min(done, total) / std::max(size_t{1}, total));
    HBRUSH bright = CreateSolidBrush(RGB(110, 185, 245));
    FillRect(buffer, &progress, bright);
    // A moving dot remains visible even during one long state construction.
    int x = 32 + static_cast<int>((GetTickCount64() - overlay.started) / 12
        % static_cast<ULONGLONG>(rect.right - 64));
    RECT dot{x - 4, 170, x + 4, 178};
    FillRect(buffer, &dot, bright);
    DeleteObject(bright);
    RECT hint{24, 193, rect.right - 24, 225};
    SetTextColor(buffer, RGB(175, 180, 190));
    DrawTextW(buffer, L"State size varies; a step may take several seconds.", -1,
        &hint, DT_CENTER | DT_SINGLELINE);
    BitBlt(dc, 0, 0, rect.right, rect.bottom, buffer, 0, 0, SRCCOPY);
    SelectObject(buffer, oldFont);
    DeleteObject(font);
    SelectObject(buffer, oldBitmap);
    DeleteObject(bitmap);
    DeleteDC(buffer);
    EndPaint(window, &ps);
}

LRESULT CALLBACK procedure(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    auto overlay = reinterpret_cast<Overlay*>(GetWindowLongPtrW(window, GWLP_USERDATA));
    if (message == WM_NCCREATE) {
        overlay = static_cast<Overlay*>(reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
        SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(overlay));
    }
    switch (message) {
    case WM_TIMER:
        InvalidateRect(window, nullptr, FALSE);
        return 0;
    case WM_PAINT:
        if (overlay) paint(window, *overlay);
        return 0;
    case WM_ERASEBKGND:
        return 1;
    case WM_CLOSE:
        return 0; // Only the host may finish this non-interactive overlay.
    case WM_APP:
        DestroyWindow(window);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    default:
        return DefWindowProcW(window, message, wparam, lparam);
    }
}
}

extern "C" UH_API void* uh_shutdown_overlay_start(intptr_t parent) {
    RECT bounds{};
    if (!GetWindowRect(reinterpret_cast<HWND>(parent), &bounds)) return nullptr;
    auto overlay = std::make_unique<Overlay>();
    std::promise<bool> ready;
    auto result = ready.get_future();
    try {
        overlay->thread = std::thread([data = overlay.get(), bounds, ready = std::move(ready)]() mutable {
            HINSTANCE module = nullptr;
            GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                reinterpret_cast<LPCWSTR>(&procedure), &module);
            WNDCLASSW cls{};
            cls.lpfnWndProc = procedure;
            cls.hInstance = module;
            cls.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
            cls.lpszClassName = L"CatPlayerShutdownOverlay";
            if (!RegisterClassW(&cls) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) {
                ready.set_value(false);
                return;
            }
            // Unowned: survives main-window destruction through plugin teardown.
            HWND window = CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                cls.lpszClassName, L"Saving", WS_POPUP | WS_BORDER,
                (bounds.left + bounds.right - 500) / 2,
                (bounds.top + bounds.bottom - 242) / 2,
                500, 242, nullptr, nullptr, module, data);
            data->window = window;
            if (!window) {
                ready.set_value(false);
                return;
            }
            SetTimer(window, 1, 33, nullptr);
            ShowWindow(window, SW_SHOWNOACTIVATE);
            UpdateWindow(window); // First paint precedes any blocking state call.
            ready.set_value(true);
            MSG message;
            while (GetMessageW(&message, nullptr, 0, 0) > 0) {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            UnregisterClassW(cls.lpszClassName, module);
        });
    } catch (...) {
        return nullptr;
    }
    if (!result.get()) {
        overlay->thread.join();
        return nullptr;
    }
    return overlay.release();
}

extern "C" UH_API void uh_shutdown_overlay_update(void* handle, const char* text,
    size_t done, size_t total) {
    auto& overlay = *static_cast<Overlay*>(handle);
    int count = MultiByteToWideChar(CP_UTF8, 0, text, -1, nullptr, 0);
    std::wstring wide(static_cast<size_t>(std::max(count, 1)), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, text, -1, wide.data(), count);
    {
        std::lock_guard lock(overlay.mutex);
        overlay.text = std::move(wide);
        overlay.done = done;
        overlay.total = total;
    }
    InvalidateRect(overlay.window, nullptr, FALSE);
}

extern "C" UH_API void uh_shutdown_overlay_stop(void* handle) {
    std::unique_ptr<Overlay> overlay(static_cast<Overlay*>(handle));
    PostMessageW(overlay->window, WM_APP, 0, 0);
    overlay->thread.join();
}
#else
extern "C" UH_API void* uh_shutdown_overlay_start(intptr_t) { return nullptr; }
extern "C" UH_API void uh_shutdown_overlay_update(void*, const char*, size_t, size_t) {}
extern "C" UH_API void uh_shutdown_overlay_stop(void*) {}
#endif
