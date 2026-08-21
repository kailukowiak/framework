# UI fixtures

Generated `DocumentView` JSON — the exact struct the frontend receives over
Tauri IPC, camelCase fields and all. Never hand-edit these files; regenerate
them with:

    cargo run -p framework-core --example generate_ui_fixtures

Every id in these files (document, frame, column, block, view) is a UUID or
a `column_id` random suffix minted fresh each time the example runs, so ids
are not stable across regeneration. Tests must select fixture data by name
— a frame's `name`, a block line's `name` — never by id.

- `blank.json` — `Document::blank("Fixture")`, a brand-new workbook with
  nothing on the canvas.
- `sales-before-formula.json` — a "Monthly sales" frame plus an empty
  "Checks" block, before any formula has been written.
- `sales-with-formula.json` — the same document after `SetBlockSource`
  writes a formula into Checks, so `computedBlocks` carries a real computed
  answer rather than an empty line.
