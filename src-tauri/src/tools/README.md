# tools

Rhai-based tool system for Cortex.

**TCAT**: Looks up a tool by name from the `tools` SQLite table.

**TOOLRUN**: Executes a Rhai script in a sandboxed engine (100k ops limit, no file I/O or network). Input args are exposed as `args: Array<String>`; the last expression is the return value.

**FORGE**: Asks Claude Haiku to write a Rhai script from a natural-language description. The generated code is validated by SBOX before being stored.

**SBOX**: Validates Rhai syntax via `Engine::compile()` before any tool is saved or run.
