#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
WORK="$ROOT/build/llama"
mkdir -p "$WORK" "$ROOT/build/runtime-headers"
if [ ! -d "$WORK/source/.git" ]; then git clone --depth 1 --branch b10488 https://github.com/ggml-org/llama.cpp.git "$WORK/source"; fi
test "$(git -C "$WORK/source" describe --tags --exact-match)" = b10488
cp Hatoriel/Hatoriel-Local-Header.h build/runtime-headers/
cp "$WORK/source/LICENSE" build/llama-LICENSE.txt
build_slice() {
 local name="$1" sdk="$2" arch="$3"
 local out="$WORK/$name"
 cmake -S "$ROOT/runtime" -B "$out" -G Xcode \
  -DLLAMA_SOURCE="$WORK/source" -DCMAKE_SYSTEM_NAME=iOS \
  -DCMAKE_OSX_SYSROOT="$sdk" -DCMAKE_OSX_ARCHITECTURES="$arch" -DCMAKE_OSX_DEPLOYMENT_TARGET=16.0 \
  -DBUILD_SHARED_LIBS=OFF -DGGML_NATIVE=OFF -DGGML_METAL=OFF -DGGML_ACCELERATE=OFF \
  -DGGML_OPENMP=OFF -DGGML_BLAS=OFF -DLLAMA_BUILD_COMMON=OFF -DLLAMA_BUILD_TESTS=OFF \
  -DLLAMA_BUILD_EXAMPLES=OFF -DLLAMA_BUILD_TOOLS=OFF -DLLAMA_BUILD_SERVER=OFF -DLLAMA_OPENSSL=OFF \
  -DCMAKE_XCODE_ATTRIBUTE_CODE_SIGNING_ALLOWED=NO
 cmake --build "$out" --config Release --target hatoriel_local --parallel 3
 local libs=()
 while IFS= read -r -d '' archive; do libs+=("$archive"); done < <(find "$out" -type f -path '*/Release*/*.a' -print0)
 test "${#libs[@]}" -ge 2
 xcrun libtool -static -o "$WORK/$name.a" "${libs[@]}"
}
build_slice device iphoneos arm64
build_slice sim-arm iphonesimulator arm64
build_slice sim-x86 iphonesimulator x86_64
lipo -create "$WORK/sim-arm.a" "$WORK/sim-x86.a" -output "$WORK/simulator.a"
rm -rf build/HatorielLocal.xcframework
xcodebuild -create-xcframework -library "$WORK/device.a" -headers build/runtime-headers -library "$WORK/simulator.a" -headers build/runtime-headers -output build/HatorielLocal.xcframework
