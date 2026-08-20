# Changelog

What changed, written for the person using FrameWork rather than the person
who wrote it. The release workflow lifts the section matching the tag it is
building into the GitHub release and into the in-app update offer, so an
entry here is the only place release notes come from — see
`.github/workflows/release.yml` and `src/lib/updates.ts`.

Every version has a section, newest first, headed `## <version>` with no `v`.
Work lands under `## Unreleased`; cutting a release renames that heading to
the version being tagged and opens a fresh `## Unreleased` above it.

## Unreleased

- Plots chart the whole frame. A plot on a file-backed frame was quietly
  charting only the first 1,000 rows, so every aggregate was computed over a
  slice. Charts now load every row; a chart that aggregates a very large
  frame in the browser gets an inline note pointing at the faster Summarize
  step, and a frame past 200,000 rows waits behind a Render click instead of
  drawing itself on open.

- Safe mode: hold ⌥ while opening a document to load it with nothing
  evaluated and no data read. A document that hangs or crashes on open can
  be reached, repaired, and saved; a banner offers "Turn on evaluation" to
  recompute in place. Every open action in the Data Library supports it, and
  a hint under the file picker says so.

- A Wrangle formula the engine or the parser refuses now says so in the
  formula bar itself, next to the formula, and keeps you editing. Before,
  the error only appeared inside the Wrangle panel — with the panel closed,
  Return on a bad formula ended the edit silently and the formula looked
  saved when it wasn't.

- Typing a new calculated column's formula in the bar works again. Adding a
  calculated column focuses its formula in the bar; pressing Return there
  used to do nothing — the column kept its blank placeholder with no error —
  because the save that created it quietly disconnected the formula from its
  editor. Return now applies the formula, including after the Wrangle panel
  was closed mid-edit.

- Plots render again in installed builds. Charts now evaluate through Vega's
  sandboxed interpreter instead of eval, which the app's security policy
  blocks — development builds never enforced that policy, so plots worked at
  the desk while every packaged build showed only an error.

- Formula editing sessions now end. Return applies the formula and finishes
  the session, Escape cancels back to the saved formula, and clicking
  elsewhere deactivates instead of silently inserting references into a
  formula you already committed. The bar shows when a session is active and
  returns to the selected cell afterwards.

- Undo and Redo in the Edit menu enable reliably after every edit, including
  Wrangle chain edits on imported frames — previously they could stay greyed
  out, leaving ⌘Z dead. Opening another document no longer carries the
  previous document's undo state with it. Menu commands also no longer
  disappear when no window reports focus at the instant the menu closes —
  an enabled Undo now always reaches the document.

- Cells that cannot take a typed value no longer open an editor or keep
  showing text that was never saved. The refusal appears at the frame with
  the reason, and a literal column stays typeable beside a calculated one.
  Rearranging or deleting columns no longer freezes a hand-entered table's
  remaining cells.

- The grid and the Wrangle step list follow the document immediately:
  deleting a column updates the grid at once, and undoing a step removes it
  from the list instead of leaving it to be silently re-saved.

- New cards land in free space instead of exactly on top of existing cards —
  imports, the Add rail, keyboard shortcuts, and the Scratchwork drawer all
  place beside what is already there.

- Escape closes the Data Library, Join, New Document, and Excel import and
  export dialogs, matching every other dialog.

- The Selection panel no longer calls every imported frame an "immutable,
  paged snapshot": it now states the engine's actual verdict, so a snapshot
  the document owns is described as typeable and a linked import explains
  what a refresh will do.

- Build live calculation matrices from zipped row and column vectors, with
  evenly repeating shorter vectors, a shared formula body, and a long-form
  result that updates whenever its inputs change. The body formula can expand
  vertically for longer calculations.

- Format text either as `format("Q{}", value)` or the chain-friendly
  `"Q{}".format(value)` spelling.

- Vector drags now follow the pointer with a compact preview, and the full
  right edge accepts a drop. Dropping onto a one-column vector table asks
  whether to place the next vector beside it with HStack or below it with
  VStack instead of opening an incompatible Apply Vector step.

