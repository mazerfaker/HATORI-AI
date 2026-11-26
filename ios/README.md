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

## 0.5.1 Native SwiftUI Interface
The diagnostic screen is replaced with a native chat view, animated idle/generation reticle, copy controls, settings sheet, streaming draft, stop, retry and confirmed history clearing. Requires a user-confirmed HTTPS Ollama-compatible server with trusted certificate; no default external server, no plain HTTP, no redirects, no bearer pairing yet. Ephemeral URLSession, bounded NDJSON and terminal-event checks, 300-second resource timeout, request cancellation; client cancellation does not guarantee remote compute stops. Recent context is prepared by Rust before transmission. History is stored in UserDefaults (100 entries / 4 MiB maximum; not separately encrypted and may be included in device backups). Failed/cancelled retries preserve the old answer. Going to background cancels the current request; interrupted partial text is not saved. No local GGUF inference, separate chats, Markdown rendering, memory sync or voice yet. Root/desktop and iOS wrapper 0.5.1; Android remains 0.4.9. CI, native UI rendering and actual server connectivity remain unverified. Existing approved Minimal design retained.
