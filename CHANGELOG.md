# Changelog

What changed, written for the person using FrameWork rather than the person who wrote
it. The release workflow lifts the section matching the tag it is building into the
GitHub release and into the in-app update offer, so an entry here is the only place
release notes come from — see `.github/workflows/release.yml` and `src/lib/updates.ts`.

Every version has a section, newest first, headed `## <version>` with no `v`. Work lands
under `## Unreleased`; cutting a release renames that heading to the version being
tagged and opens a fresh `## Unreleased` above it.

## Unreleased

- Solve periodic and dated investment returns with `finance.irr` and `finance.xirr`, or cash-flow dot calls. Invalid inputs and unsuccessful searches report an inline error; an optional guess selects among discovered roots. The Price a deal lesson now solves and checks its return.
- Financial functions are available under `finance`, including dot calls such as `principal.finance.pmt(rate, term)` and `flows.finance.xnpv(rate, dates)`, with autocomplete and argument hints. Existing unqualified formulas still work.
- Calculate loan payments, present and future values, payment periods, interest and principal, and periodic or dated net present value with `pmt`, `pv`, `fv`, `nper`, `ipmt`, `ppmt`, `npv` and `xnpv`. The Price a deal lesson now uses these functions.
- Calculate down rows retains fractional results when the starting value is an integer, so loan balances and compounding forecasts no longer lose cents at every step.

- Three finance lessons join the tutorial library: **Price a deal**,
  **Driver-based forecast**, and **Scenarios, sensitivity and goal seek**.
  Each builds the job with today's FrameWork, says where that is harder than
  it should be, and ends with what the same job becomes once the finance work
  lands. Each has a Start workbook and an Answer key.
- Fit models by choosing a frame and target; all other columns become predictors. Use a narrower derived frame to choose fewer inputs.
- Undo and Redo from the native menu now restore grid edits when the grid has keyboard focus.
- Keep the app responsive while checking recent files that may be waiting on
  cloud storage or file access.

- Learn robust OLS on the diabetes dataset and XGBoost on Iris with two new
  tutorials, each with a Start workbook and a completed Answer key.

- Train XGBoost models from a frame to predict numbers or categories, with held-out
  evaluation and saved fits powering live prediction frames.

- Fit linear, binary logistic, and random-forest models on the canvas, inspect
  coefficient tables and evaluation results, and use their predictions in
  ordinary frames. Predictions follow changes to their inputs; retraining is
  explicit, with stale fits marked and previous fits preserved if retraining fails.
- Import supported XGBoost JSON models and fitted sklearn ONNX pipelines for
  local inference without Python, preserving supported fitted preprocessing.
- Create mean confidence intervals, correlations, and Welch comparisons as
  ordinary result frames that can be referenced in formulas.
  Model operations and statistical summaries are also available through MCP.

## 0.1.9

- Opening a CSV or TSV no longer has a size limit, and no longer has two
  behaviours. Every file — twenty rows or two million — opens the same way:
  read where it lies, editable in place, and writable back to with *Update
  original*. Files over 20,000 rows used to open read-only, because a smaller
  one was copied into the workbook cell by cell and a large one could not be.
  Nothing is copied now, so the limit had nothing left to protect: workbooks
  that open a file are dramatically smaller, they save in a fraction of the
  time, and a correction to a large table costs the same as a correction to a
  small one. A whole number read from a file is now a whole number on the way
  out, where a small file used to write `5` back as `5.0`.

- **Settle and reclaim data files** in the Project panel now writes the
  corrections you have made to a table's data file into a file of their own,
  before it clears out the staged files nothing points at any more. Values on
  the canvas do not change — a correction just stops being a note beside the
  file and becomes the file. It happens only when you ask for it, never on
  save: saving records what you did, and folding corrections in would throw
  away the difference between what the file said and what you changed. Undo
  puts the notes back.

