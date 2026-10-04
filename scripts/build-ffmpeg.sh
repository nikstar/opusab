#!/bin/sh
# Build a small FFmpeg/ffprobe pair with no third-party dynamic dependencies.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=9.0.2
sha256=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
cache="$repo/.local/downloads"
source_dir="$repo/.local/ffmpeg-$version"
prefix="$repo/.local/ffmpeg-runtime"
archive="$cache/ffmpeg-$version.tar.xz"
mkdir -p "$cache" "$prefix"
if [ ! -f "$archive" ]; then
    curl --fail --location --retry 3 "https://ffmpeg.org/releases/ffmpeg-$version.tar.xz" -o "$archive.part"
    mv "$archive.part" "$archive"
fi
actual=$(shasum -a 256 "$archive" | cut -d ' ' -f 1)
[ "$actual" = "$sha256" ] || { echo 'FFmpeg source checksum mismatch' >&2; exit 1; }
if [ ! -d "$source_dir" ]; then tar -xf "$archive" -C "$repo/.local"; fi
cd "$source_dir"
if [ "$(uname -s)" = Darwin ]; then
    export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
fi
./configure --prefix="$prefix" \
    --disable-autodetect --disable-everything --disable-network \
    --disable-doc --disable-debug --disable-shared --enable-static \
    --disable-avdevice --disable-programs --enable-ffmpeg --enable-ffprobe \
    --enable-zlib --enable-protocol=file,pipe \
    --enable-demuxer=mov,mp3,ogg,flac,wav,aiff,aac,matroska \
    --enable-decoder=aac,aac_fixed,mp3,mp3float,flac,alac,opus,vorbis,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_f64le,pcm_s16be,pcm_s24be,pcm_s32be,mjpeg,png \
    --enable-parser=aac,mpegaudio,flac,opus,vorbis,mjpeg,png \
    --enable-muxer=flac,image2,null,wav \
    --enable-encoder=flac,mjpeg,png,pcm_s16le,wrapped_avframe \
    --enable-filter=aresample,aformat,anull,format,scale,null \
    "$@"
jobs=${OPUSAB_BUILD_JOBS:-8}
# A changed deployment target must also rebuild existing object files.
make clean
make -j "$jobs"
make install
mkdir -p "$prefix/licenses" "$prefix/source"
cp COPYING.LGPLv2.1 LICENSE.md "$prefix/licenses/"
cp ffbuild/config.log "$prefix/source/"
cp "$archive" "$prefix/source/"
echo "Built FFmpeg runtime at $prefix"
