# v0.8.5 Intelligent Memory

Shared: exact normalized duplicate suppression during ranked recall; optional desktop semantic results are deduplicated too. Conflicting facts are not automatically removed or treated as true. Personality prompt encourages clarification.

Windows: explicit local archive search (last 2,000 messages, first 16,000 characters each, up to 20 hits, case-insensitive Unicode lowercase matching). Results open the chat and are never automatically injected into inference. Manual extractive summary suggestion combines up to four recognized user statements from the last 64 messages, with explicit approval in Memories. Similarity warnings compare word overlap and are not semantic contradiction detection; records remain unchanged until user action.

Android/iOS inherit shared recall and prompt changes. Archive search, summary UI and similar-record review UI are Windows-only in this release. No automatic model-generated summaries or full semantic conflict detection yet. Existing memory schema unchanged.

Unit tests cover duplicate suppression, related candidates, extractive notes, Unicode archive search and deletion. CI and actual device testing remain separate.
