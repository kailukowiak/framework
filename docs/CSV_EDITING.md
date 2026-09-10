# Editing a CSV and writing out a result

FrameWork should handle “open this CSV, fix it, put it back” without asking
someone to build a reusable data pipeline. A stored CSV/TSV is read where it
lies and typed into by recording corrections against that read, so an edit
costs the edit rather than a copy of the table. Linked imports remain
refreshable inputs to a model.

Every frame exposes **Read** at the start of Wrangle. It names the input;
file and entered-data frames can replace it with a file or database query.
Column names can be pasted as a tab-separated header row or one name per line,
committed together on blur and undone together. Matching physical source names
retain column identities and display renames when the input is replaced.

Every frame offers **Export frame as…**, which asks for a new output path and
writes CSV, TSV, Parquet or NDJSON according to the extension chosen there. The chooser
offers the table's own source format first, because the format somebody already
has is the likeliest one they want more of. An existing file, the frame's own
input, and any other extension are refused.

A file-backed frame additionally offers **Update original …**, which confirms
the original path and replaces that file. It writes the format that file
already is — the path it replaces chooses the writer — so there is no way to
turn a CSV into a Parquet by overwriting it, and the menu names the format it
is about to replace. Neither action is bound to ⌘S or changes workbook
autosave, and this stays true now that an opened file is read rather than
copied. ⌘S saves the workbook — the recipe, the layout, the corrections still
recorded as corrections. Replacing somebody's file is a different act with a
different blast radius: it is not undoable, it discards the distinction
between what the file said and what they changed, and a debounced autosave
would perform it on every idle pause. It stays a thing you ask for.

Exporting saves the entire data result: manual cell edits, column names,
insertions and deletions, and Wrangle transformations, including calculated
columns, filters and sorts. Header filters and sorts are data transformations
and are included. Formatting and legacy display steps from older notebooks are
excluded.

After success, the new file is added to the canvas as a separate imported table
with no transformations, read the same way every other file is: from a staged
copy beside the workbook, at whatever size it is. The working table is
unchanged — it keeps its input and Wrangle chain. Saving the notebook therefore keeps the
recipe for reuse; discarding the notebook leaves only the values-only output
file. This split makes both halves visible instead of hiding a disconnected
recipe inside the result.

Updating the original is the in-place alternative. It atomically replaces the
source, does not keep a recovery copy, and adds the replacement to the canvas
as a new read of that file with no transformations. The original working
table and its active transformation chain stay intact; saving the workbook
keeps that recipe for reuse. Closing without saving discards the workbook-side
recipe and newly added frame, but it does not reverse the external overwrite.
Undo changes the workbook but does not silently rewrite the external file,
which changes only on another explicit update.

Errors keep the working draft. Undo removes the newly imported table from the
workbook; it does not delete the external file.

For work that should run again on next week's file, save the `.fw` model and
replace Read when the new input arrives. Export never overwrites that input;
Update original does so only after confirmation.

## Fidelity

A delimited file is read without date guessing, whitespace trimming,
NA-to-null substitution, or identifier conversion. A column is interpreted as
numeric only when every nonempty value is an unambiguous finite number, with
no leading-zero integer spelling and no more than 15 digits. Ambiguous dates,
long identifiers and mixed columns stay text. A cell's original spelling
survives a write-back rather than being stored: the token in the file is kept
whenever it still spells the value the frame holds, so `12.00` stays `12.00`
until somebody changes that cell. Explicit calculated values use the engine's
result.

The output is a fresh representation of the computed data layer: escaped
standard delimited text, Parquet carrying the data layer's own Polars types, or
NDJSON carrying one self-describing JSON object per line.
Text identifiers such as `0003` remain text in both, and a whole number is an
integer. Numeric values otherwise use the engine's canonical spelling, so an
imported `5.00` may be written as `5` where its cell has changed; where it has
not, the original token is kept, so an untouched file comes back byte for byte.
Its clean import deliberately has no formulas: the formulas remain on the
working table, and the output table contains their current values.

## Deliberate boundaries of this implementation

- Write-back is for UTF-8 CSV/TSV with a header, nonempty unique header names
  and regular records. Unsupported encoding, blank records or irregular quoting
  are reported when a write is prepared, before any byte of the file is
  replaced; they are never silently repaired. Opening such a file still works —
  it reads through the same conservative parse every import uses — so what is
  refused is putting a result back over records FrameWork cannot reproduce
  faithfully, which is the moment the answer actually matters.
- Only a stored CSV or TSV gets a write-back destination. A linked import keeps
  its connector instead: refreshing over somebody's corrections and writing
  their corrections back out are opposite promises, and a table may only make
  one of them. Replacing the input of a table that had a destination re-points
  the destination at the new file and drops the connector; a replacement that
  is not delimited text simply leaves it without one.
