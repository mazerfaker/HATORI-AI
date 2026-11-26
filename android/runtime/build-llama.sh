#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-${ANDROID_HOME}/ndk/27.2.12479018}}"
WORK="${RUNNER_TEMP:-/tmp}/hatoriel-llama-b10488"
git clone --depth 1 --branch b10488 https://github.com/ggml-org/llama.cpp.git "$WORK/source"
cmake -S "$WORK/source" -B "$WORK/build" -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE="$NDK/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-26 \
  -DANDROID_STL=c++_static -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF -DGGML_NATIVE=OFF -DGGML_OPENMP=OFF \
  -DGGML_CPU_ARM_ARCH=armv8-a -DLLAMA_OPENSSL=OFF \
  -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF -DLLAMA_BUILD_SERVER=ON
cmake --build "$WORK/build" --target llama-server --parallel 2
OUT="$ROOT/android/runtime/out"
mkdir -p "$OUT"
cp "$WORK/build/bin/llama-server" "$OUT/llama-server"
cp "$WORK/source/LICENSE" "$OUT/llama.cpp-LICENSE"
file "$OUT/llama-server"
