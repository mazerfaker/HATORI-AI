# Hatoriel shared Rust core — v0.3.8

Build library without desktop UI: `cargo check --no-default-features --lib`.
Run core tests: `cargo test --no-default-features --lib`.
Desktop remains the default Cargo feature and executable name remains hatori_ai for compatibility.

## API
- core::Request: model, mode, traits, query, history (including current user message), approved memories, profile, optional simulated mood, style and optional embedding model.
- core::prepare: constructs the system prompt and bounded chronological chat context.
- core::run: blocking generation via an injected inference::Backend; emits Token, Done, Cancelled or Error. Run on a worker thread; return false from event callback to stop receiving output. Cancellation is cooperative and cannot interrupt a blocked HTTP read.
- inference::Backend: synchronous streaming generation and model enumeration. Current implementations are local Ollama and llama.cpp HTTP servers.
- Shared modules: personality, memory_core, memory_suggest, semantic, adaptive, emotion, style, proactive.

## Platform responsibilities
Windows UI, system tray, autostart, filesystem, notifications, process management and SQLite persistence remain in desktop code. The caller supplies approved memories and history. No automatic transfer to another device. Default endpoints are loopback; no mobile remote transport, Android JNI, iOS FFI, synchronization or native mobile inference has been implemented yet.

## Validation
Mock backend tests exercise events, early cancellation and Unicode context bounds. Existing Windows workflow additionally checks and tests the no-default-features library. These commands do not constitute Android/iOS cross-compilation; mobile builds require separate toolchains and integration.
