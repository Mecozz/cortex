# vault

Encrypted credential storage for Cortex.

Secrets are encrypted with AES-256-GCM using a per-installation key stored at
`{app_data}/vault.key`. The key file never leaves the device. Vault entries are
stored in the `vault` SQLite table as `base64(nonce + ciphertext)`.

**VAULT → INJECT**: `chat_message` checks the vault for `api_key_anthropic` if
the settings field is empty, so keys can be stored securely instead of in plaintext settings.
