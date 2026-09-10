# Editing a CSV and writing out a result

FrameWork should handle “open this CSV, fix it, put it back” without asking
someone to build a reusable data pipeline. Stored CSV/TSV imports are editable
working copies. Linked imports remain refreshable inputs to a model.

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
autosave.

Exporting saves the entire data result: manual cell edits, column names,
insertions and deletions, and Wrangle transformations, including calculated
columns, filters and sorts. Header filters and sorts are data transformations
and are included. Formatting and legacy display steps from older notebooks are
excluded.

After success, the new file is added to the canvas as a separate imported table
with no transformations — literal values when the output qualifies as an
editable copy, and an ordinary paged read otherwise, which is what a Parquet
output or a large result will be. The working table is unchanged: it
keeps its input and Wrangle chain. Saving the notebook therefore keeps the
recipe for reuse; discarding the notebook leaves only the values-only output
file. This split makes both halves visible instead of hiding a disconnected
recipe inside the result.

Updating the original is the in-place alternative. It atomically replaces the
source, does not keep a recovery copy, and adds the replacement to the canvas
as a new direct CSV/TSV read with no transformations. The original working
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

The editable path reads cell text without date guessing, whitespace trimming,
NA-to-null substitution, or identifier conversion. A column is interpreted as
numeric only when every nonempty value is an unambiguous finite number, with
no leading-zero integer spelling and no more than 15 digits. Ambiguous dates,
long identifiers and mixed columns stay text. Raw cell spellings survive
numeric interpretation. Explicit calculated values use the engine's result.

The output is a fresh representation of the computed data layer: escaped
standard delimited text, Parquet carrying the data layer's own Polars types, or
NDJSON carrying one self-describing JSON object per line.
Text identifiers such as `0003` remain text in both. Numeric values use the
engine's canonical spelling, so an imported `5` may be written as `5.0` — and
in Parquet that spelling is a schema fact rather than a rendering, because an
editable copy types every numeric column as a double while the paged path reads
whole numbers as integers. The same file therefore exports a double schema when
it is small enough to edit and an integer schema when it is not. Its clean
import deliberately has no formulas: the formulas remain on the working table,
and the output table contains their current values.

## Deliberate boundaries of this implementation

- The editable path is for UTF-8 CSV/TSV with a header, nonempty unique header
  names and regular records. Unsupported encoding, blank records or irregular
  quoting are reported before importing; they are never silently repaired.
- This changes stored CSV/TSV import only for files that qualify as an
  editable copy. Linked imports, pasted data and Parquet retain their existing
  import behavior. A comprehensive recorded parse-rule system for all imports
  remains separate work.
- Typing is numbers or text on both editable and paged CSV/TSV paths. Dates,
  booleans and currency are not guessed. Paged inference inspects all records
  in a bounded-memory pass before streaming the typed artifact, so identifiers
  after the initial sample remain text too. Explicit type conversions remain
  available through column editing or Wrangle. Delimiter/encoding overrides
  and an editable per-source parse schema are not yet exposed in Read.
- The working copy uses document-owned literal rows, capped at
  `MAX_EDITABLE_ROWS` (20,000). Every autosave serializes those rows and every
  operation rebuilds a view over them, which is why the cap is far below the
  import limit. A larger file, a non-UTF-8 file, or one with blank or
  irregular records imports the ordinary source-backed paged way. Its rows cannot be
  typed into directly, while column names, calculated columns, filters, sorts,
  and the rest of Wrangle remain editable. This distinction is explained only
  if a direct cell edit is attempted; opening the file itself needs no warning.
  The import never fails for those reasons. Large-file paging and compact
  literal storage need further work.
- A source path that has become a symbolic link is refused: a rename would
  replace the link, not the file behind it.
- `BakeFrame` is only accepted for a table with a file origin, and refuses a
  result above the editable cap; taking ownership of any other table is
  *Adopt data*.
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
- A value typed over a frame whose rows live in a parquet is recorded as a
  patch against that file rather than by rewriting it: the row's ordinal in the
  base read, the column, and the text. Striking out a row and adding one past
  the end are the same kind of note, in the same ordinal space, so an added row
  is one whose every value is a patch. The file stays byte for byte as written,
  which is both what makes an edit cost the edit and what lets a
  content-addressed artifact be the thing two people read. Patches are dropped
  when the base is replaced — they name rows by position, and position does not
  survive a new file. An identity-preserving chain may stand between the grid
  and the file; a reshaping one may not, because then there is no row to name.
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
