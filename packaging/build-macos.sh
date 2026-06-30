#!/usr/bin/env bash
#
# Build macOS artifacts for md2pdf:
#   1. Release binaries (universal if both arches are installed, else native)
#   2. md2pdf.app           — double-clickable GUI app bundle
#   3. md2pdf-<ver>.pkg     — installer (GUI → /Applications, CLI → /usr/local/bin)
#   4. md2pdf-<ver>-macos-<arch>.tar.gz — portable archive (app + CLI)
#
# Usage: packaging/build-macos.sh
#
set -euo pipefail

# Keep AppleDouble (._*) metadata files out of the archives/payload.
export COPYFILE_DISABLE=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APP_NAME="md2pdf"
BUNDLE_ID="com.leondavi.md2pdf"
VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"

BUILD="$ROOT/packaging/build"
DIST="$ROOT/dist"
APP="$BUILD/$APP_NAME.app"

echo "==> md2pdf $VERSION — macOS packaging"
rm -rf "$BUILD" "$DIST"
mkdir -p "$BUILD" "$DIST"

# ---------------------------------------------------------------------------
# 1. Build release binaries (universal when possible)
# ---------------------------------------------------------------------------
HOST_ARCH="$(uname -m)"
INSTALLED="$(rustup target list --installed 2>/dev/null || true)"
have_arm=$(echo "$INSTALLED" | grep -c '^aarch64-apple-darwin' || true)
have_x86=$(echo "$INSTALLED" | grep -c '^x86_64-apple-darwin' || true)

build_target() { # $1 = target triple
    echo "==> cargo build --release --target $1"
    cargo build --release --target "$1"
}

CLI_BIN=""
GUI_BIN=""
PKG_ARCH=""

if [[ "$have_arm" -ge 1 && "$have_x86" -ge 1 ]]; then
    build_target aarch64-apple-darwin
    build_target x86_64-apple-darwin
    CLI_BIN="$BUILD/md2pdf"
    GUI_BIN="$BUILD/md2pdf-gui"
    lipo -create -output "$CLI_BIN" \
        target/aarch64-apple-darwin/release/md2pdf \
        target/x86_64-apple-darwin/release/md2pdf
    lipo -create -output "$GUI_BIN" \
        target/aarch64-apple-darwin/release/md2pdf-gui \
        target/x86_64-apple-darwin/release/md2pdf-gui
    PKG_ARCH="universal"
    echo "==> built universal binaries"
else
    echo "==> single-arch build for $HOST_ARCH (install the other target for a universal build)"
    cargo build --release
    CLI_BIN="$ROOT/target/release/md2pdf"
    GUI_BIN="$ROOT/target/release/md2pdf-gui"
    PKG_ARCH="$HOST_ARCH"
fi

# ---------------------------------------------------------------------------
# 2. Generate the icon (best effort — skipped if Python/PIL unavailable)
# ---------------------------------------------------------------------------
ICNS=""
if python3 -c 'import PIL' >/dev/null 2>&1; then
    echo "==> generating app icon"
    python3 packaging/make_icon.py
    ICONSET="$BUILD/AppIcon.iconset"
    mkdir -p "$ICONSET"
    MASTER="$ROOT/packaging/icon-master.png"
    for s in 16 32 64 128 256 512 1024; do
        sips -z $s $s "$MASTER" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
    done
    # @2x variants
    cp "$ICONSET/icon_32x32.png"   "$ICONSET/icon_16x16@2x.png"
    cp "$ICONSET/icon_64x64.png"   "$ICONSET/icon_32x32@2x.png"
    cp "$ICONSET/icon_256x256.png" "$ICONSET/icon_128x128@2x.png"
    cp "$ICONSET/icon_512x512.png" "$ICONSET/icon_256x256@2x.png"
    cp "$ICONSET/icon_1024x1024.png" "$ICONSET/icon_512x512@2x.png"
    rm -f "$ICONSET/icon_64x64.png" "$ICONSET/icon_1024x1024.png"
    ICNS="$BUILD/AppIcon.icns"
    iconutil -c icns "$ICONSET" -o "$ICNS"
else
    echo "==> PIL not available; building without a custom icon"
fi

# ---------------------------------------------------------------------------
# 3. Assemble the .app bundle
# ---------------------------------------------------------------------------
echo "==> assembling $APP_NAME.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
sed "s/__VERSION__/$VERSION/g" packaging/Info.plist > "$APP/Contents/Info.plist"
cp "$GUI_BIN" "$APP/Contents/MacOS/md2pdf"
chmod +x "$APP/Contents/MacOS/md2pdf"
printf 'APPL????' > "$APP/Contents/PkgInfo"
[[ -n "$ICNS" ]] && cp "$ICNS" "$APP/Contents/Resources/AppIcon.icns"

