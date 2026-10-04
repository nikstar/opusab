#!/bin/sh
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo"
[ "$(uname -s)" = Darwin ] || { echo 'This packaging script targets macOS.' >&2; exit 1; }
cargo=${CARGO:-cargo}
freac=${FREAC_APP:-/Applications/freac.app}
ffmpeg=${FFMPEG_RUNTIME:-"$repo/.local/ffmpeg-runtime"}
[ -x "$freac/Contents/MacOS/freaccmd" ] || { echo 'Set FREAC_APP to an official freac.app bundle.' >&2; exit 1; }
[ -x "$ffmpeg/bin/ffprobe" ] || { echo 'Run scripts/build-ffmpeg.sh first.' >&2; exit 1; }
"$cargo" build --release --locked
version=$(target/release/opusab --version | cut -d ' ' -f 2)
name="opusab-$version-macos-$(uname -m)"
stage=$(mktemp -d "$repo/.local/package.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
bundle="$stage/$name"
mkdir -p "$bundle/runtime/bin" "$bundle/licenses/ffmpeg" "$bundle/source"
cp target/release/opusab README.md LICENSE "$bundle/"
cp docs/THIRD_PARTY.md "$bundle/"
cp -R docs "$bundle/docs"
cp scripts/install.sh "$bundle/"
ditto "$freac" "$bundle/runtime/freac.app"
cp "$ffmpeg/bin/ffmpeg" "$ffmpeg/bin/ffprobe" "$bundle/runtime/bin/"
cp "$ffmpeg/licenses/"* "$bundle/licenses/ffmpeg/"
cp "$ffmpeg/source/"* "$bundle/source/"
cp scripts/build-ffmpeg.sh "$bundle/source/"
# Never ship dependencies that accidentally refer to this developer's Homebrew tree.
python3 scripts/check-bundle.py "$bundle"
mkdir -p dist
[ ! -e "dist/$name" ] || { echo "dist/$name already exists; move it aside before rebuilding." >&2; exit 1; }
mv "$bundle" "dist/$name"
COPYFILE_DISABLE=1 tar -czf "dist/$name.tar.gz" -C dist "$name"
shasum -a 256 "dist/$name.tar.gz" > "dist/$name.tar.gz.sha256"
echo "Portable package: dist/$name.tar.gz"
