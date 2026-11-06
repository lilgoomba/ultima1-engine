#!/usr/bin/env bash
# Builds ultima1-engine-<version>-x86_64.AppImage in build/: the release
# binary, SDL2 and its libraries (by linuxdeploy), the enhanced assets
# (CC-BY-SA 4.0), our icon and .desktop file. No game files: the player
# brings their own (see PLAYING.md). Built on Ubuntu 22.04 in CI so the
# AppImage runs on distributions with glibc 2.35 or newer.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=${VERSION:-$(git describe --tags --always --dirty)}
TARGET_DIR=${CARGO_TARGET_DIR:-target}
cargo build --release --locked -p u1

APPDIR=build/AppDir
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/share/ultima1-engine" "$APPDIR/usr/share/doc/ultima1-engine"
cp -r assets/enhanced "$APPDIR/usr/share/ultima1-engine/enhanced"
cp LICENSE PLAYING.md CREDITS "$APPDIR/usr/share/doc/ultima1-engine/"

# Safety net: nothing that looks like original game data goes in.
bad=$(find "$APPDIR" -type f \( -iname '*.exe' -o -iname '*.bin' -o -iname '*.u1' -o -iname '*.16' -o -iname '*.4' -o -iname '*.com' \) -print)
if [ -n "$bad" ]; then
    echo "refusing to package game data:"; echo "$bad"; exit 1
fi

LINUXDEPLOY=${LINUXDEPLOY:-build/linuxdeploy-x86_64.AppImage}
if [ ! -x "$LINUXDEPLOY" ]; then
    curl -fsSL -o "$LINUXDEPLOY" https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x "$LINUXDEPLOY"
fi
# Run the tool's AppImage without FUSE (CI runners, containers).
export APPIMAGE_EXTRACT_AND_RUN=1
export LINUXDEPLOY_OUTPUT_VERSION="$VERSION"
OUT="ultima1-engine-$VERSION-x86_64.AppImage"
LDAI_OUTPUT="$OUT" OUTPUT="$OUT" "$LINUXDEPLOY" --appdir "$APPDIR" \
    --executable "$TARGET_DIR/release/u1" \
    --desktop-file packaging/ultima1-engine.desktop \
    --icon-file packaging/ultima1-engine.png \
    --output appimage
mv "$OUT" build/
echo "built build/$OUT"
