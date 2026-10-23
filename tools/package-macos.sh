#!/usr/bin/env bash
# Builds "Isai Studio Pro.app" and a .dmg for macOS from the release binary.
# Needs Homebrew Qt 6.9+ (qtbase, qtdeclarative, qtshadertools, qtsvg, qtmultimedia) and Pillow.
# Output: dist/macos/Isai-Studio-Pro-<version>-<architecture>.dmg
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT=dist/macos
APP="$OUT/Isai Studio Pro.app"
ICON_TMP=$(mktemp -d)
trap 'rm -rf "$ICON_TMP"' EXIT

cargo build --release -p isai

rm -rf "$OUT" && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/isai "$APP/Contents/MacOS/isai"

# App icon: the light mark, centred on the app's dark ground.
python3 - "$ICON_TMP" <<'PY'
import sys
from PIL import Image
tmp = sys.argv[1]
size = 1024
icon = Image.new("RGBA", (size, size), (10, 10, 12, 255))
mark = Image.open("crates/app/assets/brand/isai-mark-light.png").convert("RGBA")
target = int(size * 0.6)
k = target / max(mark.size)
mark = mark.resize((max(1, int(mark.width * k)), max(1, int(mark.height * k))), Image.LANCZOS)
icon.alpha_composite(mark, ((size - mark.width) // 2, (size - mark.height) // 2))
os_iconset = tmp + "/icon.iconset"
import os
os.makedirs(os_iconset)
for s in [16, 32, 64, 128, 256, 512, 1024]:
    icon.resize((s, s), Image.LANCZOS).save(f"{os_iconset}/icon_{s}x{s}.png")
    if s <= 512:
        icon.resize((s * 2, s * 2), Image.LANCZOS).save(f"{os_iconset}/icon_{s}x{s}@2x.png")
PY
iconutil -c icns "$ICON_TMP/icon.iconset" -o "$APP/Contents/Resources/Isai.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Isai Studio Pro</string>
  <key>CFBundleDisplayName</key><string>Isai Studio Pro</string>
  <key>CFBundleExecutable</key><string>isai</string>
  <key>CFBundleIdentifier</key><string>org.isai.studiopro</string>
  <key>CFBundleIconFile</key><string>Isai</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Isai records from the microphone or audio interface you choose.</string>
  <key>NSHumanReadableCopyright</key><string>GPL-3.0</string>
</dict>
</plist>
PLIST

# Qt libraries, plugins and QML go inside the app. Some optional Qt plugins
# report unresolved paths; the app runs without them, so those lines are not fatal.
macdeployqt "$APP" -qmldir=crates/app/qml -no-strip

# Video plays through Qt's FFmpeg backend (every format). macdeployqt may not
# ship that plugin on its own, so copy it in and deploy again to bring its FFmpeg libraries.
MEDIA_PLUGIN="$(brew --prefix qtmultimedia)/share/qt/plugins/multimedia/libffmpegmediaplugin.dylib"
if [ ! -f "$APP/Contents/PlugIns/multimedia/libffmpegmediaplugin.dylib" ]; then
  mkdir -p "$APP/Contents/PlugIns/multimedia"
  cp "$MEDIA_PLUGIN" "$APP/Contents/PlugIns/multimedia/"
  macdeployqt "$APP" -qmldir=crates/app/qml -no-strip
fi

# Licences and notices travel with the app.
cp LICENSE THIRD-PARTY-NOTICES.md "$APP/Contents/Resources/"
mkdir -p "$APP/Contents/Resources/licenses"
cp -R licenses/. "$APP/Contents/Resources/licenses/"
cp -R crates/app/assets/fonts/licenses "$APP/Contents/Resources/licenses/fonts"

# Ad-hoc signature: the app runs, but macOS asks once before opening it (not notarised).
codesign --force --deep -s - "$APP"
codesign --verify --deep --strict "$APP"

DMG="$OUT/Isai-Studio-Pro-$VERSION-$(uname -m).dmg"
hdiutil create -quiet -volname "Isai Studio Pro" -srcfolder "$APP" -ov -format UDZO "$DMG"
hdiutil verify "$DMG"
echo "Built $DMG"
