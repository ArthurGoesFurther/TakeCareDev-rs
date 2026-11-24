#!/bin/sh
# Builds TakeCareDev.app (menu-bar only, no Dock icon) from the release binary.
# Usage: scripts/macos-app.sh [destination folder]   (default: dist/)
set -eu
cd "$(dirname "$0")/.."

dest="${1:-dist}"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)

cargo build --release --locked

app="$dest/TakeCareDev.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/takecaredev "$app/Contents/MacOS/takecaredev"

# App icon: the pixel face rendered at every size macOS asks for.
iconset=$(mktemp -d)/TakeCareDev.iconset
mkdir -p "$iconset"
for size in 32 64 128 256 512 1024; do
  target/release/takecaredev icon "$iconset/full_$size.png" "$size"
done
cp "$iconset/full_32.png" "$iconset/icon_16x16@2x.png"
cp "$iconset/full_32.png" "$iconset/icon_32x32.png"
cp "$iconset/full_64.png" "$iconset/icon_32x32@2x.png"
cp "$iconset/full_128.png" "$iconset/icon_128x128.png"
cp "$iconset/full_256.png" "$iconset/icon_128x128@2x.png"
cp "$iconset/full_256.png" "$iconset/icon_256x256.png"
cp "$iconset/full_512.png" "$iconset/icon_256x256@2x.png"
cp "$iconset/full_512.png" "$iconset/icon_512x512.png"
cp "$iconset/full_1024.png" "$iconset/icon_512x512@2x.png"
sips -z 16 16 "$iconset/full_32.png" --out "$iconset/icon_16x16.png" >/dev/null
rm "$iconset"/full_*.png
iconutil -c icns "$iconset" -o "$app/Contents/Resources/TakeCareDev.icns"
rm -rf "$(dirname "$iconset")"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>TakeCareDev</string>
  <key>CFBundleDisplayName</key><string>TakeCareDev</string>
  <key>CFBundleIdentifier</key><string>dev.tcdev.app</string>
  <key>CFBundleExecutable</key><string>takecaredev</string>
  <key>CFBundleIconFile</key><string>TakeCareDev</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Ad-hoc signature so macOS accepts the bundle as a whole.
codesign --force --deep --sign - "$app"
echo "built $app"
