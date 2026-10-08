// Compile with MSVC /std:c++20 /EHsc /DNOMINMAX /Ishim; link user32.lib.
#include "window_placement.h"
#include <cassert>
int main() {
    const RECT work{0, 0, 1920, 1040};
    auto right = uh::editorPosition({80, 80, 980, 680}, work, 546, 346);
    assert(right.x == 980 && right.y == 80);
    auto bottom = uh::editorPosition({1000, 80, 1900, 680}, work, 546, 346);
    assert(bottom.x == 1374 && bottom.y == 694);
    auto negative = uh::editorPosition({-1800, -900, -900, -300}, {-1920, -1080, 0, 0}, 546, 346);
    assert(negative.x == -900 && negative.y == -900);
    auto oversized = uh::editorPosition({80, 80, 980, 680}, work, 2400, 1400);
    assert(oversized.x == 0 && oversized.y == 0);
    auto offscreen = uh::editorPosition({-2400, -1500, -1500, -900}, work, 546, 346);
    assert(offscreen.x == 1374 && offscreen.y == 694);
}
