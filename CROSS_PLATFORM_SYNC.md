# Hatoriel 0.5.7 — Cross-Platform Sync Preview

Manual encrypted snapshot transfer, not continuous synchronization. Windows Devices, Android Memory → Protected transfer, and iOS Settings → Protected package expose the same protocol. Enter a strong passphrase (minimum 12 characters), export the encrypted text, move it to another device using a trusted method, enter the same passphrase, preview and explicitly confirm the merge. Transfer the passphrase separately. Clipboard use is user-controlled; avoid shared clipboard services if inappropriate.

## Format
Version 1 snapshot: approved memories (content/category/importance), user profile, personality mode, seven traits, dynamic style and emotion simulation setting. Current mood is reset. No chat history, model files, endpoints, access keys, reminders or remote sharing permissions. Local IDs and dates are not portable; new records get new local IDs. Up to 100 records, 500 Unicode scalars per record, 1000 per profile field. Legacy desktop records exceeding limits cause rejection rather than silent truncation.

Base64 envelope: ASCII HATORIEL-SYNC-1, 16-byte random salt, 12-byte random nonce, AES-256-GCM ciphertext and authentication tag. Argon2id v19, 32 MiB, 3 iterations, parallelism 1, 32-byte key. Magic is authenticated associated data. Malformed, altered or wrong-passphrase packages are rejected. No cloud dependency, network transfer or password recovery. A strong unique passphrase remains necessary against offline guessing. Encryption covers the package only, not existing local databases. Password entry is transient; Rust derived key and serialized plaintext buffers use Zeroizing, but platform strings and clipboard copies cannot be guaranteed erased.

## Merge
Normalize memory text for duplicate comparison. Existing local text/metadata wins on conflicts; genuinely new records are appended. Identical entries skipped. No deletion propagation; edited text may be imported as a new record. Profile/personality replacements separately opt in. Preview is checked against local snapshot and package before application. Windows updates SQLite in one transaction; Android writes preferences in one commit; iOS writes memory vault atomically and personality settings separately (not a single cross-store transaction). Mobile imports disable remote memory sharing, requiring renewed consent. Previously transmitted chat history remains separate. Android consumes imported personality settings through shared context; no dedicated personality editor or persistent simulated mood yet.

## Verification
- Shared Rust tests: roundtrip with Unicode, wrong password, tamper rejection, duplicates, local-wins conflict.
- Pending: Windows/Android/iOS compilation, actual cross-platform transfers in all directions, large packages, duplicate normalization, stale preview, permission reset, corrupt/locked storage, interruption and performance.
- No verified CI result or signed iOS binary. This is source-level implementation. Automated real-time synchronization, per-device identities, revision vectors, tombstones, conflict editing, encrypted network transport and full backup/restore are future work.
- All platform versions aligned to 0.5.7; Android versionCode 57. Approved Minimal visual identity unchanged.