- Typing is numbers or text. Dates, booleans and currency are not guessed.
  Inference inspects all records in a bounded-memory pass before streaming the
  typed artifact, so an identifier occurring only on the last line stays text.
  Explicit type conversions remain available through column editing or Wrangle.
  Delimiter/encoding overrides and an editable per-source parse schema are not
  yet exposed in Read.
- There is no row cap and no separate editable copy. A file of any size is read
  where it lies, and a correction to it is a note against the base read rather
  than a second copy of the table inside the document — the same mechanism a
  Parquet-backed table already used. The 20,000-row ceiling existed because an
  opened file used to be copied into the document as literal cells, so every
  autosave serialized the whole table; reading it in place removed the reason
  for the limit rather than raising it.
- Preparing a write holds the source file in memory to keep its tokens, so
  putting a result back costs about the size of the file. Opening one, editing
  it and paging through it do not.
- A source path that has become a symbolic link is refused: a rename would
  replace the link, not the file behind it.
- NDJSON is read and written as one flat JSON object per line, which is the
  shape an object-store dump arrives in. Its types are JSON's own, so a quoted
  identifier stays text without any of the delimited path's inference rules,
  and `.jsonl` is accepted as the same format. The schema is decided after
  reading every line rather than a leading sample, so a field that is null for
  a thousand records and a string on the last one still types correctly.
  A nested object or array is refused at import, naming the field: a grid cell
  holds one value, and the alternative was a table that imported cleanly and
  then failed to render. Flattening it — or extracting fields from a JSON text
  column — is work somebody can see, so it stays their decision. NDJSON is an
  export and import format only; it has no in-place update, for the same reason
  Parquet does not.
- A value typed over a frame whose rows live in a file — a parquet, or the
  staged copy of an opened CSV, which are now the same thing — is recorded as a
  patch against that file rather than by rewriting it: the row's ordinal in the
  base read, the column, and the text. Striking out a row and adding one past
  the end are the same kind of note, in the same ordinal space, so an added row
  is one whose every value is a patch. The file stays byte for byte as written,
  which is both what makes an edit cost the edit and what lets a
  content-addressed artifact be the thing two people read. Patches are dropped
  when the base is replaced — they name rows by position, and position does not
  survive a new file. An identity-preserving chain may stand between the grid
  and the file; a reshaping one may not, because then there is no row to name.
  That same ordinal is what write-back addresses a source line by: line
  `ordinal + 1`, since line zero is the header. Nothing per-row is stored to
  say so.
- **Settle and reclaim data files** in the Project panel folds those patches
  into a fresh file when somebody asks: each table carrying corrections is
  written a new base that already contains them, and pointing it there is what
  drops them. Only the base is folded — the Wrangle chain runs over the new
  file exactly as it ran over the old, because folding a filter in would delete
  the rows it was only hiding and folding a calculation would leave a column
  that no longer recomputes. Every displayed value survives the fold unchanged,
  and undo puts the notes back. It is deliberately not on save, for the reasons
  ⌘S is not bound to write-back either. One consequence worth knowing: folding
  moves rows, so afterwards an ordinal no longer names the line it was read
  from, and a later *Update original* respells every cell in the engine's own
  spelling instead of keeping the file's. The values are unaffected — a token
  is only ever reused when it still spells the value the table holds.
- Parquet is an export destination, not an update destination. A new file has
  no source schema to preserve, so writing one needs no contract beyond the
  data layer's own types. Replacing an existing Parquet in place does: the
  document's eight column types cannot express decimal, timestamp precision and
  time zone, integer width, nested columns, or a file's own metadata and sort
  statistics, so an in-place rewrite would quietly restate all of it. The
  delimited path escapes this only because it keeps each cell's original token
  and reuses it when the value has not changed; a typed format has no token to
  fall back on. Until a recorded source schema exists, `separator()` refuses
  write-back for anything but CSV and TSV, and `file_origin` — the record an
  update needs — is only ever set for those.
- Saving a workbook is the durable way to keep the reusable transformation
  recipe; an external CSV itself stores only data.

## Acceptance walkthrough

Use three isolated files in FrameWork Dev, with real UI edits and filesystem
readback:

1. Rename a contact column and correct a cell in a CSV containing leading-zero
   IDs, long digit strings, ambiguous dates and quoted commas.
2. Rename an inventory column, change a quantity and delete a junk row; update
   the original and check the file, retained transformation chain, and absence
   of a recovery copy.
3. Add only a calculated column to a numeric CSV, write out, verify that the
   imported output contains values and no transformations, and that the
   original table still has its calculation. Replace the original table's
   source and verify that its calculation runs once on the new values.

Also test cancellation, refusal to overwrite an existing output file, Undo
leaving external files in place, and a failed update leaving the pipeline intact.
