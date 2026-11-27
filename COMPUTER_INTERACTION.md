# Hatoriel v0.7.0 — Computer Interaction Foundation

Base: v0.6.9 commit 6651cd7b97b391d5ee8b9493df076deb0266315d. Windows CI #359, Android #247 and iOS #125 all successful; runtime tests were not executed.

## Local action lifecycle
Computer page (Windows): disabled by default every launch; user opts in for current app session, chooses a typed action, reviews exact scope, confirms once within 60 seconds. Proposal cancelled on leaving page, loss of window focus, busy chat, revocation, or expiry. No reusable approval. No action is invoked by model output, remote bridge or mobile. No new network endpoints or executable command input.
- Inspect local app status: current configured backend/model, portable/installed mode and bridge running state. Does not query model or send contents anywhere.
- Open app data folder in Windows Explorer: only canonical application data directory; no arbitrary path or file opening; rejects UNC/device-prefix paths, rechecks target before launch, fixed Explorer executable, direct argument, no shell. Launch result means process creation requested, not verified Explorer window opened. Drive-letter mapped locations and local OS redirection are not a security sandbox.
- SQLite audit: approval record inserted and bounded to 200 before execution; missing/failed journal blocks action. Outcome updated after attempt; incomplete records may remain after crash or DB failure. Stores timestamp/action/result only, no file contents, paths, secrets or conversation text. Shows newest 30. Not tamper-proof. No logging of rejected or expired proposals yet.

Four desktop regression tests cover session consent, one-shot approval, expiry/revocation, target validation, journal retention and failed journal writes. No new mobile features. CI/runtime testing pending.

## v0.7 roadmap
- 0.7.1 File System Access: user-selected read-only roots, scope validation, deny escapes.
- 0.7.2 File Search & Navigation: bounded indexed/searchable metadata in allowed roots.
- 0.7.3 File Transfer Engine: explicit original file selection and transfer consent, no implicit uploads.
- 0.7.4 Application Control: allowlisted app launches, reviewed arguments, no arbitrary shell.
- 0.7.5 Screen Awareness: explicit capture, review, no hidden monitoring.
- 0.7.6 Clipboard Intelligence: explicit read/write and preview.
- 0.7.7 Desktop Automation: reviewed typed steps, stop, bounded execution.
- 0.7.8 Permission & Action Manager: centralized scope, expiry, revocation, auditing.
- 0.7.9 Stability: all platform CI, failure analysis, artifact verification. Runtime/device tests deferred to release QA.

Unfinished v0.6 items remain: mobile multimodal transport, bundled Piper/pdftotext, OCR, local speech compatibility and real device testing. Do not claim full automation or system access yet.

## v0.7.1 File System Access (Windows preview)
Session-only read-only grants, up to 8 specific folders selected via native picker. Grant, immediate directory listing and text preview each require explicit one-shot confirmation and pre-execution audit. Revoke clears visible results, pending requests and scoped access. Disable all permissions clears all scopes. No drive-root grants, UNC/device prefixes, relative traversal, alternate data streams, symlinks or Windows reparse-point components. Canonical paths checked before and after reads; regular files only. Up to 200 immediate entries scanned per listing, 64 KiB plain-text preview (UTF-8/BOM UTF-16), no recursion, file mutations, upload, automatic AI access or mobile control. Text is displayed locally, not sent to inference or stored in audit. PDF/DOCX remain supported via existing attachment workflow only. Path checks mitigate common escapes but are not a kernel sandbox; concurrent filesystem mutation/TOCTOU, mapped drive letters and hard links are not fully isolated. File access is synchronous and may stall on slow folders. Three desktop tests and one Unix-only symlink test added; physical-device testing not performed. Previous v0.7.0 CI running when work began; new CI pending.
