# v0.8.4 — Long-Term Relationships

Shared Rust context changes for Windows, Android and iOS.

- Explicit project, interest and communication preference statements can produce memory suggestions through the existing approval flow. No automatic storage or profile rewriting.
- Recognized short follow-ups use the latest substantive user turn (within the last 32 messages) for retrieval. New topics keep their own query. Assistant messages never seed retrieval.
- Only existing approved memory records are searched; the mobile context opt-in and endpoint restrictions remain unchanged. Desktop optional semantic retrieval uses the same query.
- Hatoriel should maintain a consistent personality while distinguishing remembered facts from inference and respecting corrections. This is prompt guidance, not a guarantee of model behavior.
- Previous chat transcripts are not globally searched. Cross-chat continuity relies on explicitly approved records. Existing storage, deletion, sync and profile formats are unchanged.
- Tests cover follow-up retrieval, absent history, topic changes, mobile parity and suggestion classification. CI and device runtime validation are separate.