- Correcting a value in a table whose rows live in a data file no longer
  rewrites that file. A typed-over cell, a struck-out row and a row added at
  the end are recorded as small notes against the file instead, so an edit costs
  the edit rather than a full read and rewrite of everything around it — and the
  file is left exactly as it was. Tables that have a filter or a sort in Wrangle
  can now be typed into as well, and a correction lands on the row it was made
  on rather than the position that row happened to be at. Notes are dropped if
  the underlying file is replaced, since they name rows by position.

- Documents are smaller: a cell with no formula of its own no longer writes one
  out as an empty field, which was roughly a quarter of a document holding
  literal rows.

- Open a CSV or TSV, fix it, and put the result back, without building a
  reusable pipeline first. A stored CSV/TSV opens as an editable table: type
  into cells, rename columns, insert and delete rows, and add calculated
  columns, filters and sorts. The table's right-click menu then
  offers **Update original CSV…**, which confirms the path once and then
  atomically replaces the source file, without leaving a recovery copy. It
  always writes the format that file already is. Either way the result joins the
  canvas as a clean table with no transformations, while the table you worked in
  keeps its input and its Wrangle chain — so saving the workbook keeps the
  recipe for next week's file. Neither action is bound to ⌘S, and neither
  changes workbook autosave.

- **Export frame as…** writes any table to a new file as CSV, TSV, Parquet or
  NDJSON, chosen in the save dialog, and saves the whole data result: manual
  edits, renames, calculated columns, filters and sorts alike. The chooser
  offers the table's own source format first, so a table read from Parquet
  suggests Parquet. Parquet carries the computed types rather than respelling
  everything as text, which makes it the right destination for a result too
  large or too typed for CSV. An existing file and the table's own input are
  both refused — replacing a file in place is what Update original is for.

- NDJSON joins CSV, TSV and Parquet as a format FrameWork both reads and
  writes, including from Finder and the command line, which is how a dump out
  of an object store usually arrives. One flat JSON object per line, `.jsonl`
  accepted as the same thing, and the types are JSON's own — a quoted
  identifier stays text with no inference rules involved. The schema is decided
  after reading every line rather than a leading sample, so a field that is
  empty for a thousand records and filled on the last one still types
  correctly. A file with nested objects is refused with the field named, rather
  than imported into a table that cannot be displayed.

- Opening a data file is now a way to start work. Pick FrameWork from **Open
  With** for a CSV, TSV, Parquet or JSON-lines file, or name one on the command
  line, and it opens into an unsaved workbook — including when FrameWork was not
  already running, and without leaving an empty Untitled window beside it.
  Opening the same file again raises the window already showing it instead of
  starting a second, competing draft of the same file. Installing FrameWork does
  not change what a double-click on a CSV opens: it adds itself next to whatever
  you already use, on Windows as well as macOS, and updating does not quietly
  claim those files back. A `.fw` document is FrameWork's own format and still
  opens on a double-click.

- Wrangle now starts with **Read**, which names the input and makes it
  changeable. Replace a file with another file or with a database query; choose
  how each source column is read — text, integer, number, boolean or date, or
  back to whatever the source says — before any transformation runs; and paste
  a whole row of column headers, tab-separated or one name per line, to rename
  every column in a single undoable edit. Columns whose source names still
  match keep their identities and their display names when the input is
  replaced.

- Imported values keep their own spelling. A leading-zero identifier such as
  `0003`, a long digit string and an ambiguous date all stay text instead of
  being guessed into a number or a date — on the paged import path as well as
  the editable one, and including values that first appear long after the
  opening rows. A column is read as numeric only when every value in the file
  is an unambiguous finite number. Explicit conversions stay where you can see
  them, in Read or in Wrangle.

- A linked import — one you asked to keep a connection to — is still read-only
  in its cells, because the next refresh replaces them. The boundary is
  explained only if you try to type into one, and the explanation names the
  file the rows are read from and says what a refresh will do to them. Column
  names, calculated columns, filters, sorts and the rest of Wrangle work on it
  normally.

