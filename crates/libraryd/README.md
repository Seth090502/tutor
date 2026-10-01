# libraryd

One `Arc<dyn vault_api::VaultApi>` served identically over MCP (streamable HTTP, `rmcp`) and gRPC (`tonic`).
Spec: `docs/phase1/SPEC.md` section 11. Wire contract: `proto/vault.proto`. Client: `crates/vault-client`.

## Run

```
libraryd --config <libraryd.toml> --init-token     # once: writes [server].token_file (64 hex), never overwrites
libraryd --config <libraryd.toml> --mock           # fixture data, until vault-core is wired in
```

Config lookup: `--config`, `VAULT_CONFIG`, `%ProgramData%\vault\libraryd.toml`, `%APPDATA%\vault\libraryd.toml`.
Only `stage`, `machine`, `[vault].root` and `[server]` are read here (`src/config.rs`); everything else is ignored.

| Endpoint | Address (default) | Auth |
|---|---|---|
| MCP | `http://127.0.0.1:7341/mcp` | `Authorization: Bearer <token>`, `Host` in `allowed_hosts` |
| Health | `http://127.0.0.1:7341/healthz` | none |
| gRPC | `127.0.0.1:7342` | `authorization: Bearer <token>` metadata |

Claude Code: `claude mcp add --scope user --transport http libraryd http://127.0.0.1:7341/mcp --header "Authorization: Bearer <token>"`

## Behaviour worth knowing

- **Same data both ways.** Both interfaces call the same trait method. MCP returns the response as `structuredContent`
  and as text; gRPC returns the typed message. `tests/parity.rs` checks every tool.
- **MCP text.** Passages are wrapped `<vault_passage address=".." page=".." id="xxxx">..</vault_passage id="xxxx">`
  with a fresh random id per block (`page` is omitted when unknown, attribute values are escaped, and an id never
  occurs in its own block's text).
- **MCP size cap.** A response whose text plus structured JSON exceeds 20,000 tokens (4 characters per token) is
  shortened by dropping trailing items; the response's own `truncated` field (and the text) says so. gRPC is not capped.
- **Tool list.** Names are `vault_api::TOOL_NAMES`. Descriptions and input schemas are snapshot-tested
  (`tests/snapshots/tools.json`); regenerate on purpose with `UPDATE_SNAPSHOT=1 cargo test -p libraryd --test tool_snapshot`.
- **Errors.** MCP: a tool failure is an `isError` result with `{"kind","message"}` as structured content; only an
  unknown tool is a protocol error. gRPC: `InvalidArgument`, `NotFound`, `Unimplemented` (not yet available),
  `Unavailable`, `Internal`, which `vault-client` maps back to the same `ApiError`.
- **Deadlines.** Each call has a 60 s server-side deadline on both interfaces (`Unavailable` when exceeded).
- **Streaming.** gRPC `ReadStream`, `OutlineStream`, `GrepStream`, `FindDocsStream`, `SearchStream` and
  `DefineStream` split a batch into one item per chunk, run up to 8 at once, and stream the chunk responses in request
  order with item indexes rebased to the original request. `max_tokens` and `max_hits` apply per chunk.
- **Shutdown.** Ctrl+C stops accepting connections. gRPC calls in flight get up to 10 s to finish, then are ended with
  `Unavailable`. MCP sessions are closed at once: an MCP call in flight when the server stops is cut off and the client
  sees the stream close (rmcp ends sessions as soon as it is told to stop).
- **Logs** go to stderr; the token and presented tokens are never logged.

## Wiring the real vault

`src/main.rs` has one marked place (`WIRE-UP POINT`) where the real `vault_core::Vault` replaces the mock. Nothing else
changes.

## Tests

`cargo test -p libraryd` starts the server on ephemeral ports and drives it with the `rmcp` client and `vault-client`.
Latency (informational): `cargo test -p libraryd --release --test latency -- --ignored --nocapture`.
