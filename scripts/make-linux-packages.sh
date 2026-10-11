#!/usr/bin/env bash
# Build the .deb, the .rpm and the Arch .pkg.tar.zst for one of the two Linux
# packages, from its single description in packaging/linux/<package>.nfpm.yaml.
#
# Usage:
#   ./scripts/make-linux-packages.sh <dadrock|dadrock-desktop> [binary] [arch]
#
# `binary` defaults to target/release/dadrock (or
# desktop/target/release/dadrock-desktop), `arch` to the machine's own. Both
# are arguments because the release workflow packages the binaries that were
# already built and published, cargo-dist's for the CLI and
# release-desktop.yml's for the app, for two architectures, rather than
# rebuilding them here: the bytes in the .deb are then the same bytes users
# get from the release tarballs.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

usage="usage: $0 <dadrock|dadrock-desktop> [binary] [arch]"
package="${1:-}"
case "$package" in
  dadrock)         default_binary="$root/target/release/dadrock" ;;
  dadrock-desktop) default_binary="$root/desktop/target/release/dadrock-desktop" ;;
  *) echo "$usage" >&2; exit 1 ;;
esac

version="$(awk -F'"' '/^version/ { print $2; exit }' Cargo.toml)"
target="${2:-$default_binary}"
staging="$root/dist/linux-staging"

if [ -n "${3:-}" ]; then
  arch="$3"
else
  case "$(uname -m)" in
    x86_64)  arch=amd64 ;;
    aarch64) arch=arm64 ;;
    *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
fi

case "$arch" in
  amd64|arm64) ;;
  *) echo "unsupported arch argument: $arch (expected amd64 or arm64)" >&2; exit 1 ;;
esac

if ! command -v nfpm >/dev/null 2>&1; then
  cat >&2 <<'MISSING'
nfpm is not installed.

All three packages are generated from one description rather than maintained
three times, and nfpm is what does that. Install a release from
https://github.com/goreleaser/nfpm/releases and put it on PATH.
MISSING
  exit 1
fi

if [ ! -f "$target" ]; then
  echo "no binary at $target" >&2
  exit 1
fi

if [ "$package" = dadrock-desktop ]; then
  # The desktop package pins `dadrock (= version)`, with the version taken
  # from the workspace manifest. A desktop crate left at another version would
  # produce a package that claims to match a CLI it was not released with.
  desktop_version="$(awk -F'"' '/^version/ { print $2; exit }' desktop/Cargo.toml)"
  if [ "$desktop_version" != "$version" ]; then
    echo "desktop/Cargo.toml is at $desktop_version but the workspace is at $version; run 'make bump'" >&2
    exit 1
  fi
  for icon in 32x32.png 128x128.png 128x128@2x.png icon.png; do
    [ -f "desktop/icons/$icon" ] || { echo "missing desktop/icons/$icon" >&2; exit 1; }
  done
fi

# A binary built on NixOS names its ELF interpreter in /nix/store, so the
# package installs cleanly on Debian and then fails with "cannot execute:
# required file not found". The release workflow is unaffected because it
# packages binaries built on Ubuntu runners, but a local `make linux-packages`
# on NixOS would otherwise produce a package that looks fine and does not run.
if command -v readelf >/dev/null 2>&1 &&
   readelf -p .interp "$target" 2>/dev/null | grep -q /nix/store; then
  echo "refusing to package: $target has a Nix ELF interpreter" >&2
  echo "unpack a release tarball and package that binary instead:" >&2
  echo "  ./scripts/make-linux-packages.sh $package /path/to/unpacked/$package amd64" >&2
  exit 1
fi

rm -rf "$staging"
mkdir -p "$staging" "$root/dist"
install -m755 "$target" "$staging/$package"

# deb and rpm spell a prerelease with a tilde (nfpm writes 1.2.3-rc.1 as
# 1.2.3~rc.1, so that it sorts before 1.2.3), and an exact dependency on the
# CLI package has to use the spelling the CLI package was actually given.
dep_version="$(printf '%s' "$version" | sed 's/-/~/')"

export DADROCK_VERSION="$version" DADROCK_ARCH="$arch" DADROCK_DEP_VERSION="$dep_version"
for packager in deb rpm archlinux; do
  nfpm package --packager "$packager" \
    --config "packaging/linux/$package.nfpm.yaml" --target dist/
done

rm -rf "$staging"

# The `[0-9]` keeps the CLI's globs from also listing the desktop package,
# whose name starts with the CLI's.
for artifact in dist/"${package}"_*.deb dist/"${package}"-[0-9]*.rpm dist/"${package}"-[0-9]*.pkg.tar.zst; do
  [ -e "$artifact" ] || continue
  echo "package: $artifact"
done