- **Map values** and **Rename columns** both create or choose a mapping frame
  from the column menu. A mapping frame is an ordinary two-column table: blank
  keys match missing values, blank replacements clear them, and you choose
  whether unmatched values stay unchanged, go blank, or take custom text.
  Mapping a header row renames once, as one undoable edit, and leaves unmatched
  names alone.

- Header filters now reach downstream calculations and exports, the same way a
  Wrangle filter does. Use a separate frame when you want an independently
  filtered result.

- Formula editing keeps its own ⌘Z and ⇧⌘Z history until the formula is
  committed, so undoing inside a formula no longer reaches past it into the
  document. Typing a decimal point no longer opens function suggestions.

- The inspector starts collapsed, stays collapsed while you edit a column from
  the top bar, and overlays the canvas instead of squeezing it: Fit to window
  uses the full canvas width, so closing the inspector reveals the rest of a
  fitted table without a second fit. The formula bar stays one compact row
  until you expand it.

## 0.1.8

- Formula examples in text cards preserve code formatting, backticks, and line breaks.

- A new Dictionaries and value mapping tutorial includes an editable Start
  workbook, an answer key, and an in-workbook walkthrough.

- Dictionaries are compact Key/Value tables with unique keys. Create one from
  the canvas menu or use an existing two-column table as a dictionary, then
  choose Map values on a column to create a live Wrangle transformation.
  Scratchwork also supports lookup with an optional missing-key fallback.
- Dragging a literal column's fill handle now fills the indicated rows, with
  number and date series inferred from the selected values. Other selections
  repeat their values; one Undo restores the whole drag. Calculated columns
  no longer show a fill handle that merely opened an editor.
- Charts over paged tables reload after relevant formulas, source refreshes,
  and undo change their data. Loading and failed reads no longer leave old
  rows presented as the current answer.
- Pasting a rectangle that overruns the columns or touches calculated cells
  is refused as a whole, with an explanation, instead of silently skipping data.
- XLSX export lets you choose the current filtered/sorted rows or the entire
  table, with exact row counts for the chosen scope.
- A failed pending save now prevents window close or application quit, keeping
  the document available for retry or Save As.
- A refused cell edit offers making the rows editable directly. Text and
  category cells also offer an exact-match replacement in Wrangle, explicitly
  applying to all matching rows and future refreshes.
- Pointing at a cell while writing a formula keeps the keyboard in the
  formula: the cell is no longer selected as well, so the next keystrokes no
  longer land in the grid as a rejected value. A reference clicked into a new
  calculated column drops into the formula instead of replacing the column's
  name, and calculated columns can be pointed at by their cells as well as
  their headers.
- A calculation can now read a column written beside it in the same "Add or
  replace columns" step. Clicking that column inserts the reference like any
  other, and Return moves the calculation into a step of its own directly
  below, where the column it reads already exists — Wrangle shows the two
  numbered steps. Anything else in the step that reads the moved calculation
  travels with it. A name that also arrives from above still means the
  column from above, so replacing a column while another formula in the same
  step reads it is unchanged.
- A formula started from a cell of an existing column now remembers which
  cell it started from, so pointing at the row above writes `.shift(1)` the
  way it does for a brand-new calculated column, instead of a plain
  reference to the whole column.
- The formula bar no longer changes height when a formula opens in it, so
  the canvas stays put between choosing "Formula here" and clicking the cell
  you meant. A click aimed at the first row used to land a row too high once
  the bar had grown, quietly costing the reference its `.shift(…)`.
- Calculated columns are easier to write: the backticks on the name are
  optional (`Profit = `Revenue` - `Cost`` works, and the Wrangle line reads
  it back as `` `Profit` `` once saved), both ways of adding a column open the
  same fully selected line so the first keystroke replaces it, and a formula
  becomes a step only on Return. Half-typed text is no longer swept
  into the document by a later save, and Wrangle no longer reports an error
  about a name still being typed. "Add calculated column" from a header adds
  the column at the end of the table; "Insert column here" remains the
  positional way.
