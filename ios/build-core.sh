#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p build/headers build/simulator
sed '/#include "Hatoriel-Local-Header.h"/d' Hatoriel/Hatoriel-Bridging-Header.h > build/headers/HatorielCore.h
export IPHONEOS_DEPLOYMENT_TARGET=16.0
export CARGO_TARGET_DIR="$PWD/build/rust-target"
for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  rustup target add "$target"
  cargo build --manifest-path native/Cargo.toml --release --target "$target"
done
lipo -create "$CARGO_TARGET_DIR/aarch64-apple-ios-sim/release/libhatoriel_ios.a" "$CARGO_TARGET_DIR/x86_64-apple-ios/release/libhatoriel_ios.a" -output build/simulator/libhatoriel_ios.a
rm -rf build/HatorielCore.xcframework
xcodebuild -create-xcframework \
  -library "$CARGO_TARGET_DIR/aarch64-apple-ios/release/libhatoriel_ios.a" -headers build/headers \
  -library build/simulator/libhatoriel_ios.a -headers build/headers \
  -output build/HatorielCore.xcframework