- Search every available formula from the Help menu or with ⇧⌘P, including
  Excel names and everyday terms such as range, iterator, vector, list and
  array. Each result explains its arguments, output and null behavior and can
  copy its syntax or insert it at the active formula cursor. ⇧⌘H opens the same
  modeless browser over practical guides and the rules that distinguish
  Variables, Scratchwork, Wrangle, live data, ownership, row order and value
  shapes. Exact references cover every Wrangle transformation, Join behavior,
  direct and conditional formatting, number formats, crosstab display, and
  frame summaries.
- Add a compact standalone variable directly to the canvas: one name, one
  formula field shared with the top formula bar, and one live scalar or vector
  answer. Formulas accept numbers, text, booleans, vectors, and explicit quoted
  dates such as `"2024-10-10"d`. Its name sizes to its content, its window stays
  visible and resizable, and multiline autoformatted formulas scroll when the
  window is smaller than their contents.
- Filling a number or date series no longer locks the rest of its table or
  hides the empty add-row line. The filled column remains formula-controlled.
- Named collections are now called vectors consistently in tutorials and
  controls. Their values wrap across Container cards, and their floating editor
  keeps the canvas visible and interactive instead of blurring it.
- Rendered Markdown cards now allow ordinary text selection, and long notes
  keep their reading position when switching into or out of edit mode.
- Select one or more columns in a lookup table and drag them onto a key
  header in another table to bring just those columns over. FrameWork suggests
  the matching lookup key and offers the full Join dialog only when needed.
- Join previews and Wrangle now report matches, missing rows, blank keys, and
  duplicate keys from the complete dataset, including live and paged tables,
  rather than from the rows currently rendered on the canvas.
- A joined table’s key relationship is now visible and editable as the first
  compact step in Wrangle.
- Vector footers now drag reliably on macOS using FrameWork's in-app pointer
  gesture, including drops on empty canvas and table headers or edges.
- Named Scratchwork answers and Container values or results now use that same
  drag gesture. Scalars travel as one-value vectors; vector-valued lines keep
  their live length.
- Vectors can now be dragged onto table headers to apply them across columns,
  with repeating patterns shown as one compact Wrangle step instead of one
  formula per column.
- Drag a vector onto empty canvas to make a live one-column table, then drop a
  second vector on the table’s right edge to pair it as another column. Lengths
  must match, so row-by-row pairing never happens silently.
- Drag a header onto a header in another table to open Join with both key
  columns already selected.
- A new final tutorial teaches two-vector table creation, row-count-aware date
  series, and visual key joins, with live checkpoints when an upstream row is
  added.
- Every practical tutorial now opens with its complete rendered walkthrough in
  both the Start workbook and Answer key. Markdown code examples remain literal
  instead of being mistaken for live formula holes.

## 0.1.4

- Opening FrameWork on Windows no longer leaves a terminal window sitting
  behind it.
- Refreshing a command connection on Windows no longer flashes a console
  window while the command runs.
- Release notes in the update dialog render as markdown — headings, lists,
  tables, and links, wrapped to the dialog — instead of a block of raw text.
- The update offer no longer repeats the download and first-launch
  instructions. You are already running FrameWork and one click from an
  automatic install; it shows what changed and nothing else.

## 0.1.3

- Linux desktops recognise FrameWork and its `.fw` documents: the app appears
  in the applications menu with its icon, and double-clicking a document opens
  it.

## 0.1.2

- FrameWork offers its own updates. When a newer release exists it says so
  once, and Install and restart downloads, verifies the signature, and
  replaces the running copy in place. Skip stops it asking about that version;
  Check for Updates in the menu asks whenever you want an answer.
- Copies installed through `apt`, `dnf`, or a software centre say so instead
  of failing: those update the way everything else on the system does.

## 0.1.1

- macOS no longer reports the app as damaged and refusing to open. The bundle
  is signed, so the first launch is the ordinary unidentified-developer
  warning that **Open Anyway** clears, not a dead end.
- The download link on the site points at the current release rather than a
  fixed version.

## 0.1.0

First public build: the canvas, frames, formulas, and documents, packaged as
a `.dmg`, `.exe`/`.msi`, `.deb`, `.rpm`, and `.AppImage`.