- Escape closes the Reference panel from anywhere inside it.
- A cell value the engine refuses no longer lingers as a draft: the refusal is
  shown once and the cell goes back to showing what it holds.
- New cards land where you are looking. ⌘J, the ADD rail, imports and pastes
  place a card in the closest gap that needs the least scrolling, and the
  canvas moves only that far, so the table you were about to click is still
  on screen. Scratchwork cards start wide enough for a fifty-character line.
- Opening a document shows the document: the canvas starts at the top-left of
  its contents, nothing is selected, and no editor is left running.
- Frame cards fit their rows, up to a dozen, instead of showing five of six
  behind a scrollbar; a paste that adds rows grows the card unless you sized it
  yourself, and adding a calculated column scrolls the grid to it.
- The status bar's Count, Sum and Average include calculated columns; a
  Wrangle-produced column used to count as nothing at all. Selecting several
  columns and applying a number format keeps saying how many columns it is
  formatting, and the formula bar names a cell by the row it sits in after a
  sort.
- A value card under an active scenario shows the scenario after its number
  ("1.15 · Upside").
- Dragging a column header shows a label of what it carries, and the header
  under the pointer lights up when dropping would match two tables. In the
  lookup dialog, marking a key unique confirms "Month is unique" in place
  instead of making the buttons jump.
- A tab made with + → Table view is called "Sales vs budget view" rather than
  "copy", and its pass-through columns no longer wear a formula badge.
- Wrangle's + Group starts on the selected column, or the first column that
  names things, with the name selected so it reads as a choice.
- A new bar chart draws its categories in the table's own row order rather
  than resorting them by height.
- Return commits a frame, container or plot rename and leaves the field.
- The inspector's labels and helper text are no longer selectable, so a
  resize drag that overshoots into it stays a resize.
- The left rail's "Data" button is now "Sources", matching the panel it opens;
  "Library" remains the Data Library. Recent documents that share a name show
  their folder ("The FrameWork tour › Start"). Resetting the tutorials reports
  the result inline. The splash suggests checking for a macOS Documents
  permission prompt if opening takes more than a few seconds, and the initial
  window fits smaller screens. The column right-click menu is grouped with
  separators.
- Tutorial walkthrough cards show relative links as their text and relative
  images as their caption instead of raw markdown. The Grand Tour and
  Month-over-month tutorials were corrected to match the app.

## 0.1.7

- Hold ⌘ and FrameWork shows you the keys. Every button that has a keyboard
  shortcut, from the rail and the inspector tabs to a card's fit and collapse
  controls, the zoom readout and the Scratchwork toggle, wears its shortcut in
  a small badge for as long as the key is down, and nothing moves when they
  appear. Let go, press anything else, or switch away and they are gone. The
  Scratchwork window does the same.

- The FrameWork mark at the top of the left rail opens Quick Commands, so the
  command palette is reachable without knowing ⇧⌘P or opening the menu.

- Cards arrive at the size of what is in them. A paste into an empty frame, a
  paste onto bare canvas, an import and a join each open a card wide enough for
  their columns and tall enough for about a dozen rows, and a new Scratchwork
  block starts wide enough for a line and its answer. A join lands in free
  space in the viewport, comes up selected, and is scrolled into view; jumping
  to a card now only scrolls when the card is off screen, and then centres it.

- Selection follows the gesture. Pressing a column header selects that column
  rather than widening back out to the whole card, pressing a member of a
  container selects the member, a container has a name field in the inspector,
  and Quick Commands closes when you press outside it.

- Committing a column formula hands the keyboard back to the grid, so the next
  arrow key moves the selection instead of moving the caret. The Format tab now
  says which scope each block of controls changes, and a number format applied
  over a grid selection reaches every numeric column in the range rather than
  one of them.

- Turning the wheel over a grid stops at the grid's edge instead of continuing
  into the canvas underneath.

- Variable (⌥⌘V), Calculation Matrix (⌥⌘M) and Canvas Only (⇧⌘C) now have
  shortcuts, in the menu, in Quick Commands and on the rail badges. Holding
  Shift or Option together with ⌘ narrows the badges to the shortcuts that use
  that modifier.

