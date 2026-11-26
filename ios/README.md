# Hatoriel iOS Foundation 0.5.0

Native SwiftUI diagnostic application for iOS 16+, adopting approved Crimson Core Minimal colors and procedural red reticle. Respects Reduce Motion and pauses animation outside active scene. Uses the existing headless Rust core, not a rewrite. The Rust static library exports a bounded UTF-8 JSON history API and explicit string ownership/free functions. Swift prepares the context off the main thread and displays the result locally. No inference, server connections, memory persistence, microphone, notifications, attachments, sync or local GGUF runtime in this version. The diagnostic output is prepared context, not an AI-generated response.

## Build
Requires macOS with Xcode, Rust/rustup, XcodeGen and iOS SDKs.
From repository root:
```sh
bash ios/build-core.sh
cd ios
xcodegen generate
open Hatoriel.xcodeproj
```
XCFramework includes arm64 iPhone and arm64/x86_64 simulator libraries. The GitHub macOS workflow runs shared Rust and FFI unit tests and builds unsigned simulator/device .app archives. A physical iPhone requires valid Apple signing and provisioning; the unsigned archive is not an installable IPA. No signing keys are stored in the repository.

## Versioning
Root core/desktop and iOS wrapper: 0.5.0. Android application and JNI wrapper remain 0.4.9 to avoid presenting the iOS foundation as an Android release. Existing Windows/Android functionality remains in place.

## Verification
Three Rust FFI test methods cover Unicode, invalid history and allocation/free ownership. CI compilation, simulator rendering and physical-device installation have not been verified. QA_049.md remains the acceptance checklist for Windows/Android. Next: native iOS chat UI and transport, then richer shared-core integration.
