# tutor-ui (Rust TUI bridge)

Rust owns the terminal loop only. All teaching logic stays in `tutor/` Python
(stdlib). The TUI shells to `python -m tutor.cli` from the workspace root.

## Build / run (needs Rust toolchain)

Member of the cargo workspace at the repo root. Run from the root:

```powershell
cargo run -p tutor-ui -- quiz --domain math --n 5
cargo run -p tutor-ui            # interactive menu
```

Prefer the existing GUI when available: `repos/open-webui-plugin/` plugs the
same tutor core into Open-WebUI via `python -m tutor.cli serve`.
