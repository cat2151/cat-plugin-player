#pragma once

#if _WIN32
#include <Windows.h>
#include <algorithm>

namespace uh {
// Own both selected GDI objects and their DC; restore selection before deletion.
struct CaptureBitmap {
    HDC dc = nullptr;
    HBITMAP bitmap = nullptr;
    HGDIOBJ previous = nullptr;
    CaptureBitmap(HDC source, int width, int height) {
        dc = CreateCompatibleDC(source);
        if (dc) bitmap = CreateCompatibleBitmap(source, width, height);
        if (bitmap) previous = SelectObject(dc, bitmap);
    }
    ~CaptureBitmap() {
        if (previous && previous != HGDI_ERROR) SelectObject(dc, previous);
        if (bitmap) DeleteObject(bitmap);
        if (dc) DeleteDC(dc);
    }
    bool valid() const { return dc && bitmap && previous && previous != HGDI_ERROR; }
};

inline bool captureThumbnail(HWND window, uint8_t* rgba, int32_t* width, int32_t* height) {
    RECT rect{};
    if (!IsWindowVisible(window) || IsIconic(window) || !GetClientRect(window, &rect))
        return false;
    const int w = rect.right, h = rect.bottom;
    // Bound memory consumed by unusually large or broken editor dimensions.
    if (w <= 0 || h <= 0 || w > 8192 || h > 8192 || int64_t(w) * h > 16777216)
        return false;
    const double scale = std::min({1.0, 192.0 / w, 128.0 / h});
    const int tw = std::max(1, int(w * scale)), th = std::max(1, int(h * scale));
    HDC screen = GetDC(window);
    if (!screen) return false;
    bool success = false;
    {
        CaptureBitmap full(screen, w, h), thumbnail(screen, tw, th);
        if (full.valid() && thumbnail.valid()) {
            PatBlt(full.dc, 0, 0, w, h, BLACKNESS);
            // PW_RENDERFULLCONTENT includes composited content on modern Windows.
            if (PrintWindow(window, full.dc, PW_CLIENTONLY | 0x00000002)) {
                SetStretchBltMode(thumbnail.dc, HALFTONE);
                SetBrushOrgEx(thumbnail.dc, 0, 0, nullptr);
                if (StretchBlt(thumbnail.dc, 0, 0, tw, th, full.dc, 0, 0, w, h, SRCCOPY)) {
                    SelectObject(thumbnail.dc, thumbnail.previous);
                    BITMAPINFO info{};
                    info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
                    info.bmiHeader.biWidth = tw;
                    info.bmiHeader.biHeight = -th;
                    info.bmiHeader.biPlanes = 1;
                    info.bmiHeader.biBitCount = 32;
                    info.bmiHeader.biCompression = BI_RGB;
                    if (GetDIBits(thumbnail.dc, thumbnail.bitmap, 0, th, rgba, &info, DIB_RGB_COLORS) == th) {
                        // Some GPU editors return success with a blank image.
                        bool varied = false;
                        for (int i = 0; i < tw * th; ++i) {
                            const int offset = i * 4;
                            varied |= rgba[offset] != rgba[0] || rgba[offset + 1] != rgba[1]
                                   || rgba[offset + 2] != rgba[2];
                        }
                        if (varied) {
                            for (int i = 0; i < tw * th; ++i) {
                                std::swap(rgba[i * 4], rgba[i * 4 + 2]);
                                rgba[i * 4 + 3] = 255;
                            }
                            *width = tw;
                            *height = th;
                            success = true;
                        }
                    }
                }
            }
        }
    }
    ReleaseDC(window, screen);
    return success;
}
} // namespace uh
#endif
