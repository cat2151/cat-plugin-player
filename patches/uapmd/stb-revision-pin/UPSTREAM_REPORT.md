# Unsent report: pin rtlog stb dependency

rtlog-cpp 36381f9b2d64f7852590dd0c2564dbfdee90e116 fetches nothings/stb
without a revision and includes stb_sprintf.h from its root. The 2026-10-19
stb move to deprecated/ breaks fresh builds. Our shared host declares stb first
at 2c980bb59875b0d32144a71867fbdebb2f77cd20 and checks the source and target.
Please pin a compatible revision or adopt the new upstream header layout.
Status: not submitted; no external communication authorized.
