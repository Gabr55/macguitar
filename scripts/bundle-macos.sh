#!/usr/bin/env bash
# Build MacGuitar.app for Apple Silicon (arm64), with its icon, and a zip of
# it ready to share.
#
#   scripts/bundle-macos.sh            build target/macos-arm64/MacGuitar.app
#   scripts/bundle-macos.sh --install  also copy it to /Applications
#   scripts/bundle-macos.sh --debug    bundle the debug build (faster to compile)
set -euo pipefail

cd "$(dirname "$0")/.."

profile=release
install=false
for arg in "$@"; do
  case "$arg" in
    --install) install=true ;;
    --debug) profile=debug ;;
    *) echo "unknown option: $arg" >&2; exit 1 ;;
  esac
done

target=aarch64-apple-darwin
rustup target add "$target" >/dev/null 2>&1 || true
if [ "$profile" = release ]; then
  cargo build --release --target "$target"
else
  cargo build --target "$target"
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
out=target/macos-arm64
app="$out/MacGuitar.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "target/$target/$profile/macguitar" "$app/Contents/MacOS/macguitar"
cp resources/icon/macguitar.icns "$app/Contents/Resources/macguitar.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>MacGuitar</string>
  <key>CFBundleDisplayName</key><string>MacGuitar</string>
  <key>CFBundleIdentifier</key><string>io.github.gabr55.macguitar</string>
  <key>CFBundleExecutable</key><string>macguitar</string>
  <key>CFBundleIconFile</key><string>macguitar</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>CFBundleGetInfoString</key><string>MacGuitar $version for macOS on Apple Silicon (arm64)</string>
  <key>LSArchitecturePriority</key><array><string>arm64</string></array>
  <key>LSRequiresNativeExecution</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
PLIST

# an ad-hoc signature keeps Gatekeeper from calling the bundle damaged
codesign --force --deep --sign - "$app" >/dev/null 2>&1 || true

# the zip keeps the bundle's attributes and signature
zip="$out/MacGuitar-$version-macos-arm64.zip"
rm -f "$zip"
ditto -c -k --keepParent "$app" "$zip"

echo "built $app ($(lipo -archs "$app/Contents/MacOS/macguitar"))"
echo "built $zip"

if [ "$install" = true ]; then
  rm -rf /Applications/MacGuitar.app
  cp -R "$app" /Applications/
  # make Finder and the Dock pick up the new icon
  touch /Applications/MacGuitar.app
  echo "installed /Applications/MacGuitar.app"
fi
