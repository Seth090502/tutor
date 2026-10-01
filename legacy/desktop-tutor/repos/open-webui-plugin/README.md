# Open-WebUI plugin (existing GUI — prefer over building a new one)

Open-WebUI already ships chat + retrieval UI, so V1.1 plugs the tutor in as
tools instead of rebuilding a GUI.

1. Start the tutor API (the single contract):
   `python -m tutor.cli serve --port 8797`
2. In Open-WebUI: Settings -> Tools -> Add OpenAPI spec ->
   paste `repos/open-webui-plugin/openapi.json` (server 127.0.0.1:8797).
3. Exposes `search`, `quiz` (answers withheld), `progress`, `attempt`.
   Grading/feedback stays in tutor core: `attempt` takes the chosen letter and
   returns correct/explanation/misconception graded server-side.

Use the Rust TUI (`ui-rust/`) only for a local terminal UI; use Open-WebUI
for the full chat GUI.