- Fixes in the Scratchwork window: the pop-out no longer blanks with a React
  error when the caret sits inside a backtick, ⌘J activates the session it
  raises, and pointing at canvas values from the pop-out works on the first
  click into a window instead of needing a click to activate it first. A
  container card's add buttons no longer stretch to full width, so the values
  you add are visible at the card's default size.

- A new first tutorial, **The FrameWork tour**, opens the lesson list. In about
  25 minutes it goes once over what FrameWork does that a spreadsheet does not:
  paste a table that arrives typed, write one formula for a whole column,
  declare the row order and see it in Wrangle, keep Scratchwork in its own
  window, bring a budget over by dragging its header onto a key, branch a tab
  and summarize it, switch the same model between Base, Upside and Downside
  from the corner of the canvas, and chart the result beside the table. Every
  section names the lesson that teaches it properly, so the tour is a map
  rather than a replacement. **Create tutorials** now makes twelve workbooks.

- Edits now reach the `.fw` file as one write after about two seconds of idle
  time instead of a write per operation, which is what was producing
  conflicted copies in Dropbox and Google Drive during active editing. A
  pending write is still flushed at once before Save As, Open, New, Package or
  Compact, and on window blur, window close, or quitting, so nothing is left
  unsaved.

- Quick Commands (⇧⌘P) now indexes more than the menu. Select a frame, a card
  or a column and the palette offers that object's own actions under its name:
  sort, pin, hide or delete a column, create a frame from a frame, rename, fit,
  collapse, delete, running exactly what the right-click menu runs. A Recent
  group lists the last eight documents and opens one on Enter. Shortcuts are
  searchable as text, so "⌘3", "cmd 3" and "shift cmd p" all find the command
  that holds the key. A query that names no command no longer dead-ends: the
  last row hands the phrase to Find, which opens already searching for it.

- The application's menu commands now have one definition. Every item in the
  menu bar, every row in Quick Commands, and everything the Scratchwork window
  hands back to its workbook are checked against that one list, so a command
  can no longer appear in the menu while doing nothing, the way Undo once could
  in the Scratchwork window. Commands the Scratchwork window forwards to its
  workbook now arrive there in every build, not only in the installed app.

- The Keyboard Shortcuts dialog now lists the grid navigation that already
  worked but was undocumented: ⌘Arrow jumps to the edge of the data, ⇧⌘Arrow
  extends the selection there, ⇧Space selects the row, and Home, End, ⌘Home
  and ⌘End reach the ends of a row or of the frame.

- Scratchwork can open in its own ordinary workbook window from the Window
  menu or Quick Commands. It stays on the same autosaved block and undo
  history as the canvas, ⌘J raises it when it is already open, and pointing at
  canvas values or columns still inserts references into its active line.

- ⌘⇧P now opens Quick Commands, a dense searchable list of document, canvas,
  view and help actions—including Open, Save As, Scratchwork and inspector
  commands. ⌘K opens the unified Formula and FrameWork Reference, choosing
  formulas automatically while a formula editor is active; the former ⌘⇧H
  help shortcut is no longer needed.

- An update offered as FrameWork opens now appears in front of the Recent
  documents and Projects library. The library returns after the update prompt
  is dismissed instead of covering it or closing alongside it.

## 0.1.6

- Wide frames keep their key columns in view. Right-click a column header and choose
  _Pin columns through here_ to freeze everything up to that column: the frozen columns
  and the row numbers stay put while the rest of the grid scrolls sideways, with a
  divider where the frozen part ends. _Unpin columns_ on any frozen column releases
  them. The setting is saved with the workbook and undoes in one step.

- ⌘F finds things. Type into the Find palette and FrameWork searches the whole workbook
  at once: card and column names, formulas, Scratchwork lines, and the values in your
  frames, including linked files too large to hold in memory. Pick a result with the
  arrow keys or the mouse and it takes you there: the card comes forward and the cell is
  selected.

