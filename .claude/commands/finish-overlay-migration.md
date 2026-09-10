---
description: Fold patches on explicit request, then migrate the opened-CSV path off literal rows
---

Two pieces of work, in this order. Read `AGENTS.md` first — it governs style, and
its rule about comments carrying the *argument* applies to everything here.

## Where things stand

`main` is green: overlay editing works for frames whose rows live in a parquet.
A typed-over cell, a struck-out row and a row added past the end are recorded as
patches against the base read, keyed by the row's ordinal in it. The cost is the
number of corrections, not the size of the table, and the file is left byte for
byte as written. `docs/CSV_EDITING.md` describes the contract.

What is *not* done: an opened small CSV still copies every cell into the
document, so `MAX_EDITABLE_ROWS` (20,000) and the large documents remain for that
path only.

## Task 1 — compaction, on explicit request

Fold accumulated patches into a fresh artifact when somebody asks, and only then.

- Home: `compact_document_data` (`src-tauri/src/lib.rs`). Not ⌘S, and not a new
  button.
- Shape: for each frame where `has_row_patches()`, materialize the data layer,
  write a fresh artifact with `write_frame_artifact` (`data/import.rs`), then
  point the frame at it with `replace_base_artifact`, which clears the patches —
  that is the fold. The old artifact falls to the sweep.
- Order matters: `replace_base_artifact` clears the patches, so materialize
  *before* replacing, or the fold loses exactly what it was folding.
- Test it: after a fold the file contains the corrections, `has_row_patches()` is
  false, and every displayed value is unchanged across the fold.

**Do not re-open the ⌘S question.** Four reasons it must not compact: autosave is
debounced to one write per idle pause, so folding on save rewrites the whole file
on every pause you take; it breaks undo across a save, because a patch is the
only place the prior value exists; it gives the content-addressed artifact a new
id every few seconds, which ends "two people read the same file"; and saving a
workbook means writing down what you did, where folding discards the distinction
between what the file said and what you changed. For a file-backed frame the
user-facing fold already exists — Update original, or Export frame as…

## Task 2 — migrate the opened-CSV path

Goal: an opened CSV is read lazily and edited by patches, so the row cap and the
oversized documents go away.

**This is one atomic change.** Three things that look separable are not:

1. `render_file` (`data/file_writeback.rs`) iterates `frame.rows` and maps them to
   source lines through `origin.row_ids`.
2. `DelimitedFileOrigin.row_ids` is load-bearing, not bloat. Literal rows shift
   relative to the file — delete literal row 1 and position 1 becomes the old row
   2 — so `position + 1` stops naming the source line. UUIDs survive that shift;
   ordinals do not. Write-back therefore cannot move to ordinals while literal
   rows exist, and literal rows cannot go until write-back moves.
3. `prepare_open_delimited` cannot produce an artifact-backed frame, because core
   has no data directory: staging is the desktop's job (`stage_import_file`). So
   `Operation::OpenDelimitedFile { name, path, x, y }` has to gain an artifact or
   be folded into `ImportFrameFromArtifact` with an optional `file_origin`.

Point 3 makes this an operation-surface change, rippling into both enums in
`operation/kinds.rs`, `prepare`/`apply`/`invert`, the desktop caller in
`src-tauri/src/file_writeback.rs`, `describe_operations` (generated from the
`Operation` enum and served over MCP), and the e2e specs that invoke operations
by name.

Suggested order inside the one change:

1. Give `ImportFrameFromArtifact` an optional `file_origin` — path, sha256,
   column ids, **no** row ids — and have the desktop's `import_file_into_session`
   set it for csv/tsv instead of taking the literal path.
2. Rewrite `render_file` to take the collected indexed plan
   (`frame_page_plan(frame, Layer::Data)` carries `ROW_INDEX`) and walk it: the
   ordinal names the source line (`ordinal + 1`), values come from
   `polars_value_at` + `scalar_value_to_raw`, a token is reused only when the
   decoded value still matches, and an ordinal past the end of the file is an
   added row with no token to keep. A draft of exactly this was written and
   reverted in the session that produced this brief — the shape was right, it was
   simply wrong to land on its own.
3. Delete `row_ids`, the literal-row construction in `prepare_open_delimited`,
   and `MAX_EDITABLE_ROWS`. Rewrite `attach_editable_source` to re-point the
   origin rather than remap literal cells. Check whether `BakeFrame` still has a
   caller once write-back stops baking.

Write-back's safety net is the byte-exact tests —
`editable_csv_preserves_identifiers_dates_quotes_and_noop_bytes`,
`renamed_headers_and_manual_edits_write_back_and_undo_can_be_written_again`, and
`sorting_and_filtering_keep_tokens_attached_to_stable_rows`. They have to keep
passing, and they only mean anything once all of the above is in place. That is
why this cannot be half-landed: the failure mode is silently writing a wrong CSV
over somebody's real file.

## Traps already paid for — do not rediscover these

- **One ordinal space.** The page used to mint its own row index *after* patches
  ran, so `source:<frame>:2` meant the third surviving row to the page and the
  third row of the file to the patch that id addressed. `apply_row_patches` now
  mints the single index the page carries out, before anything is removed. Any
  new read path must address rows by that same ordinal.
- **`coalesce` cannot express a cleared cell.** Emptying a cell is a patch *to*
  nothing, indistinguishable from no patch — the cell silently refilled itself
  from the base. Patches carry a presence marker beside the value.
- **A calculated column must be refused in `prepare`.** The old rewrite refused
  it only by accident, failing when it looked for a column the parquet does not
  contain. A patch is recorded happily and then asks the plan to merge over a
  column the base read never produces.
- **`materialize_rows_with_identity` must apply entry columns.** Skipping them
  was safe only while the frame sets did not overlap; `WithColumns` is both
  identity-preserving and `is_computed`.
- **Run the whole gate, not `cargo check -p framework-core`.** `--all-targets`
  builds the examples, and a new `FrameObject` field breaks
  `examples/generate_sample_documents.rs` without it.

## Decisions already made

- Positional patches belong only where nothing refreshes — adopted data, or an
  opened file about to be written back. `EntryColumn`, keyed by the row's own key
  columns, is the answer for live data.
- ⌘S on an opened data file should write back to that file, with a warning once
  on close if the session holds a recipe the file cannot carry. It belongs with
  Task 2. This reverses the "Neither action is bound to ⌘S" line in
  `docs/CSV_EDITING.md`, which was right for the old storage model — record the
  reason rather than quietly deleting it.
- Breaking the document format is fine; report what breaks. All 14 checked-in
  `.fw` files are generated by `crates/framework-core/examples/`.

## Gate — all six, on every commit

```
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace --no-fail-fast
npx tsc --noEmit
npm run lint
npx vitest run
```

Capture output and check the exit code directly; piping into `tail` hides a
failure. `cargo fmt --all` is fine to run, but only your own hunks should show up
in `git diff` — a repo-wide reformat belongs in its own commit.