# Ad-hoc code signature (lets the app run locally without "is damaged" errors).
echo "==> ad-hoc signing"
codesign --force --deep --sign - "$APP" 2>/dev/null || echo "   (codesign skipped)"

# ---------------------------------------------------------------------------
# 4. Staging tree for the installer pkg
# ---------------------------------------------------------------------------
STAGE="$BUILD/stage"
mkdir -p "$STAGE/Applications" "$STAGE/usr/local/bin"
cp -R "$APP" "$STAGE/Applications/"
cp "$CLI_BIN" "$STAGE/usr/local/bin/md2pdf"
chmod +x "$STAGE/usr/local/bin/md2pdf"
# Strip extended attributes so no AppleDouble files end up in the payload,
# then (re-)sign — the CLI's ad-hoc signature is stored in an xattr, so it must
# be applied AFTER cleaning.
find "$STAGE" -name '._*' -delete 2>/dev/null || true
xattr -cr "$STAGE" 2>/dev/null || true
codesign --force --deep --sign - "$STAGE/Applications/$APP_NAME.app" 2>/dev/null || true
codesign --force --sign - "$STAGE/usr/local/bin/md2pdf" 2>/dev/null || true

COMPONENT="$BUILD/md2pdf-component.pkg"
pkgbuild \
    --root "$STAGE" \
    --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    --install-location "/" \
    "$COMPONENT"

# Distribution wrapper with a license screen.
DIST_XML="$BUILD/distribution.xml"
cat > "$DIST_XML" <<XML
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>md2pdf $VERSION</title>
    <license file="LICENSE"/>
    <options customize="never" require-scripts="false" hostArchitectures="arm64,x86_64"/>
    <choices-outline>
        <line choice="default"/>
    </choices-outline>
    <choice id="default" title="md2pdf">
        <pkg-ref id="$BUNDLE_ID"/>
    </choice>
    <pkg-ref id="$BUNDLE_ID" version="$VERSION" onConclusion="none">md2pdf-component.pkg</pkg-ref>
</installer-gui-script>
XML

PKG="$DIST/md2pdf-$VERSION.pkg"
productbuild \
    --distribution "$DIST_XML" \
    --package-path "$BUILD" \
    --resources "$ROOT" \
    "$PKG"

# ---------------------------------------------------------------------------
# 5. Portable tarball (app + CLI + docs)
# ---------------------------------------------------------------------------
TAR_DIR="$BUILD/md2pdf-$VERSION"
mkdir -p "$TAR_DIR"
cp -R "$APP" "$TAR_DIR/"
cp "$CLI_BIN" "$TAR_DIR/md2pdf"
cp README.md LICENSE "$TAR_DIR/" 2>/dev/null || true
TARBALL="$DIST/md2pdf-$VERSION-macos-$PKG_ARCH.tar.gz"
tar -czf "$TARBALL" -C "$BUILD" "md2pdf-$VERSION"

# ---------------------------------------------------------------------------
# 6. Drag-to-install DMG (md2pdf.app + Applications symlink)
# ---------------------------------------------------------------------------
echo "==> building DMG"
DMG_SRC="$BUILD/dmg"
rm -rf "$DMG_SRC"
mkdir -p "$DMG_SRC"
cp -R "$APP" "$DMG_SRC/"
ln -s /Applications "$DMG_SRC/Applications"
# A short note plus the optional CLI for power users.
cat > "$DMG_SRC/README.txt" <<TXT
md2pdf $VERSION

To install: drag md2pdf.app onto the Applications folder shown here.

First launch: right-click md2pdf.app -> Open (the app is signed ad-hoc,
not notarized, so Gatekeeper asks for confirmation the first time only).

Optional command-line tool: copy the 'md2pdf' binary in the 'cli' folder
to /usr/local/bin (or anywhere on your PATH).
TXT
mkdir -p "$DMG_SRC/cli"
cp "$CLI_BIN" "$DMG_SRC/cli/md2pdf"
chmod +x "$DMG_SRC/cli/md2pdf"
find "$DMG_SRC" -name '._*' -delete 2>/dev/null || true

DMG="$DIST/md2pdf-$VERSION-macos-$PKG_ARCH.dmg"
rm -f "$DMG"
hdiutil create \
    -volname "md2pdf $VERSION" \
    -srcfolder "$DMG_SRC" \
    -fs HFS+ \
    -format UDZO \
    -ov \
    "$DMG" >/dev/null
echo "==> wrote $DMG"

# ---------------------------------------------------------------------------
echo
echo "==> Done. Artifacts in dist/:"
ls -lh "$DIST"
echo
echo "App bundle: $APP"