- Values can hold more than one number. Give a value a second reading under a name
  (Base, Upside, Downside) and switch the whole workbook between them from the corner of
  the canvas: every result, calculated column and Scratchwork line that reads the value
  moves with it, and switching back puts them where they were. A value's scenarios are
  edited as one small table in the inspector, where a blank cell means "same as the
  base", and a new scenario can start as a copy of one you already wrote. A value card
  under a scenario shows the number in force and which scenario it came from.

- A selected column carries a small square at the bottom-right corner of the selection,
  the way a spreadsheet does. Dragging it down does not copy cells: it opens that
  column's formula, so the gesture that fills a range in Excel writes the rule for the
  whole column here. The rows you drag across are outlined while you hold it, and
  letting go anywhere but below the selection does nothing. ⌘D and ⌘R still fill values
  literally.

- Date columns have display formats of their own. Pick `2026-09-01`, `1 Sep 2026`,
  `Sep 1, 2026`, `Sep 2026` or `Q3 2026` in the column's Format section and the grid,
  crosstab views and the Stats drawer all show it; editing still takes `YYYY-MM-DD`.

- Excel export can include a sheet called _How it was computed_: one row per exported
  column saying whether it is a source field, a formula or the output of a Wrangle step
  and which values it reads, then each frame's steps in order and every named value's
  formula. It is on by default; a checkbox in the export dialog leaves it out. The
  numbers stay values, as before.

- Refreshing a linked frame no longer breaks it when the source changes shape. A field
  that disappears from the data empties only its own column, which says so where it is,
  while every other column and every frame downstream that does not read it goes on
  calculating. The refresh also reports what changed in the frame's Source panel: fields
  that arrived, fields that went, a column kept without data because a formula still
  reads it, and a column whose type moved.

- A tutorial or recent document that is on disk but blocked from opening by macOS stays
  in the Data library, greyed out and labelled "can't be read", instead of failing when
  clicked. When a document does fail to open, the message names the folder FrameWork
  could not read and the System Settings panel that fixes it, and it sits at the top of
  the Data library rather than below the list.

- FrameWork keeps a log of what it is doing: documents opened and saved, and anything
  that goes wrong along the way, including commands that fail behind the interface. It
  appears in the terminal running `npm run tauri dev` (set `RUST_LOG=debug` for more) or
  in Console.app for the bundled app. If the interface hits an error it cannot recover
  from, a plain message and a Reload button replace the blank window.

- A typed column now refuses text it cannot hold. Typing `abc` into an integer column,
  pasting it there, or adding a row with it is turned back with a message naming the
  column and the type it wanted, instead of being stored and shown as a blank.

- The inspector collapses into a thin bar at the right edge so it stays easy to find
  without taking table space: use its `i` button, ⌘⇧I, View → Inspector, or the hide
  button in its header. Asking for a section again (⌘1, ⌘2, ⌘3, Formula here, a Wrangle
  prompt) brings it back.

- `=` in a cell is now a doorway to the column's formula rather than something typed
  into the cell. Press it on any cell, a selected column, or the new-row line and the
  column's formula opens in Wrangle, seeded with the column's name, with the whole
  column visibly the subject. On a calculated column it opens the existing formula. The
  formula bar no longer labels cells with spreadsheet coordinates.

- The Stats drawer's columns now line up with the grid's. Its statistics were sized to
  their own numbers and drifted away from the columns they describe.

- Pasting with nothing selected drops the clipboard onto the canvas as a new frame. Tab-
  or comma-separated text arrives with its own headers and column types, the way a paste
  into an empty frame already did, and one undo removes the whole frame again.

- Native menu commands now act only on the workbook they were invoked from. With several
  workbooks open, shortcuts such as ⌘O no longer raise the Data Library and its loading
  state in every window at once.

- Cut, copy, and paste shortcuts work on grid selections again. Native Edit menu
  commands now reach the selected cells while text fields keep their ordinary system
  clipboard behavior. Right-click Copy now uses the cell or range the context menu was
  opened on instead of a stale earlier selection.

