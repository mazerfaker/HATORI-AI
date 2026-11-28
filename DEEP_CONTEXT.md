# v0.8.6 Deep Context

Windows: use a manually selected archive excerpt in the editable next-message draft; no automatic transfer. Explicit local-model summary worker (Ollama or llama-server on loopback), bounded user statements and output, cancellation and manual approval. Model drafts may contain errors; review before saving. Existing extractive summary remains.

Shared: recognize explicit preference corrections as new suggestions, never overwrite old records automatically. Android/iOS expose extractive summary and approval through shared Rust JSON bridge. No model-generated summary on mobile yet. Desktop optional semantic embedding batch retains both recent and high-importance memories (64 maximum). Existing semantic search remains opt-in. No automatic cross-chat retrieval, semantic staleness classifier, or schema changes.

Tests cover summary JSON, local model draft/cancellation, correction classification, and old high-priority embedding candidate. CI and actual device runtime validation separate.
