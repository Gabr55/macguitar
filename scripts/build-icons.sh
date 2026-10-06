#!/usr/bin/env bash
# Regenerate the app icons from resources/icon/icon.svg.
# Needs rsvg-convert (brew install librsvg) and, for the .icns, macOS iconutil.
set -euo pipefail

cd "$(dirname "$0")/.."
src=resources/icon/icon.svg
out=resources/icon

# window icon used at runtime on Linux and Windows
rsvg-convert -w 256 -h 256 "$src" -o "$out/icon-256.png"

if command -v iconutil >/dev/null; then
  set_dir=$(mktemp -d)/macguitar.iconset
  mkdir -p "$set_dir"
  for size in 16 32 128 256 512; do
    rsvg-convert -w "$size" -h "$size" "$src" -o "$set_dir/icon_${size}x${size}.png"
    rsvg-convert -w $((size * 2)) -h $((size * 2)) "$src" -o "$set_dir/icon_${size}x${size}@2x.png"
  done
  iconutil -c icns "$set_dir" -o "$out/macguitar.icns"
fi
echo "icons written to $out"