- The vectors, dates, and visual joins tutorial answer key now evaluates its live
  row-count check instead of showing an unknown-name formula error.

## 0.1.5

- Plots chart the whole frame. A plot on a file-backed frame was quietly charting only
  the first 1,000 rows, so every aggregate was computed over a slice. Charts now load
  every row; a chart that aggregates a very large frame in the browser gets an inline
  note pointing at the faster Summarize step, and a frame past 200,000 rows waits behind
  a Render click instead of drawing itself on open.

- Safe mode: hold ⌥ while opening a document to load it with nothing evaluated and no
  data read. A document that hangs or crashes on open can be reached, repaired, and
  saved; a banner offers "Turn on evaluation" to recompute in place. Every open action
  in the Data Library supports it, and a hint under the file picker says so.

- A Wrangle formula the engine or the parser refuses now says so in the formula bar
  itself, next to the formula, and keeps you editing. Before, the error only appeared
  inside the Wrangle panel — with the panel closed, Return on a bad formula ended the
  edit silently and the formula looked saved when it wasn't.

- Typing a new calculated column's formula in the bar works again. Adding a calculated
  column focuses its formula in the bar; pressing Return there used to do nothing — the
  column kept its blank placeholder with no error — because the save that created it
  quietly disconnected the formula from its editor. Return now applies the formula,
  including after the Wrangle panel was closed mid-edit.

- Plots render again in installed builds. Charts now evaluate through Vega's sandboxed
  interpreter instead of eval, which the app's security policy blocks — development
  builds never enforced that policy, so plots worked at the desk while every packaged
  build showed only an error.

- Formula editing sessions now end. Return applies the formula and finishes the session,
  Escape cancels back to the saved formula, and clicking elsewhere deactivates instead
  of silently inserting references into a formula you already committed. The bar shows
  when a session is active and returns to the selected cell afterwards.

- Undo and Redo in the Edit menu enable reliably after every edit, including Wrangle
  chain edits on imported frames — previously they could stay greyed out, leaving ⌘Z
  dead. Opening another document no longer carries the previous document's undo state
  with it. Menu commands also no longer disappear when no window reports focus at the
  instant the menu closes — an enabled Undo now always reaches the document.

- Cells that cannot take a typed value no longer open an editor or keep showing text
  that was never saved. The refusal appears at the frame with the reason, and a literal
  column stays typeable beside a calculated one. Rearranging or deleting columns no
  longer freezes a hand-entered table's remaining cells.

- The grid and the Wrangle step list follow the document immediately: deleting a column
  updates the grid at once, and undoing a step removes it from the list instead of
  leaving it to be silently re-saved.

- New cards land in free space instead of exactly on top of existing cards — imports,
  the Add rail, keyboard shortcuts, and the Scratchwork drawer all place beside what is
  already there.

- Escape closes the Data Library, Join, New Document, and Excel import and export
  dialogs, matching every other dialog.

- The Selection panel no longer calls every imported frame an "immutable, paged
  snapshot": it now states the engine's actual verdict, so a snapshot the document owns
  is described as typeable and a linked import explains what a refresh will do.

- Build live calculation matrices from zipped row and column vectors, with evenly
  repeating shorter vectors, a shared formula body, and a long-form result that updates
  whenever its inputs change. The body formula can expand vertically for longer
  calculations.

- Format text either as `format("Q{}", value)` or the chain-friendly
  `"Q{}".format(value)` spelling.

- Vector drags now follow the pointer with a compact preview, and the full right edge
  accepts a drop. Dropping onto a one-column vector table asks whether to place the next
  vector beside it with HStack or below it with VStack instead of opening an
  incompatible Apply Vector step.

