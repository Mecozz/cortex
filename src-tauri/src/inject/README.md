# inject

Context assembler. Builds the `CompletionRequest` that gets sent to the active provider.

## Phase 1 (current)

- Takes conversation history + optional system prompt.
- Prunes history to the last 20 messages (HISTSUM stub).
- No memory retrieval yet — that arrives in Phase 2.

## Phase 2+

INJECT will read from PASS1/PASS2 retrieval and inject relevant facts, episodic context,
and open tasks into the prompt before each send.
