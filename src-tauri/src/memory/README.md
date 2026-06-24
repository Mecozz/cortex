# memory

Long-term memory pipeline for Cortex.

**INSTCAP** — extracts facts from conversation turns via Claude Haiku.  
**CONF** — stores facts with contradiction invalidation (category-scoped `is_current` flag).  
**PASS1** — fast retrieval of current facts for prompt injection.

Facts are injected into the system prompt by `inject::assemble()` before each completion call.
