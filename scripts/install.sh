#!/bin/sh
# Install the latest MacGuitar release on an Apple Silicon Mac:
#
#   curl -fsSL https://raw.githubusercontent.com/Gabr55/macguitar/master/scripts/install.sh | sh
#
# Downloads MacGuitar.app from the latest GitHub release and puts it in
# /Applications, or in ~/Applications when /Applications is not writable,
# or in $MACGUITAR_INSTALL_DIR when set. To uninstall, delete the app.
#
# Everything lives inside main, called on the last line, so a download cut
# off halfway runs nothing rather than half a script.
set -eu

repo="Gabr55/macguitar"

say() { printf '%s\n' "$*"; }
die() { printf 'macguitar install: %s\n' "$*" >&2; exit 1; }

main() {
  command -v curl >/dev/null 2>&1 || die "curl is required"

  [ "$(uname -s)" = "Darwin" ] ||
    die "releases are built for macOS; elsewhere, build from source: cargo install --locked --git https://github.com/$repo"
  # an x86_64 shell under Rosetta reports x86_64 on an Apple Silicon Mac
  if [ "$(uname -m)" != "arm64" ] &&
    [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" != "1" ]; then
    die "releases are built for Apple Silicon; on an Intel Mac, build from source: cargo install --locked --git https://github.com/$repo"
  fi

  # the latest tag, read off the redirect of releases/latest rather than
  # from the API, which rate-limits anonymous callers
  latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
    "https://github.com/$repo/releases/latest") ||
    die "could not reach GitHub"
  tag=${latest##*/}
  case "$tag" in
    v[0-9]*) ;;
    *) die "could not find the latest release (got $latest)" ;;
  esac
  archive="MacGuitar-${tag#v}-macos-arm64.zip"

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  say "Downloading MacGuitar $tag"
  curl -fL --progress-bar -o "$tmp/$archive" \
    "https://github.com/$repo/releases/download/$tag/$archive" ||
    die "could not download $archive"
  ditto -x -k "$tmp/$archive" "$tmp" || die "could not unpack $archive"
  [ -d "$tmp/MacGuitar.app" ] || die "$archive holds no MacGuitar.app"

  dest=${MACGUITAR_INSTALL_DIR:-/Applications}
  if [ -z "${MACGUITAR_INSTALL_DIR:-}" ] && [ ! -w "$dest" ]; then
    dest="$HOME/Applications"
  fi
  mkdir -p "$dest"
  rm -rf "$dest/MacGuitar.app"
  ditto "$tmp/MacGuitar.app" "$dest/MacGuitar.app"
  say "Installed $dest/MacGuitar.app"
}

main "$@"