- Search every available formula from the Help menu or with ⇧⌘P, including Excel names
  and everyday terms such as range, iterator, vector, list and array. Each result
  explains its arguments, output and null behavior and can copy its syntax or insert it
  at the active formula cursor. ⇧⌘H opens the same modeless browser over practical
  guides and the rules that distinguish Variables, Scratchwork, Wrangle, live data,
  ownership, row order and value shapes. Exact references cover every Wrangle
  transformation, Join behavior, direct and conditional formatting, number formats,
  crosstab display, and frame summaries.
- Add a compact standalone variable directly to the canvas: one name, one formula field
  shared with the top formula bar, and one live scalar or vector answer. Formulas accept
  numbers, text, booleans, vectors, and explicit quoted dates such as `"2024-10-10"d`.
  Its name sizes to its content, its window stays visible and resizable, and multiline
  autoformatted formulas scroll when the window is smaller than their contents.
- Filling a number or date series no longer locks the rest of its table or hides the
  empty add-row line. The filled column remains formula-controlled.
- Named collections are now called vectors consistently in tutorials and controls. Their
  values wrap across Container cards, and their floating editor keeps the canvas visible
  and interactive instead of blurring it.
- Rendered Markdown cards now allow ordinary text selection, and long notes keep their
  reading position when switching into or out of edit mode.
- Select one or more columns in a lookup table and drag them onto a key header in
  another table to bring just those columns over. FrameWork suggests the matching lookup
  key and offers the full Join dialog only when needed.
- Join previews and Wrangle now report matches, missing rows, blank keys, and duplicate
  keys from the complete dataset, including live and paged tables, rather than from the
  rows currently rendered on the canvas.
- A joined table’s key relationship is now visible and editable as the first compact
  step in Wrangle.
- Vector footers now drag reliably on macOS using FrameWork's in-app pointer gesture,
  including drops on empty canvas and table headers or edges.
- Named Scratchwork answers and Container values or results now use that same drag
  gesture. Scalars travel as one-value vectors; vector-valued lines keep their live
  length.
- Vectors can now be dragged onto table headers to apply them across columns, with
  repeating patterns shown as one compact Wrangle step instead of one formula per
  column.
- Drag a vector onto empty canvas to make a live one-column table, then drop a second
  vector on the table’s right edge to pair it as another column. Lengths must match, so
  row-by-row pairing never happens silently.
- Drag a header onto a header in another table to open Join with both key columns
  already selected.
- A new final tutorial teaches two-vector table creation, row-count-aware date series,
  and visual key joins, with live checkpoints when an upstream row is added.
- Every practical tutorial now opens with its complete rendered walkthrough in both the
  Start workbook and Answer key. Markdown code examples remain literal instead of being
  mistaken for live formula holes.

## 0.1.4

- Opening FrameWork on Windows no longer leaves a terminal window sitting behind it.
- Refreshing a command connection on Windows no longer flashes a console window while
  the command runs.
- Release notes in the update dialog render as markdown — headings, lists, tables, and
  links, wrapped to the dialog — instead of a block of raw text.
- The update offer no longer repeats the download and first-launch instructions. You are
  already running FrameWork and one click from an automatic install; it shows what
  changed and nothing else.

## 0.1.3

- Linux desktops recognise FrameWork and its `.fw` documents: the app appears in the
  applications menu with its icon, and double-clicking a document opens it.

## 0.1.2

- FrameWork offers its own updates. When a newer release exists it says so once, and
  Install and restart downloads, verifies the signature, and replaces the running copy
  in place. Skip stops it asking about that version; Check for Updates in the menu asks
  whenever you want an answer.
- Copies installed through `apt`, `dnf`, or a software centre say so instead of failing:
  those update the way everything else on the system does.

## 0.1.1

- macOS no longer reports the app as damaged and refusing to open. The bundle is signed,
  so the first launch is the ordinary unidentified-developer warning that **Open
  Anyway** clears, not a dead end.
- The download link on the site points at the current release rather than a fixed
  version.

## 0.1.0

First public build: the canvas, frames, formulas, and documents, packaged as a `.dmg`,
`.exe`/`.msi`, `.deb`, `.rpm`, and `.AppImage`.
