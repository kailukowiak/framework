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
- `sales-with-margin.json` — the same document again after
  `SetFramePipeline` adds a calculated Margin column, so the frame holds a
  literal column beside a chain-calculated one and `editing` reports the
  per-frame answer the grid gates edits on.
- `sales-margin-delete-region.json` — one more `SetFramePipeline` on the
  same store, whose Select step drops Region. Generated from the same
  document so every id matches `sales-with-margin.json`: a test can render
  one view and re-render with the other to stand in for an undo or a chain
  edit arriving from outside the component.
- `imported-sales.json` — a separate document holding a linked import: an
  artifact-paged frame with a file connector, which the engine reports as
  not cell-editable (`editing.cells` false, with the reason text). The
  parquet it points at is `imported-sales.parquet` beside these fixtures,
  written by this generator; the artifact path is stored relative to the
  workspace root, which is where this example sets its working directory.
