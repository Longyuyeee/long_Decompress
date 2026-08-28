#!/usr/bin/env bash
set -euo pipefail

EXPECTED_SOURCE_SHA256="cf38e0e28c7e5605942c4a77755349b0145804a397af37eb1fb4c77cb237f635"
SOURCE_DATE_EPOCH="1786505700"
FIXED_PREFIX="/opt/long-decompress/ffmpeg-9.0.1"

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <ffmpeg-9.0.1.tar.xz> <build-directory> <output-directory>" >&2
  exit 2
fi

source_archive="$(realpath "$1")"
build_directory="$(realpath -m "$2")"
output_directory="$(realpath -m "$3")"

assert_safe_target() {
  local target="$1"
  [[ "$target" == */long-decompress-ffmpeg-c01/* || "$target" == */test-results/video-c01-audit/* ]] || {
    echo "refusing unsafe cleanup target: $target" >&2
    exit 1
  }
}

require_package_version() {
  local package="$1"
  local expected="$2"
  local actual
  actual="$(dpkg-query -W -f='${Version}' "$package" 2>/dev/null || true)"
  [[ "$actual" == "$expected" ]] || {
    echo "build package mismatch: $package expected=$expected actual=${actual:-missing}" >&2
    exit 1
  }
}

assert_safe_target "$build_directory"
assert_safe_target "$output_directory"

for tool in sha256sum tar make nasm x86_64-w64-mingw32-gcc x86_64-w64-mingw32-strip; do
  command -v "$tool" >/dev/null || { echo "missing required build tool: $tool" >&2; exit 1; }
done

command -v dpkg-query >/dev/null || { echo "dpkg-query is required to freeze the Ubuntu build toolchain" >&2; exit 1; }
require_package_version gcc-mingw-w64-x86-64-posix 13.2.0-6ubuntu1+26.1
require_package_version mingw-w64-x86-64-dev 11.0.1-3build1
require_package_version binutils-mingw-w64-x86-64 2.41.90.20240122-1ubuntu1+11.4
require_package_version nasm 2.16.01-1build1
require_package_version make 4.3-4.1build2
[[ "$(x86_64-w64-mingw32-gcc --version | head -n 1)" == "x86_64-w64-mingw32-gcc (GCC) 13-posix" ]] || {
  echo "x86_64-w64-mingw32-gcc must resolve to the frozen POSIX compiler alternative" >&2
  exit 1
}

actual_source_sha256="$(sha256sum "$source_archive" | cut -d' ' -f1)"
[[ "$actual_source_sha256" == "$EXPECTED_SOURCE_SHA256" ]] || {
  echo "FFmpeg source SHA-256 mismatch: expected=$EXPECTED_SOURCE_SHA256 actual=$actual_source_sha256" >&2
  exit 1
}

rm -rf "$build_directory" "$output_directory"
mkdir -p "$build_directory" "$output_directory/bin" "$output_directory/licenses"
tar -xJf "$source_archive" -C "$build_directory"
mv "$build_directory/ffmpeg-9.0.1" "$build_directory/src"
mkdir -p "$build_directory/build"
cd "$build_directory/build"

export SOURCE_DATE_EPOCH
../src/configure \
  --target-os=mingw32 \
  --arch=x86_64 \
  --enable-cross-compile \
  --cross-prefix=x86_64-w64-mingw32- \
  --prefix="$FIXED_PREFIX" \
  --disable-everything \
  --disable-autodetect \
  --disable-debug \
  --disable-doc \
  --disable-shared \
  --enable-static \
  --disable-network \
  --disable-schannel \
  --disable-bzlib \
  --disable-iconv \
  --disable-lzma \
  --disable-sdl2 \
  --disable-zlib \
  --enable-ffmpeg \
  --enable-ffprobe \
  --enable-avcodec \
  --enable-avformat \
  --enable-avfilter \
  --enable-swscale \
  --enable-swresample \
  --enable-mediafoundation \
  --disable-hwaccels \
  --enable-d3d11va \
  --enable-protocol=file,pipe \
  --enable-demuxer=mov,matroska,avi,asf \
  --enable-muxer=mp4 \
  --enable-decoder=h264,hevc,mpeg4,msmpeg4v1,msmpeg4v2,msmpeg4v3,wmv1,wmv2,wmv3,vc1,vp8,vp9,av1,aac,mp3,ac3,eac3,opus,vorbis,flac,wmav1,wmav2,wmapro,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le \
  --enable-encoder=h264_mf,aac \
  --enable-parser=h264,hevc,mpeg4video,vp8,vp9,av1,aac,ac3,opus,vorbis \
  --enable-filter=scale,format,fps,setsar,setdar,transpose,aresample \
  --enable-bsf=aac_adtstoasc,extract_extradata,h264_mp4toannexb,hevc_mp4toannexb \
  --extra-cflags=-O2 \
  --extra-ldflags='-static -static-libgcc -Wl,--no-insert-timestamp' \
  >"$output_directory/configure.log" 2>&1

make -j"${LONG_FFMPEG_BUILD_JOBS:-8}" ffmpeg.exe ffprobe.exe >"$output_directory/build.log" 2>&1
x86_64-w64-mingw32-strip ffmpeg.exe ffprobe.exe
cp ffmpeg.exe ffprobe.exe "$output_directory/bin/"
cp ../src/COPYING.LGPLv2.1 ../src/COPYING.LGPLv3 "$output_directory/licenses/"
cp ffbuild/config.mak "$output_directory/config.mak"

{
  echo "source=ffmpeg-9.0.1.tar.xz"
  echo "source_sha256=$actual_source_sha256"
  echo "source_date_epoch=$SOURCE_DATE_EPOCH"
  echo "fixed_prefix=$FIXED_PREFIX"
  x86_64-w64-mingw32-gcc --version | head -n 1
  nasm -v
  dpkg-query -W gcc-mingw-w64-x86-64-posix mingw-w64-x86-64-dev binutils-mingw-w64-x86-64 nasm make
  sha256sum "$output_directory/bin/ffmpeg.exe" "$output_directory/bin/ffprobe.exe"
} >"$output_directory/build-identity.txt"
