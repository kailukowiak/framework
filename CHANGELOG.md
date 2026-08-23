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
