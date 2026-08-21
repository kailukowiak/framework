# Structured Canvas Spreadsheet — Design Specification

This document has three jobs. Sections 1–33 define the durable product model
and constraints. Section 34 holds the current replacement threshold followed by
dated design notes; those notes preserve the reasoning that produced the
implementation and are not, by themselves, claims about current state. Section
35 is the decision log. For the exact implemented surface, use `README.md` and
`docs/architecture.md`; an item here is complete only when it says **landed** or
those implementation references show it.

## 1. Product concept

The application is a spreadsheet-like environment built around **structured objects rather than an infinite grid**.

It should preserve the qualities that make Excel and Numbers useful:

- extremely fast ad-hoc data entry;
- formulas typed directly by users;
- calculations that can be created incrementally;
- multiple frames and calculations visible together;
- free-form layout;
- lightweight scratch calculations when necessary;
- summaries, totals, grouping, and analytical transformations;
- low ceremony.

At the same time, it should avoid making physical cell coordinates the fundamental data model.

The central principle is:

> **Everything is an object, but objects should appear only as structured as the user needs them to be.**

A single value can be an object.
A group of related values can become a record.
A collection of records can become a frame.
Frames can be transformed, grouped, summarized, joined, or displayed in multiple ways.

The canvas provides freedom of layout. The objects provide structure.

---

## 2. Product philosophy

The application should combine several existing ideas without becoming a clone of any one product.

From **Numbers**:

- a canvas rather than one infinite spreadsheet grid;
- multiple independent frames on a sheet;
- visually attractive layout;
- frames as first-class objects.

From **Excel**:

- extremely fluid direct editing;
- formulas can be typed where the user is already working;
- exceptions and exploratory calculations are allowed;
- summaries can live directly beside the underlying data.

From **dataframes / Polars**:

- columns have identity and type;
- column expressions are first-class;
- transformations operate over structured data;
- group-by, filtering, joins, rolling calculations, and window operations are natural;
- derived data is represented as operations rather than copied formulas.

From **databases**:

- objects have stable internal identities;
- relationships are explicit;
- physical layout does not determine semantic identity.

From **document/layout applications**:

- data objects can be positioned freely;
- multiple views of the same underlying data can appear simultaneously;
- explanatory text, charts, parameters, and frames can coexist.

---

## 3. Fundamental conceptual model

The document should contain **objects positioned on canvases**.

A canvas is analogous to a Numbers sheet, but the canvas itself contains no cells.

```text
Document
│
├── Canvas: Overview
│   ├── Frame View
│   ├── Parameter
│   ├── Summary
│   ├── Chart
│   └── Text
│
├── Canvas: Detailed Analysis
│   ├── Another view of the same Frame
│   └── Derived Frame
│
└── Data/Object Graph
    ├── Values
    ├── Records
    ├── Frames
    ├── Formulas
    ├── Relationships
    └── Transformations
```

The canvas controls **where something appears**.

The object model controls **what something is**.

These must remain separate.

---

## 4. Every piece of data is an object

All objects carry a stable internal identifier — completely hidden from normal users — and an optional name. References display human-readable notation (`Orders.amount`) but store IDs, so renaming `Orders` to `Purchases` breaks nothing. Names are human aliases; IDs are identity. (The identifier scheme itself — name-derived slugs rather than UUIDs — is settled in §29.)

---

## 5. Object hierarchy

The smallest useful model is **Value → Record → Frame**, with additional objects representing formulas, views, and transformations.

**Value** — a scalar (`4.25%`, `$850,000`, a date), optionally labelled; internally `{ id, name?, datatype, value }`. *Updated 2026-08-13: a standalone value no longer gets its own canvas card — a scalar's home is a line in a formula block; see [The block is the only home for a scalar](#the-block-is-the-only-home-for-a-scalar-2026-08-13). The kind stays in the model and inside containers.*

**Record** — a collection of named fields, conceptually a typed struct (`Mortgage { price, down_payment, rate, term }`). Especially useful for assumptions, parameters, configuration, metadata, and individual entities. Visually lightweight — it must not feel like the user has created a database schema.

**Frame** — structured rows and columns, with a stable object ID, optional name, columns with stable IDs and datatypes, rows with stable IDs, and explicit display ordering. Frames may begin unnamed; a user creates one without entering a title or configuring a schema.

---

## 6. Progressive structure

Structure emerges instead of being designed up front: a lone number is a Value, a labelled stack of them reads as a Record, repeated instances become a Frame. The reverse moves — frame row as record view, record field as value view — are alternative presentations, not conversions of the underlying data.

---

## 7. Canvas interaction

The canvas should feel close to Numbers: multiple independent objects placed anywhere, and none of the whitespace is data. Objects can be moved, resized, duplicated as views, aligned, grouped visually, collapsed, and expanded.

---

## 8. Adding new data must be nearly frictionless

This is critical. Creating structure cannot feel like creating frames in a database application. The primary interaction is: click or double-click empty canvas space and begin typing. *(Updated 2026-08-13: every empty-canvas gesture now opens a formula block, and a lone number or `rate = 0.08` is a block line — see [The block is the only home for a scalar](#the-block-is-the-only-home-for-a-scalar-2026-08-13). Column-shaped text still becomes a frame.)* A lightweight insertion control, and possibly slash commands, supplement this but are never required.

---

## 9. Paste should be a first-class creation mechanism

Pasting should intelligently produce the appropriate object: delimited rows, TSV, spreadsheet cells, or a JSON array → Frame; key/value lines or a JSON object → Record; a lone scalar → a block line. Dropped CSV or Parquet → frame/data source.

*Landed:* adding a frame makes an empty 2×2 on the canvas, and pasting into an empty frame replaces it with the clipboard's contents — headers, column count, and types all inferred by the core's Polars reader, the same one a file import goes through. Pasting into a frame that already has data writes from the anchor and grows it downward instead. The modal with a textarea in it is gone; the grid was always the better text box.

This makes the system feel far less constrained than its internal architecture actually is.

---

## 10. Formula model

Formula flexibility is essential; the system supports several calculation scopes rather than forcing everything into one paradigm.

### 10.1 Scalar formulas

A calculated value: `= DownPayment / PurchasePrice`. Now specified as computed values living on block lines — see [Computed values](#computed-values-the-0-d-case) and [Formula block](#formula-block-scratchpad).

### 10.2 Computed columns

A formula applied uniformly across a column (`quantity * unit_price`), compiling naturally to a Polars expression. The user writes the formula once. There are no thousands of copied cell formulas.

### 10.3 Cell-level calculations belong in Scratchwork *(decided 2026-08-16)*

A typed frame column has one semantic type and, when calculated, one visible declaration. FrameWork no longer creates formulas attached to individual cells: that scope hides exceptional logic inside the data grid and recreates the copied-cell model the calculated-column design is meant to avoid. A calculation needed only once belongs in `Scratchwork`, where it remains compact, named when useful, and independently referenceable.

Typing `=` with one cell selected therefore moves the cursor to a fresh Scratchwork formula. Typing `=` with a whole column selected still opens the all-rows calculated-column declaration in Wrangle. Ordinary typing in a document-owned cell remains literal editing.

The persisted cell-override representation remains readable for compatibility with documents created while this scope was available. Existing overrides may still be inspected, corrected, cleared, and rendered with `ƒ`, but the ordinary UI does not create another one.

### 10.4 Live computation and positional identity are independent *(decided 2026-08-16)*

This distinction is an invariant, not a per-surface UX choice:

- **Whole-frame meaning stays live.** `` `Orders`.`Amount`.sum() ``, mean, mode, quantiles, counts, grouped aggregates, and other semantic queries evaluate against the current base or derived frame. An upstream row edit or refresh recomputes the Scratchwork line and every downstream Scratchwork line that reads it. Materialization is an optional cache; freezing is an explicit historical capture. Neither is a prerequisite for a live Scratchwork calculation.
- **Liveness is not a capability restriction.** No operation is refused merely because its input frame is live. Filters, lookups, ordered queries, calculated columns, and aggregates all continue to describe the current result. A later refresh may legitimately make a key disappear, empty a result, or surface a type error; that is ordinary live-query behavior and not a reason to snapshot the frame first.
- **A profile click is formula authoring.** Clicking `Sum × Amount` inserts the same durable aggregate expression a person could type. This remains available for imports and derived frames because the expression describes the current dataset rather than a screen position.
- **Specific cells belong only to internal datasets.** A cell in a document-owned frame has a durable row id. A row produced by an import, live query, or derived frame does not, so the UI must not translate its current screen ordinal into `.head(n).last()` or another coordinate. This is the one formula capability unavailable on a live frame; column references, profile statistics, key-based filters, ordered queries, and every other semantic operation remain available.
- **Cell references must be as easy as Excel.** With a Scratchwork editor active, clicking a cell in an internal dataset inserts its scalar reference directly. The user never has to discover, type, or manage an internal row id. The current ordinal spelling is offered only where stored and shown row order coincide; extending it through an internal frame's sort or filter requires resolving the click to the literal row id in the expression tree rather than weakening the interaction or persisting the visible position.

The implementation boundary belongs in the shared active-editor/formula-picking path, not separately in each frame orientation or formula bar. Every new formula surface must prove both halves in tests: live semantic propagation after an upstream edit, and refusal of a live-frame cell click without disabling any non-cell operation.

---

## 11. Formula syntax

Easy to type and strongly inspired by Polars expressions, without requiring Rust or Python: `quantity * unit_price`, `if(status == "paid", amount, 0)`, `date.year()`, `customer.region`. Internally a formula compiles into an expression tree over stable IDs — `Multiply(Column(quantity_id), Column(unit_price_id))`. Human-readable source text is not the canonical representation, which is what makes renaming objects safe. The canonical reference form is backtick-delimited exact names (§27.1).

---

## 12. Formula scopes

Formulas need at least four distinct semantics.

### 12.1 Scalar

One expression produces one value.

```text
MortgageAmount = Price - DownPayment
```

### 12.2 Vector

Every row can be computed independently.

```text
total = quantity * price
```

### 12.3 Window

Calculation depends on nearby rows or groups.

```text
change = value - lag(value)
```

```text
rolling_mean(value, 7)
```

```text
rank(revenue)
```

Window calculations require explicit ordering.

### 12.4 Sequential

Calculation depends on the previously calculated state.

```text
Balance[n] =
    Balance[n-1]
    + Deposit[n]
    - Withdrawal[n]
```

The UI could expose:

```text
Starting value: 0

For each row:
Previous + Deposit - Withdrawal
```

This handles amortization, inventories, balances, simulations, and other recursive spreadsheet calculations.

---

## 13. Order must be explicit

Spreadsheet formulas often silently depend on row order.

This application should expose that assumption.

For example:

```text
Running Balance

Order by:
    Date ascending

Partition by:
    Account

Starting value:
    Opening Balance

Formula:
    Previous + Deposit - Withdrawal
```

This is more explicit than Excel while remaining understandable.

Rows still have stable UUIDs internally.

Their visual position is not their identity.

---

## 14. Frame summaries

Frames have a first-class summary/profile drawer under their displayed rows. It opens and closes from the compact `Σ` control in the frame title, and its top line is always the statistic multi-select rather than another row at the end of the data. The divider above the drawer resizes its height independently of the frame card, letting a reader keep one row visible or inspect a longer profile without giving the selector away. A statistic is one dense row beneath that selector: its short name (`Sum`, `Mean`, `25%`, `Mode`) occupies the row-header gutter, and every frame column gets its answer or an explicit `n/a` when that statistic does not apply to the column's type. A one-click profile preset gives the familiar dataframe/Data Wrangler readout — count, missing, distinct, min, quartiles, mean, max, sum and mode — while the multi-select can show any combination, only `Sum`, or nothing. It repeats no sample values because the records immediately above already are the sample.

These are not ordinary data rows and they do not enter lineage. The selected rows, drawer visibility and drawer height are view state, while their answers are queried over the complete displayed frame (after its display filter) in one on-demand aggregate pass, and only while the drawer is open. Keeping a profile on a million-row import must not make every unrelated canvas edit rescan it. A whole-column selection may still offer *Keep Sum/Mean/Count* as the shortcut into the same drawer; the result is the whole statistic row across every compatible column, not a mostly empty row holding one number.

A profile cell is also a pointing surface for formulas. Clicking `Sum × Amount` while a formula editor is active inserts the ordinary aggregate expression represented by that cell, such as `` `Amount`.sum() `` or the qualified cross-frame form. The same expression can be typed directly, including ``.quantile(0.25)``, ``.null_count()``, and ``.drop_nulls().mode(True).first()``. The expression, not the drawer's visibility, is the durable dependency: closing or removing a displayed statistic cannot break a calculation that was built from it.

---

## 15. Grouped summaries

A frame can be grouped with subtotals — underneath each group, at the bottom, as a separate derived frame, or in a chart — without creating a pivot frame. Grouping belongs naturally to the frame view; the underlying operation is `group_by(...)` plus aggregation.

---

## 16. Derived frames

More substantial transformations produce Derived Frames (filter → group → aggregate → sort), which appear just like other frames on the canvas. Derived frames are not directly editable unless explicitly materialized; their source definition is.

---

## 17. Transformation UI

Both formula syntax and a visual transformation pipeline (Source / Filter / Group / Aggregate / Sort), compiling to a Polars LazyFrame query. The visual model and the textual expression must describe the same underlying operation graph.

---

## 18. Map/Reduce-style operations

Dataframe operations (select, with_columns, filter, sort, group_by, aggregate, join, explode, pivot, unpivot, rolling, window, scan) are first-class, but user-facing terminology stays approachable: *Keep columns, Add calculated column, Filter rows, Sort, Group, Summarize, Join frame, Expand list, Pivot*. Users should not need to know what MapReduce means.

---

## 19. Relationships

Frames can define explicit relationships (`Orders.customer → Customers`, internally by ID; the user sees names). A formula then says `customer.region` rather than `XLOOKUP(...)`. Relationships should behave almost like object references.

---

## 20. Views

The same object may appear differently in different locations — frame, record, card, summary, or chart view — all views onto one source object. Selecting a frame row can present it as a record card elsewhere. No data duplication occurs.

---

## 21. Naming

Objects are unnamed by default; the user should never be confronted with `Frame1 / Frame2 / Frame3` unless a temporary UI label is necessary. An unnamed frame appears simply as its content, and the interface offers *Name this frame…* when a reference needs it, after which dot notation and autocomplete apply. Imported data usually provides column names naturally. (Autocomplete has since grown type-aware; see Formula UX in §34.)

---

## 22. Object grouping

Objects are visually groupable — moving together, collapsing, carrying a heading, optionally exposing shared parameters — without merging their data models. Grouping must not implicitly alter formulas or storage: presentation hierarchy, never data hierarchy.

---

## 23. Object-level operations

Selecting an object exposes contextually appropriate operations — a frame offers filter/sort/group/summarize/join/calculated column/derive/chart/duplicate view/rename; a record offers field-level moves; a value offers edit/formula/format/reference. The UI avoids presenting irrelevant database operations.

---

## 24. Direct manipulation

Structured operations should feel as direct as spreadsheet editing: drag a column onto another frame → offer *Create relationship?*; drag a column onto the canvas → offer a summary; drag a record field out → another view of the same value. Numbers-like visual interaction, semantic structure retained.

---

## 25. Data types

Columns and values have real data types: Boolean, Integer, Float, Decimal, String, Date, Datetime, Duration, Currency, Percentage, Category, List, Record/Struct, Reference — with null as a value state available in every type rather than a type of its own (§27.1). Formatting is separate from datatype (`datatype = Decimal`, `format = Currency(CAD)`); the distinction is important.

---

## 26. Local persistence architecture

*(The original sketch here proposed SQLite + Parquet + JSON layers. What shipped is simpler, and is now the settled design.)* A `.fw` document is deterministic, pretty-printed JSON — schema, formulas, literal rows, layout — with content-addressed Parquet artifacts in a `.framework/<id>/` sidecar for imported and materialized data. Polars sits above as the dataframe/query engine. §29 records what the format already gets right and must not break.

---

## 27. Rust architecture

The core is Rust (`framework-core`), and the UI communicates with it through a clean command/query API. (The conceptual module sketch this section once held is superseded by the actual crate layout — see [docs/architecture.md](docs/architecture.md).)

### 27.1 AI-native and programmatic control

The command/query API is a product surface, not only an internal UI boundary.

AI assistants, scripts, tests, and the desktop UI should be able to inspect and manipulate the same semantic objects without browser automation, simulated clicks, or spreadsheet coordinates. MCP should be the first LLM-facing adapter, with ordinary CLI/SDK adapters using the same underlying contract.

Formula references should canonically use backtick-delimited exact names. For example, `` `Quantity` * `Unit price` `` and `` `Imported data`.`Amount` `` are explicit without exposing internal IDs. Backticks distinguish exact display names from optional forgiving shorthand lookup; once parsed, both forms still store stable IDs. Autocomplete and rendered formulas should emit the exact form.

Formula functions should be defined by a canonical registry rather than coupled directly to public Polars method names. Each entry should declare a stable function ID, canonical friendly name, searchable aliases, category, signature, description, argument rules, supported execution contexts, type behavior, purity, and an approximate cost class. Saved expression trees store the stable ID. Autocomplete displays the friendly name and aliases, and the Polars compiler maps the ID to the current engine implementation.

The executable catalog now covers deterministic row expressions, null handling, booleans, strings and categories, calendar dates, aggregates, shifts, grouped windows, common rolling calculations, and generated numeric/date sequences. Null is a value state available in every column type, not a datatype of its own; blank input is null, while malformed non-empty input is an error. More specialized option-heavy methods, timezone work, nested-value transformations, nondeterministic functions, and shape-changing operations remain unavailable until their execution contracts are explicit. The maintained catalog and exclusions live in `docs/formula-function-catalog.md`; the UI must never autocomplete a deferred function as though it were executable.

The automation surface should:

- expose task-level tools such as inspect document, read frame, set value, append row, set cell, add literal or calculated column, change column type, and add summary;
- accept convenient names while returning stable object, frame, row, and column IDs;
- return structured data, computed results, formula errors, and document revisions;
- distinguish read-only tools from mutations;
- support optimistic revision checks so stale agents do not silently overwrite newer intent;
- use the canonical formula parser, operation model, validation, history, and persistence code;
- remain testable without starting the visual application.

It should not expose raw filesystem mutation or make screen coordinates the programming model. Canvas coordinates are presentation metadata and are only relevant when placing or moving views.

For simultaneous human/agent collaboration, one process must own canonical mutable state. The desktop, MCP clients, and scripts should eventually connect to that shared service, which serializes operations and publishes document changes. Multiple independent processes must not load and overwrite the same document file.

#### Implementation snapshot and direction (2026-08-14)

**Snapshot.** At this date, `framework-mcp` exposed 22 tools over stdio. Twenty focused tools covered document and frame inspection, type-aware formula completion, formula blocks, frames, values, cells, rows, literal and calculated columns, categories, overrides, summaries, and undo/redo. Two generated escape-hatch tools exposed the whole public mutation surface: `describe_operations` recursively rendered the canonical Rust `Operation` contract and `apply_operation` deserialized one of those exact operations before sending it through the same validation, history, persistence, and collaboration path as the desktop. A new enum variant therefore became programmatically writable after recompilation without acquiring another MCP handler. This established full mutation-enum parity, not full query/workflow parity. Access was opt-in per machine in Settings. MCP and the desktop opened independent in-memory stores, while immutable per-writer operation events made edits to an open document visible without either process overwriting the other's snapshot. `README.md` and `docs/automation-api.md` carry the current tool count and remaining query coverage.

**Direction: the running app becomes the server.** The tool router is already library-shaped; host it inside the desktop process and expose it through two doorways — **in-process** for the AI chat panel (§34), which is the non-technical path: paste an API key, chat in a sidebar, watch edits land live, revisioned and undoable; and **localhost** (streamable HTTP) for external clients — Claude Desktop or any MCP client connects to the *running app*, never the file. The stdio binary remains for scripts and CI on closed documents. One process owns the document in every configuration; transports are doorways, not owners.

**No webview, no browser automation.** An agent reading `inspect_document` sees types, formulas, lineage, and IDs — strictly more than pixels. Screen-driving would be a downgrade from the surface already built, and screen coordinates stay banned as a programming model. The one job sight is good for — judging a layout — gets a `screenshot_canvas` *tool* returning a rendered image: an optional sense organ, never a control surface.

**Punch list**: embed + localhost transport (the unlock; plumbing, not design) → chat panel as MCP client → verbs an agent still lacks (derived frames and wrangle steps, plots, canvas placement) → screenshot tool → verify the optimistic revision checks specified above are actually enforced rather than assumed.

---

## 28. Operation-based mutation

Every user mutation should become a typed operation.

Examples:

```text
SetValue
InsertRow
DeleteRow
AddColumn
SetColumnType
RenameObject
SetFormula
SetColumnFormula
MoveCanvasObject
ResizeCanvasObject
CreateRelationship
AddSummary
```

This enables:

- undo;
- redo;
- audit history;
- autosave;
- semantic diffs and commits (see §29).

The UI should never directly mutate canonical state.

### 28.1 Undo and redo

**History is a LIFO stack of inverse operations, held in memory for the session only.** That representation is already built and is the right one. The simplification below is the target after the publication model in §29 replaces the current multi-writer journal; until then, the second-writer guards remain real implementation requirements.

Three things come out:

- **The skip loop.** `Store::undo` pops entries until one applies, because a remote writer could have deleted the frame an inverse names. With one writer the stack is strictly LIFO over a document nobody else touched, so each inverse meets exactly the state it was computed against and always applies. Undo becomes pop, apply, done — and the failure mode where ⌘Z silently does nothing goes with it.
- **The rollback clone.** `apply_history` clones the whole document before applying so it can restore after a partial failure. There is no partial failure to survive.
- **The validation clone.** `prepare_event` clones the entire `Store` on *every* operation, because writing an immutable event file is irreversible and must be validated first. No journal, no irreversible publication, no clone — and this one sits on the hot path.

**Depth stops being a compromise.** `UNDO_DEPTH` is ten, which reads as a limitation only while undo is the sole way back. It isn't: undo covers the current train of thought, and a commit covers everything older. That division only holds if checkpoints are frequent, so the application should commit on a cadence — on close, on idle, on demand — rather than leaving it to somebody's discipline. Nothing about undo needs to survive a restart; surviving a restart is what a commit is for.

**Do not wire ⌘Z to git.** They are different altitudes. Undo is ephemeral, fine-grained and personal; a commit is durable, coarse and shared. Conflating them yields either a commit per keystroke or an undo granularity set by whenever someone last remembered to commit.

### 28.2 Autosave, and what a checkpoint is for

**Autosave stays, and version control is what finally makes it comfortable.** Every operation already persists on its way through the store, so there is no unsaved state and no Save command. That was only ever uneasy because there was no way back past it — and a checkpoint is exactly that way back. The answer to *I want to try something without committing to it* is therefore not a manual save; it is a visible discard, **revert to the last checkpoint**. Make that findable and autosave stops being something to weigh.

Autosave and committing are orthogonal, and conflating them is the mistake to avoid: autosave writes the working file, a checkpoint records a point worth returning to. Exploring freely and committing nothing is already the supported path.

Three qualifications:

- **Debounce the write** — *recorded, deliberately not done yet; save-on-every-operation stands until the versioning work is real.* `persist_session` writes the whole snapshot on *every* operation, on the order of fifty times a minute during real work. Two seconds idle, plus on blur and on quit, would give the same durability in practice with a fraction of the I/O, and would stop filesystem watchers thrashing on a document nobody has finished thinking about. It buys nothing until something is watching.
- **Version control is opt-in per document.** A scratch canvas, and a `.fw` sitting in a downloads folder, have no repository and should never mention one. Quick analysis must not require thinking about history at all — that is most of what makes it quick.
- **Write layout separately from the model.** `Document` already separates `objects` from `views`, but they share one file, so dragging a card marks the whole document modified. Splitting the write keeps pure exploration — pan, zoom, drag, resize — out of the diff of what the analysis actually says, and keeps "modified" meaning something.

**⌘S comes back, and it means commit.** It was removed because there was nothing left to save, which is true and beside the point: the intent behind pressing it survives autosave completely — *I am at a good point, record this.* Under version control that intent has a name. Bind ⌘S to a checkpoint with a generated summary and the option to name it, offered only for documents that are actually versioned.

One consequence elsewhere: reclaiming unused data files computes reachability from the document, the undo and redo stacks, *and* the journal events not yet applied. Dropping the journal removes a root, and a session-scoped stack releases the rest sooner.

---

## 29. Sharing model

**A document is a program. Data is its input. We share the program and never the data.**

Sharing is asynchronous and git-shaped — commit, push, pull — not live co-editing. What travels is a small text file of source references, formulas, structure and layout. What never travels is a row of anyone's data.

See the decision log entry *Sharing is publication, not collaboration (2026-08-13)* for the reasoning and the rejected alternatives.

### What a shareable document contains

Nothing typed in. No entered rows, no live scalar values, no inline lookup frames — a lookup frame lives as a CSV beside the `.fw`, where git can diff and review it line by line. **Shareable is a checkable property**, and the application should report which objects disqualify a document rather than expecting people to remember the rule. Local scratch documents may hold whatever they like; they are simply not shareable until they don't.

Data lives read-only in a source both parties read directly — object storage, or a live warehouse connection. Access to it is the recipient's own grant, so "you do not have access to this source" is a first-class UI state. **Credentials are never written into a document.**

### Stable IDs, not UUIDs

Stable identifiers are still required: a formula must keep pointing at the right column after someone renames it. Globally-unique-without-coordination identifiers are not — that property existed only to let two writers mint IDs simultaneously, and there is one writer now.

```text
source column   → creation-name slug plus a short random suffix
derived column  → the same shape
                  readable in plans and errors, stable across renames,
                  independent of the frame name, never reused
frame / object  → slug minted from its name
document        → UUID; it names the .framework/<id>/ sidecar
```

The suffix, for example `amount~k7m3q2`, is identity; the slug is diagnostics. It is not frame-qualified because frame names are edited more often than column names, and leaving the old frame name embedded in every physical column would make a harmless rename permanently misleading. A source-backed column separately stores the physical field it reads. Replacing or refreshing a source matches that binding, retains IDs for surviving fields, mints IDs for additions, drops unused deletions, and keeps referenced missing fields so the resulting failure names what disappeared rather than degrading to `#REF`.

Random UUIDs made diffs unreviewable, which defeated the point of a git-shaped workflow. They also named the columns *inside* materialized snapshot parquets, making those files opaque to every other tool. The readable stable IDs now in place make cached outputs intelligible in pandas or DuckDB too.

Changing the scheme invalidates cached snapshots — they regenerate — and wants a `FRAMEWORK_FILE_VERSION` bump with a converter. The seam already exists in `Store::load`.

### What the file format already gets right

Worth not breaking: `Store::save` writes pretty-printed JSON; every serialized collection is a `Vec` or `BTreeMap`, so the same document produces byte-identical output every time; artifact paths are relativized on write, so a clone resolves at a different path; and `Document` separates `objects` from `views`, so moving a card does not diff the model.

One field does not belong in a committed document: `version_vector`. It is replication state, meaningless across clones, and pure diff noise.

### Later, and only if wanted

A second transport can carry **awareness** — who is in the document, who is editing which formula, who pushed four minutes ago. Ephemeral, advisory, forgotten on disconnect, and **never authoritative**. Git remains the source of truth. Split that way, the live channel needs no conflict resolution at all, which is the entire reason the hard problems stay solved.

---

## 30. Archived MVP sequence

*Superseded.* This section held the original MVP 1–6 sequencing sketch. The current replacement threshold and the dated design notes now live in §34. Its "MVP 6: collaboration" line is overtaken by §29 — sharing is publication — and by the standing non-goal in §31.

---

## 31. Explicit non-goals

The product should initially refuse several temptations.

Do not attempt to support:

- complete Excel file compatibility;
- VBA;
- arbitrary coordinate-based formulas;
- INDIRECT/OFFSET-style dynamic cell addressing;
- macros;
- arbitrary spreadsheet grids;
- every possible chart;
- real-time collaboration — now a standing non-goal rather than a deferral, see §29;
- a full SQL IDE;
- enterprise BI functionality.

These are exactly the features that could turn the project into an unbounded Excel clone.

---

## 32. The central compromise

The application should not prohibit unusual calculations.

Instead it should distinguish:

```text
structured default
+
explicit exception
```

Examples:

A calculated column has one visible formula; a one-off calculation lives in Scratchwork.

A frame has a schema, but a user can add another column immediately.

A frame is structured, but it can sit anywhere on the canvas.

An object can remain unnamed until a name becomes useful.

A calculation can be dataframe-oriented, but users can type it directly.

This preserves exploratory spreadsheet behavior without making arbitrary physical coordinates the underlying programming model.

---

## 33. Core product statement

The application can ultimately be described as:

> **A free-form canvas for structured data.**

Or more technically:

> **A spreadsheet where every piece of information is an identifiable object, frames behave like dataframes, formulas operate on semantic references rather than coordinates, and users retain the freedom to make local exceptions when necessary.**

The application should feel simple enough to start by typing one expression, yet structured enough that the same document can contain relationships, group-by operations, sequential calculations, large local datasets, and publishable analyses.

That progression—from one expression to a real analytical model without forcing the user to change tools—is the central product idea.
---

## 34. Replacement threshold, roadmap, and design notes

The first section below is the current product-level priority. The dated
sections after it are design records: they explain why features have their
present shape and include both landed and proposed work. They are retained for
their reasoning, not used as a second current-state checklist. See
[docs/product-brief.md](docs/product-brief.md) for the audience and
[docs/architecture.md](docs/architecture.md) for implementation state.

### Excel replacement threshold

For a willing user adopting the structured style, formula-count parity is not
the barrier. FrameWork can already cover substantial dataframe-shaped analysis;
it becomes a credible primary tool when the whole job can begin and end here.
The remaining blockers, in priority order, are:

1. **Interchange and delivery.** Accept the Excel files people receive and
   produce useful handoffs without adopting Excel as a second calculation
   model. Explicit worksheet-range and defined-table import is **landed
   (2026-08-19)**: the user chooses the rectangle and header row, previews
   cached values, sees formula/error warnings, and may repeat the gesture for a
   workbook containing several tables. Excel formulas, formatting, charts, and
   workbook layout are deliberately not imported. The remaining handoff is a
   results-only XLSX export is **landed for tables and scalar answers
   (2026-08-19)**: selected frames become worksheets, and constants, result
   cards, and named Scratchwork lines share a Name/Value sheet with qualified
   names. No FrameWork expression is translated into an Excel formula. Plot
   images, reusable export manifests/templates, and report or present output
   follow that same values-and-presentation boundary.
2. **Repeatable data access.** Move beyond linked CSV/TSV/Parquet files to
   broader database and API sources, append workflows, schema-change handling,
   progress, and cancellation. Refresh stays an explicit table action.
3. **Trust at scale.** A table, summary, export, and plot must either represent
   the complete result or say exactly how it was sampled. Global work needs
   streaming execution, resource limits, and visible cost rather than silent
   partial answers or an unresponsive application.
4. **Finished business workflows.** Reconciliation needs tolerance,
   one-to-many matching, and persistent reviewed matches; planning needs fiscal
   calendars, scenarios, sensitivity, goal seek, and eventually iterative
   solve; close work needs validation, accounting formats, exact decimal
   behavior, and templated export.
5. **An irregular-work escape hatch.** Scratchwork handles one-off calculations
   and crosstabs handle long-to-wide presentation, but compact schedules,
   forms, and mixed scalar layouts still need the constrained Databoard
   described below. It must reuse stable objects rather than introduce a
   coordinate-based calculation model.
6. **Safe adoption.** Reliable session undo, durable checkpoints and recovery,
   approachable reusable automation, migration diagnostics, and in-product
   teaching must make trying FrameWork safer than keeping the familiar
   workbook.

This is deliberately not a promise of complete Excel compatibility, VBA,
arbitrary coordinate formulas, or every chart. Those remain non-goals (§31).
The threshold is job completion around the structured core, not imitation of
Excel's implementation.

### Scale and persistence notes (2026-08-12)

Carried out of the caching/performance session. This section mixes landed
implementation notes with the remaining performance proposals; the status of
each entry is stated locally.

- **Readable column IDs in place of UUIDs.** Implemented as `name_slug~suffix`, independent of frame identity and immutable across renames. Source fields have a separate binding, so display renames do not alter a Parquet or SQL lookup and source replacement can reconcile additions and deletions without repointing an existing ID at different data. Materialized snapshots therefore carry useful physical names in query plans and external tools without making editable names canonical identity.
- **Offer caching by cost, never by nag.** Caching is manual and opt-in, which is right: a stale number that silently feeds a formula is worse than a slow one, and a per-frame toggle asks people to predict cost before they have felt it. The middle path is to offer materialization once a read crosses a cost threshold, rather than asking up front or deciding silently.
- **Deduplicate identical derivations.** A real document arrived with three derived frames computing the same group-by over the same 1.18M rows, each re-running it on every page read. Detect identical lineage fingerprints and either share one computation or say so in the UI.
- **~~Live or static import, as a choice at import time.~~ Landed.** Import asks before the file picker opens — *store it in this document* (the default) or *keep it linked to the file* — and both settings live in Preferences, including whether to ask at all. It was the one field the entry predicted: `connector: linked.then(...)`, no second import path. Of the two questions the original entry raised, (2) — whether static is a property of the import or of the document — landed as File → *Package this document*: one edit, one undo entry, connectors worked out from the document and artifacts written for any frame that was reading a path directly, composing with per-frame choice rather than replacing it. What remains is (1): whether "no external dependencies" ultimately means a self-contained directory or a single-file `.fw` container. That governs the file format and should not be settled by accident; the parquet already travels beside the document in `.framework/<id>/data/`, so a static import is self-contained now that paths are relative. Note that "static" already means something to the interface: `ComputedFrame.live` is inherited down the lineage, and a frame nothing upstream re-reads says so rather than warning about a refresh that will never come.

  Worth recording why the default went to stored. A linked import makes the document depend on a file that may be moved, edited, or on a machine nobody else has, and it makes its own values uneditable because a refresh would replace them. Neither is wrong, but neither is what someone importing a CSV for the first time is asking for, and the option that cannot surprise anybody is the one that should not need to be chosen.

- **~~Collect the parquet files an edit leaves behind.~~ Landed.** File → *Reclaim unused data files* sweeps the sidecar. Reachability is computed from three places rather than guessed at: the document, the undo and redo stacks, and the events in the journal that this store has *not yet applied* — another writer's import, sitting in their event file waiting to be merged, points at a file this document has never heard of. Events already applied are deliberately not consulted, since a merge only ever replays what comes after the version vector. Reachability is read by collecting every string in the serialized operations, because a hand-written match over forty operation variants that carries whole frames and whole documents is one missed arm away from deleting a file something still needs. Note the consequence of undo holding references: right after an editing session almost nothing is collectable, and reopening the document is what lets go.

- **~~Take ownership of an imported frame's rows.~~ Landed** — as two actions rather than one, because the interesting decision turned out to be what happens to the frame you started from. *Freeze a copy* writes the current values to a parquet and adds them as a second frame, leaving the original live; *take ownership* converts the frame in place, dropping the connector or chain that would have overwritten it. Both write the same file through `Store::write_owned_frame_data`, whose one difference from a snapshot is that the columns are named the way an import names them — so what comes back is an ordinary imported frame rather than a cache with a fingerprint, and every existing path already knows how to read, page, and now edit it.

  The size guard the entry worried about turned out to be unnecessary, and the reason is worth keeping: ownership stores the values as parquet beside the document, not as rows inside it, so a 1.18M-row frame costs what it always cost. Editing one rewrites the file — parquet has no in-place write — which is charged once per committed edit. The identity question the entry raised is answered by the same design: a scanned row's ordinal is its identity, and it is stable because nothing else writes that file. Two pieces of the original entry's reasoning worth keeping: ownership could never be a *flag*, because an imported frame's values are scanned from its parquet on every read — a typed value into `frame.rows` is exactly the silent no-op the editing metadata refuses — and the halfway house it weighed, an override layer keyed to positional row identity on scanned rows, stays rejected: a refresh that reorders rows moves the edit onto the wrong one.

**Post-aggregation calculated columns — already built, and the entry was stale.** It described the old `FrameDerivation` field layout, where `projections` and `aggregates` sat side by side and a derivation chose one. The chain replaced that: steps compose, and each is parsed against the schema at its own position, so a `WithColumns` step after a `Summarize` reads the aggregates the summarize produced. No post-aggregate stage and no new scoping context were needed — the wrinkle the entry worried about was solved by the unification. Both the core and the step editor already allow it; `a_calculated_column_can_follow_a_summarize_in_the_chain` now pins it down.

**Save As becomes an ordinary copy — landed, plus two things the entry did not know were broken.** Artifact paths are now written relative to the document, so a `.fw` and its `.framework` sidecar are portable by `cp -r` with no relinking needed; relinking stays as the fallback for documents written before this and for imports that legitimately point outside the sidecar. In memory paths stay absolute, so nothing that reads a parquet has to know where the document lives or whether it has been saved at all.

The two bugs. First, `save_as` copied a frame's *imported* artifact and not its *snapshot*, so a copy went on reading the original's caches — and broke outright the day the original was deleted. Every place that touches artifacts now goes through `Document::artifacts_mut`, so there is one list to forget rather than two. Second, and worse: the lineage fingerprint hashed the artifact's **path**, which meant moving a document, copying it, or opening it on a machine with a different home directory marked every snapshot in it stale. The address of a file is not part of what a frame computes; identity is the artifact id. Note the one-time effect of that fix — existing documents recorded their fingerprints under the old formula, so each one reports its snapshots stale once, and refreshing clears it for good.

**Refresh everything stale — landed, and one half of it was already true.** The entry asked for a document-level refresh and for staleness to be inherited. The second half turned out to be half-built: a fingerprint hashes the *whole* lineage, not just the parent's snapshot record, so a cached frame already reported itself stale when an edit landed three frames upstream. What genuinely had no signal was the **live** frame below a stale cache — it has no snapshot of its own to fall behind, and its rows are exactly as old as the snapshot it reads. `ComputedFrame.upstream_stale` is that signal, and the canvas says "reading old numbers" on the card.

The document-level action is `snapshot_refresh_order` — every cached frame, parents first — driven by a host loop that asks `snapshot_is_stale` at each step rather than collecting the stale ones up front. Both details are load-bearing. Refreshing a parent rewrites the snapshot its children read, so a list gathered before the pass is answering for a document that stops existing at the first refresh, and a child refreshed before its parent recomputes from numbers about to be replaced *and* stamps itself current. A frame under one that failed is skipped for the same reason: it would cache old numbers under a fingerprint claiming they are new.

**Undo as inverse events — landed.** History is now a stack of `ReplicatedOperation` inverses computed at apply time, so a remote event no longer clears it and undo reverts one edit rather than every difference between two snapshots.

One deviation from the plan above, worth recording. It said only `DeleteColumn` and `DeleteObject` need more than one op. In practice dropping a column also drops its summaries *and* its cells' one-off overrides, so `AddColumn` + `SetCells` restores a column that has quietly lost both — and the same shape recurs wherever an edit rebuilds a frame wholesale (`SetFrameContent`, `SetFrameSteps`, `PromoteDisplayToSteps`, `SetFrameSource`) or rearranges the canvas (the tab operations add and remove whole cards as strips empty and fill). Rather than grow a bespoke multi-op inverse for each, three operations exist solely to be inverses: `RestoreFrame`, `RestoreObject`, `RestoreViews`. They carry one frame, one object, or the view list — never a document, which is what keeps the distinction from the snapshots they replace. The ~25 operations that *can* describe their own prior state still invert to themselves.

Landed in the same session, for context: lineage-scoped cache invalidation replacing the global revision counter; `total_rows` no longer computed by running the query (`view()` with a grouped frame over 1.18M rows went 44ms → 67µs); materialize-to-snapshot with staleness reporting (grouped page reads 53ms → 1.7ms); view-local multi-column sort with drag-ordered keys; artifact relinking on load.

### Object kinds

Frames (frames), series, dicts (records), standalone values — plus, later:

- **Databoard**: a freeform Excel-like grid object that lives on the canvas alongside frames. Deferred; not yet designed. One guardrail to preserve when it is: board cells should be first-class scalar objects with stable IDs — a grid *arrangement* of standalone values — not a coordinate-addressed second data model. Cross-references between board cells and frames/series then work through the same stable-ID resolution, lineage, and rename safety as everything else. Computed values plus the formula block are the prerequisites: a board is the same namespace arranged in 2-D rather than as an ordered list, so build those first and the board is a layout problem.

Computed values, formula blocks, series, and named functions are specified below in [Series, functions, and expansion](#series-functions-and-expansion-the-drag-the-formula-job).

### Series, functions, and expansion (the "drag the formula" job)

**Status.** This section records the design sequence rather than the remaining
build order. Formula blocks, cross-object scalar references, series, expansion,
and pivot/unpivot have landed. Named functions remain outstanding; the
Databoard remains a future layout surface.

The dominant Excel job is: *I have data, a constant, and something hardcoded; I write one calc and drag it over a range.* That decomposes into four cases, and only one of them needs new primitives at the row level:

1. **Row-wise, same frame** — `=B2*C2*$F$1` dragged down. Already solved: this is a calculated column, and the keyboard canon already reinterprets Ctrl+D / drag-fill as "promote to column formula."
2. **Cross-object iteration** — run a calc across a set of inputs that is not a column of the frame being written into (categories, scenarios, rates). The real gap.
3. **Two-dimensional drag** — vary two inputs, produce a grid. The sensitivity-grid workflow.
4. **Ad-hoc scratch** — type a calc, get a number, use it somewhere else. Needs the 0-D case below; the Databoard is its dense *arrangement*, not a separate data model.

Cases 2 and 3 turn out to be the same primitive. Case 4 is the smallest unit of all of them.

#### Computed values (the 0-D case)

At the time of this design, `ValueObject` was literal-only — `raw` was parsed straight to a Polars literal, so a standalone value could be `0.08` but not `` = 0.08 * (1 - `discount`) `` and not `` = `ledger`.`amount`.sum() ``. The formula block subsequently became the user-facing answer: type an expression anywhere, get a result, and reference a named line elsewhere. `ValueObject` remains useful inside containers without regaining a one-number canvas card.

Give `ValueObject` an optional formula alongside `raw`. It is a **0-D calculated column** — same parser, same stable-ID expression tree, same dependency graph, so the existing cycle check and rename safety apply with no new machinery.

What a scalar formula may reference:

- **Other values** — `` `base_rate` * (1 - `discount`) ``. Derived assumptions become objects instead of arithmetic buried in a column formula, and they show up in lineage where a reviewer can find them.
- **Bare arithmetic, no references** — `= 1200 * 0.07 / 12`. A calculator that leaves a named, inspectable artifact instead of a dead cell. Must work with zero setup and no frame selected.
- **Aggregates over frames and series** — `` `ledger`.`amount`.sum() ``, `` `rates`.mean() ``. This is the dual of the no-broadcast rule: **a column in scalar position must reduce.** Evaluate in a `select` context and reject `height != 1` with an error naming the column that failed to aggregate. At the time, this depended on widening qualified resolution beyond a formula's own frame; that resolution work has since landed.

Two things keep the chaos energy alive rather than designing it out:

- **Promote from anywhere**: typing a formula with nothing selected creates a line in a block at the cursor. No dialog, no required name. (Originally a value card; see [The block is the only home for a scalar](#the-block-is-the-only-home-for-a-scalar-2026-08-13).)
- **Auto-naming**: scratch calcs land as `value_3` and get renamed later or never. Forcing a name up front is what kills the gesture, and renaming is already free because names are aliases.

The honest tension: Excel scratch works because cells are anonymous and free, while every FrameWork value is an object with a canvas card — forty scratch calcs is forty cards and a mess. The formula block below is the answer to that density problem; the Databoard is its 2-D cousin.

#### Formula block (scratchpad)

A canvas object holding an **ordered list of expression lines**, each showing its result in a gutter. Lines can reference document data, and other objects can reference the named lines. Name a block `General calculations`, and its named lines are reachable from anywhere as `` `general calculations`.account_balance ``. Numi is the reference for the *interaction shape* only — a linear scratchpad with instant per-line results — not for its input language.

This is the density mechanism for computed values *and* the on-ramp to the whole product: it is the only object usable on an empty canvas, before any import, in the first ten seconds. Everything else requires data first.

- **A block is a namespace, not a new value kind.** Its members are computed values as specified above — the block contributes containment, ordering, and a qualified name. No second scalar model.
- **A line holds whatever its expression evaluates to**, scalar or series, with the gutter showing the result alongside its dtype and length. "Arbitrary calculations" means not forcing every line to reduce to one number. This also absorbs the literal-series constructor below: a hand-typed list is just a block line, so there is one place to type a list rather than two.
- **Members later include named functions.** That answers "where does a function object live": a card per function is heavy, and a block is the natural home. Blocks then hold values and functions in one namespace.
- **Dot resolution is receiver-typed, so it cannot collide with method calls.** A block is not an expression receiver — it has no Polars methods — while a column has methods and no members. `` `calcs`.balance `` and `` `amount`.sum() `` disambiguate on what the receiver resolves to, with no grammar change.
- **Qualified resolution had to widen.** `FormulaReference::Qualified` originally resolved only against the formula's own frame. Block members and cross-object scalar aggregates both needed the document namespace — one resolution change serving two features rather than new syntax. That change has landed.
- **Auto-named lines, unqualified inside, qualified outside.** Every line lazily gets `line_4` so it is always addressable; naming is just renaming. References within a block may be unqualified; references from elsewhere must qualify. A flat global namespace of scratch names is how this turns into a swamp.
- **Lexical order within a block**: a line may reference only lines above it. The block reads top-to-bottom as a worked calculation and the error for a bad reference is obvious. Across blocks and other objects, the ordinary dependency graph and cycle check apply.
- **Comment and label lines.** Prose lines make the block a worked note, and they are where provenance for hand-entered numbers goes — "checking balance, read off the bank site, 2026-08-11." Today such numbers live as naked literals with no record of where they came from.

**One language, and the whole of it.** Lines are Polars expression syntax with backtick references — the same language as every other formula, with no natural-language layer. The requirement that matters is the opposite of a simplified calculator mode: **a scratch line gets the full expression surface a column formula gets.** Same catalog, same namespaces, same autocomplete, same aggregates over frames and series. There is no scratchpad-lite subset to design, maintain, or explain, and nothing a user learns here fails to transfer to a column formula. A declaration accepts either `down payment = 40000` or `` `down payment` = 40000 ``; the latter round-trips exactly, including doubled-backtick escaping, so a name does not acquire different declaration syntax merely because it moved between Scratchwork and Wrangle.

Two adjacencies worth noting. A block named `Base case` holding named lines **is** an assumption bundle, so this partly pre-builds the scenario-sets primitive. And the **Databoard** is the same namespace in a 2-D arrangement rather than a 1-D list — the block should ship first regardless, because it has no coordinate system at all and therefore no way to drift back toward coordinate addressing, and because being textual makes it writable by the AI panel.

#### Series object

First-class 1-D canvas object, the sibling of a standalone value, materializing as a Polars Series and referenceable from any formula by backtick name. Two constructors:

- **Derived** — `{ source_frame_id, column_id, filter?, distinct?, sort? }`. Refreshes with its source, renders a lineage cord, survives renames through the existing stable-ID resolution.
- **Literal** — a hand-typed list, which is simply a block line whose expression is a list; `Expr::List` already exists. No separate literal-series creation path.

**A series is a query, never a positional capture.** Snapshotting "rows 5–17" of a view into a detached list re-imports precisely the fragility this model exists to remove: re-sort, refresh, or edit a filter, and the captured list silently means something else.

#### Pick from a view

Keep the pointing gesture; change what it produces. With a formula open — a scratch line in a block, or any formula editor — clicking a column in a frame view inserts a reference to it. Gesture in, readable declaration out, rendered through the dual-representation formula bar. Clicking a column and watching `` `ledger`.`amount`.sum() `` appear as text is how a spreadsheet user learns the expression language without a tutorial, from the one gesture they already trust.

- **No sub-ranges.** Positional selections — twelve scattered cells, rows 5–17 — do not compile to anything. **The way to reference a subset is to add a filter to the frame's Wrangle chain**, which is the same rule as "a series is a query, never a positional capture," now applied to the gesture as well as the object. Users get one answer to "how do I take a subset," and it is the one that survives a re-sort.
- **Insert the bare reference; aggregate as a separate pick.** A click inserts `` `ledger`.`amount` ``, not `.sum()` — auto-aggregating guesses, and guesses wrong for mean or max. A block line can hold a series, so the bare reference is already a valid result. The aggregate comes from the selection-aware ribbon (Sum / Mean / Min / Max) or `Alt+=`, appending the method explicitly.
- **Filters and sorts are lineage.** There is no second, non-propagating View chain. A header sort writes a trailing Wrangle sort, and filters are authored in Wrangle; descendants therefore see the same frame the user sees. Someone who wants disposable exploration branches a tab and names it accordingly.
- **A tab needs no special handling** — a tab *is* a pass-through child frame with its own Wrangle chain, and grouped results and joins are already first-class objects with IDs, so referencing what you see is just referencing an object.

**The blocker is the mode system, not the gesture.** For a click on a frame to insert into a formula rather than move focus, there has to be a real *reference-picking* mode: while active, clicks anywhere insert references and only Esc or Enter leaves it. That is Excel's mid-formula behavior, which users already know. It is a fourth state beside navigate / edit / canvas in the keyboard canon — the same prerequisite this design has now hit three separate times, and the actual critical path.

Make the picking visible with machinery that already exists: highlight the picked column and draw a transient cord from the editing line to the source, in the same visual vocabulary as lineage. On commit the transient cord becomes a real lineage cord, so the gesture visibly creates the dependency.

#### No implicit positional broadcast

Once a series is first class, a series and a frame column will meet inside one expression, and aligning two independent objects by ordinal is the single silent-wrongness failure mode the row-identity model exists to prevent.

- **Legal**: aggregate position (`` `rates`.mean() ``), membership (`` `x`.is_in(`categories`) ``), and explicit expansion.
- **Rejected at parse time**: elementwise pairing of a series with a column, with an error pointing at expand-or-join.

#### Function object

The missing piece behind "a formula type" is not a formula-valued cell — it is a **named calculation applied in many places**: `FunctionObject { name, params, body: Expr }`. Effectively LAMBDA, resolved through machinery that already exists.

- Another node in the dependency graph, so the existing cycle check rejects recursion for free.
- Constants are either captured `ValueObject` references (assumptions stay editable, inspectable, and in lineage) or lifted to explicit params — the hardcoded number becomes an object instead of a buried literal.
- Gets a canvas card with cords to every value, column, and series it references; renames stay safe.
- Keep it first-order for now: no closures, no functions as arguments. The compiler surface stays as small as it is today.

#### Expansion is the for-loop primitive

Dragging a calc across a set of inputs is not `map(f, list)` — it is a **cross join against a spine, followed by an ordinary calculated column**. Add `Expand` as a derived-frame mode: cartesian product over series and/or frames, stable output-column IDs, one more `LazyFrame` plan alongside the existing join, group, project, and filter modes.

What collapses into it:

- Drag a formula across categories → `expand(categories)` + calculated column
- Two-variable sensitivity grid → `expand(series A, series B)` + calculated column + long→wide pivot for presentation
- Depreciation/amortization schedule → `expand(assets, time spine)` + calculated column
- Scenario × period modeling → `expand(scenarios, periods)`

Four backlog workflows, one primitive plus the already-planned pivot. It also makes the loop's *shape* an inspectable canvas object rather than an invisible consequence of how far someone dragged.

#### Build order

This is the historical dependency order that produced the landed surfaces,
not the current priority list. Named functions are the remaining item in this
sequence; the Databoard and broader product boundaries are tracked above.

Computed values → widened qualified resolution → formula block → series (derived + literal) → pick-from-view → `Expand` derived mode → named functions (as block members) → long↔wide pivot.

Computed values go first: smallest change, no new object kind. Widened resolution is the small shared dependency underneath both cross-object aggregates and block members. The formula block then delivers case 4 and doubles as the empty-canvas on-ramp. Series and `Expand` together deliver case 2; functions make a calc reusable rather than retyped; the pivot is presentation for case 3. Each step is independently useful, so the sequence can stop anywhere without leaving a half-built primitive.

The dependency worth flagging was reference-picking mode: pick-from-view could not be implemented as another isolated gesture. The active-editor registry and pointing path that landed from this work remain the shared boundary for formula clicks; future formula surfaces must extend it rather than creating their own drafts.

### The scratch drawer (2026-08-13)

The formula block above is the object; this section is how it is presented, how a line's result renders, and how a line that touches two frames evaluates. Three decisions made in the scratch-sheet session: scratch is **live**, the pad is a **drawer rather than a modal**, and promotion is an **ID move**. Numi remains the reference for the interaction shape — a linear pad with a per-line result gutter — and for nothing else; the ease came from the display, not the input language.

#### A drawer, not a popup

Every document has one well-known scratch block, presented in a slide-over drawer with a global shortcut and a toolbar button. Not a modal. The workflow being served is *glance at the frame, type a line, glance again* — a popup that covers the canvas destroys exactly the thing it exists for. The drawer stays open while the user clicks around the document, which is also what lets pick-from-view compose with it: with a scratch line focused, clicking a column inserts the reference mid-line.

- **The drawer is a view of a block, not a new kind.** The scratch pad is an ordinary formula block whose home happens to be the drawer instead of the canvas. Same namespace rules (`` `scratch`.margin `` from anywhere), same members, same machinery; drawer versus canvas is furniture.
- **Each line is the real formula editor** — the same type-aware autocomplete endpoint, the same execute shortcut, the same error card as a column formula. No scratchpad-lite editor to build or explain.
- **Auto-name in the gutter, dimmed.** Lines land as `line_4` (or an alias derived from the expression — the frontend already turns `` `debit`.sum() `` into "Debit Sum"); click to rename, or write `margin = ...` in the line itself. Forcing a name up front kills the gesture.
- **Live by default, never frozen as a prerequisite.** A scratch line is a view over the same dependency graph as everything else: upstream edits recompute it, including through other lines. "Scratch" means *disposable*, never *stale* — a recorded number that silently appears merely because the source was live is the spreadsheet failure this model exists to prevent. An explicit historical capture may still be requested and must announce itself as frozen; it is a separate user decision, not an evaluation mode the frame forces on the line.

#### Results render by rank

A line evaluates in a `select` context, and the shape of what comes back decides its presentation — the same rule as "a column in scalar position must reduce," now read as display guidance instead of only as an error:

- **Scalar** (height 1): inline in the gutter, right-aligned, formatted by dtype. The Numi picture.
- **Series** (height n): a collapsed chip — `1,204 rows · Number`, with a small sparkline when numeric — expanding in place to a capped preview (~50 values) with a "show more" that pages.
- **Frame** (a frame-valued expression): a chip expanding to a small paged grid, read through the same paged boundary as every other frame view.

Expansion is presentation only. Looking inside a series result creates no object; a line's result becomes referenceable data by being *named and referenced*, not by being unfolded.

#### Promotion is re-parenting

A line gets a promote affordance that moves it out of scratch — into a named canvas block (`Assumptions`). Onto the canvas as a standalone value card is no longer one of the destinations; see [The block is the only home for a scalar](#the-block-is-the-only-home-for-a-scalar-2026-08-13). Because every reference stores the member's stable ID and names are aliases, promotion is a parent change on the same object: nothing that referenced `` `scratch`.margin `` breaks or even changes, it simply re-renders as `` `assumptions`.margin ``. Demotion is the same move backward. Deleting a referenced line refuses with the existing referenced-value machinery, same as deleting a referenced canvas value.

#### Mixed-frame lines scalarize at the frontier

`` `orders`.`amount`.sum() * `rates`.`fx`.last() `` is a reasonable scratch line and has no single frame to run in. Polars cannot evaluate it as one expression; the compiler splits it instead: each **maximal single-frame subtree** evaluates in its own `select` context, any subtree whose result meets another frame's data must reduce to height 1 (error names the subtree that failed to aggregate, in the same voice as the no-broadcast rule), the scalar results substitute in as literals, and the residual pure-scalar expression evaluates on a unit frame. A line rooted entirely in one frame may still return a series; it is only at the *meeting point* of two frames that everything must already be scalar. Elementwise pairing of raw columns across frames stays rejected at parse time — this is the expression-evaluation face of "no implicit positional broadcast," not a relaxation of it.

#### `today()` is allowed, and live

The catalog defers nondeterministic functions, but "days until the close date" is a signature scratch use and scratch is now defined as live. Resolve the tension by policy rather than exclusion: `today()` is permitted and evaluates at compute time, consistent with lines being views. Reproducibility gets its lever later as a document-level *as-of date* override, which also serves the close-package workflow. (The accept-time capture rule under [Scale and undo](#scale-and-undo-one-package) is about volatile results entering the *event log* — data entry — and is unchanged by this; a formula result never enters the log at all.)

### The block is the only home for a scalar (2026-08-13)

The canvas no longer makes a value, a result, or a list. Those three were each a card holding one number, and a page of scratch arithmetic became a page of cards — the density problem the formula block was introduced to solve, still being created by the menu next to it. So the menu stopped offering them: **right-click, ⌘J, the rail, and double-clicking the canvas all make a block**, and a constant is written as a line of one, `rate = 0.08`, exactly the way a calculation is.

- **Enforced in the model, not just hidden in the menu.** `AddValue`, `AddResult`, `AddSeries` and `ImportSeriesFromFile` refuse without a container, and `MoveIntoContainer(null)` refuses to put one back on the canvas. A rule that only the toolbar knows is not a rule: the MCP server would still scatter cards, and so would a replayed event from an older build.
- **A container is the exception.** Inside one, a value is part of an arrangement somebody laid out rather than a card that drifted loose — the dashboard case. The container's own buttons still make all three, which is now the only way to.
- **The kinds stay in the model.** `ValueObject`, `ResultObject` and `SeriesObject` are what a saved `.fw` already contains, and what a container holds. This removes a way to *create* them, not a way to read them.
- **The MCP server writes blocks.** `create_value` is gone, replaced by `create_block` and `set_block_source` — which is a better surface for an agent anyway: a whole worked page in one call rather than a card per number.
- **The demo document says so too.** Its one assumption, `Tax rate = 5%`, is a line of a block called `Assumptions` rather than a lone card, so the first thing anyone opens demonstrates where a constant goes.
- **A line loses the card, not the notation.** See [`4.25%` is a literal](#425-is-a-literal--is-the-remainder): a constant on a line keeps being money or a rate, because that was always a type rather than decoration.

#### Many blocks, one of them well known

⌘J goes to a block named **`Scratchwork`**, always, and conjures it on first press. It used to go to the selected block, or the newest — which made "where did I put that" a question the one always-reachable surface should never raise. Every other block is somewhere you go on purpose by clicking it; this is the one you drop into without looking. The toggle back is unchanged.

The rest are ordinary canvas blocks, named `Block 1`, `Block 2`, … and edited by clicking them. **They read each other by name**: `` `Assumptions`.rate `` from any block, any column formula, anywhere. Two things had to change for the spelling in [Formula block](#formula-block-scratchpad) to actually work:

- **The parser takes `` `block`.line `` back off the lexer.** The lexer stops a dotted reference at the first name that is not backticked, because everywhere else a bare name after a dot is a method or a namespace — `` `amount`.round(2) ``, `` `amount`.str.zfill(2) ``. That rule is right for a value and wrong for a block, because a block has no methods at all. So the parser handles the one case the lexer cannot: head names a block, no `(` follows, no sibling line has that name.
- **A rename rewrites the text that named it.** A reference is an id everywhere else, so renaming a frame re-renders the formulas that read it and nothing is stored. A block line is the exception — its text is kept as typed, because that text is what the author is looking at — so `RenameObject` now carries the affected blocks' rewritten lines. Which lines those are is settled by rendering each one against the document and against a renamed copy, and keeping the ones that came out differently. Without this, renaming `Block 1` to `Assumptions` left every other block spelling the old name, and the next keystroke in one of them broke a line for no visible reason.

#### `4.25%` is a literal, `%%` is the remainder

A constant living on a block line meant every constant lost its notation, because the only literal a formula had was a bare number. That is the wrong trade: `4.25%` is how a rate is written on paper and in every spreadsheet, and a language that makes somebody type `0.0425` instead is asking them to do the conversion the machine is for.

So the sign binds to a number the way a duration's `d` does. `Expr::Percentage` holds the fraction — `4.25%` is `0.0425` — and carries [`DataType::Percentage`], which is a *type* this document has always had and not a formatting flag. Arithmetic is on the fraction throughout; only reading it back asks. `rate + fee` is a percentage because both sides are; `20000 * rate` is a number, because that is what the multiplication produced; `[5%, 10%]` stays percentages, while `[5%, 1]` drops to numbers under the existing promotion tree.

- **The remainder keeps a spelling, and gains a better one.** `%` is still the remainder everywhere it is not stuck to the end of a number, so `total % 3` and `10 % 3` are unchanged. `%%` says it with no space needed. The one spelling that lost its old meaning is `10%3` — no space, no doubling — and rather than pick for the author the lexer refuses it by name.
- **A percentage inside a sentence writes itself.** `` "up " + 4.25% `` is `up 4.25%`, and `` "costs " + `Price` `` is `costs $5`. `as_text` renders through this document's rules, and money and percentages are exactly the case where Polars' `cast(String)` says something the document itself would never print.
- **The gutter believes the expression over Polars.** Polars knows a float; it has no place for the difference between `0.0425`, `4.25%`, and `$4.25`. So where the expression's own declared type says money or a percentage and Polars agrees it is a number at all, the expression wins. Only in that direction — an expression that has been through `cast("string")` shows the text.
- **`{:.1}%` is gone.** Percentage formatting was fixed at one decimal place, which showed `4.25%` as `4.2%`: a digit somebody typed, dropped on the way to the screen.

#### How a number is written travels through the arithmetic

Excel does not do this: a spreadsheet formats a *cell*, so `=B2*C2` inherits whatever the cell it lands in was last set to and the notation is lost the moment a number moves. Here notation is a **type on the value**, so it can propagate — and the rule that makes propagation come out right every time is one sentence.

**Money is a dimension. A percentage is a way of writing a pure ratio.**

That settles every case without a frame of special pleading. Applying a rate to an amount spends the ratio and keeps the amount's kind; dividing money by money cancels the dimension and leaves the ratio.

| | | |
|---|---|---|
| `$5 × 3` | `$15` | money survives a plain multiplier |
| `$100 × 5%` | `$5` | the ratio is spent, the dimension stays |
| `100 × 5%` | `5` | nothing had a dimension |
| `50% × 50%` | `25%` | ratio × ratio is a ratio |
| `margin / revenue` | `38%` | **money ÷ money cancels to the ratio** |
| `$10 / 2` | `$5` | sharing money out leaves money |
| `$5 + $3` | `$8` | |
| `$5 × $5` | `25` | money² is not money |
| `$100 // 3` | `$33` | a remainder keeps the left side's dimension |
| `$2 ** 3` | `8` | a power leaves every dimension behind |

`margin / revenue → 38%` is the line the whole idea exists for.

**Two ties the dimensions cannot break**, because both sides are dimensionless and only the writing is in question. Both go to the reading that cannot come out absurd:

- `rate * 12` is a **number**. Dimensionally either would do, but the commonest percentage line anyone writes is applying a rate to an amount, and `20000 * 4.25%` announcing `85000%` is far worse than `4.25% * 12` answering `0.51`.
- `rate / 12` **is** a rate, because sharing a rate out over twelve months leaves a rate — an annual figure over the months of the year is the line being served, and nothing about it can look absurd.

**Two literals, so a block can hold a figure at all.** `$250000` and `4.25%` are lexed as `Expr::Money` and `Expr::Percentage` — the plain number plus the fact that it was written with a mark on it. Without them, money could only enter a formula from a column or a value card, and a block would be a worse place to keep a figure than the cell it replaced.

**Overridable, and the override breaks the chain.** `.show("money")`, `.show("percent")`, `.show("plain")` — said out loud, believed over whatever the arithmetic worked out, and everything above reads the `.show` rather than the node under it. Deliberately not one of `cast`'s targets: `cast` converts and changes what a value *is*, while this changes nothing about the value and everything about how it is written. `"plain"` is the escape hatch for exactly the complaint that motivates having one — technically money, not wanted as money.

**Where the gutter gets it from.** Polars knows a float and has no place for the difference between `0.0425`, `4.25%`, and `$4.25`. So where the expression's declared type says money or a rate and Polars agrees it is a number, the expression wins; where Polars says text, the answer has been through a `cast` or a `format` and the text wins. Non-numeric types propagate nothing — a date, a duration, or a piece of text in arithmetic answers "cannot be told from here" rather than guessing.

**Not the same question as a list.** `promote_types` asks what type could hold *both* of two values, which is right for `[5%, 1]` — two things that agree on nothing but the number, so the list is numbers. It is the wrong question for `$100 * 5%`, which is not "the type that holds money and a rate". It is five dollars.

### Function catalog roadmap (Excel parity, 2026-08-13)

The question is what function surface makes this credible against Excel, and how each piece is acquired. The stance first: **parity is measured in jobs, not function names.** Excel ships ~500 functions; published usage studies put the overwhelming majority of real-world formulas inside the top ~25 (SUM, IF, AVERAGE, COUNT/COUNTA, ROUND, the IF-family aggregates, MIN/MAX, CONCAT/TEXT, VLOOKUP, date arithmetic). Every one of those maps to something already in or near the catalog — VLOOKUP is relationships and joins, the SUMIF family is `.filter(...)` composed with an aggregate, TEXT is `format`/`strftime`. So the work is not porting a function list; it is unlocking what the engine already has, curating it into the supported catalog, and teaching Excel vocabulary to find it.

That last clause is the registry the syntax section already calls for: catalog entries carry **searchable aliases**, so typing `average` surfaces `.mean()`, `sumif` surfaces the `.filter(...).sum()` shape as a snippet, and `vlookup` points at relationships. The alias layer is how an Excel user finds the Polars idiom without the catalog maintaining two function languages — there is no `AVERAGE()` to keep in sync with `.mean()`, only a signpost.

#### Acquisition mechanisms, cheapest first

1. **Promote from the generated surface.** Most of Polars 0.55's `Expr` methods are already bound by the generator; promoting one into the supported catalog is an availability flag, autocomplete exposure, and a behavior test. The scratch pad and computed values create the scalar execution context that the deferred *aggregate* category was waiting on, so its unlock is this mechanism, not new code.
2. **Hand-written compile arms** for methods the generator skips over options-structs and enums (`quantile`'s interpolation, `rank`'s method, `ewm_mean`'s decay). Hand-written arms already take precedence over generated ones; the path exists.
3. **Expansion functions**: a catalog ID whose compile arm emits a *composed* Polars tree. Closed-form financial math (`PMT`, `PV`, `FV`, `NPER`, `NPV`) is arithmetic and belongs here — the saved tree stores the stable function ID, so the expansion is invisible to persistence, exactly as the registry design intends.
4. **Native Rust scalar functions** for what no Polars expression can be: iterative solvers (`IRR`, `XIRR`, `RATE` — a Newton iteration over a collected series, ~hundreds of lines of owned code, not a second engine) and business-day arithmetic (`networkdays`/`workday` equivalents, holiday list as a series argument). These evaluate in the scalar context, outside the expression compiler.

The decision already on record stands: no second formula language, no embedded evaluator. Everything above is either Polars or plain Rust the engine owns.

#### Priority order

This is the acquisition order recorded in the 2026-08-13 catalog discussion.
Percent literals and most of the aggregate, string, cumulative, and rolling
surface have since landed. Financial solvers, business-day arithmetic,
`str.to_date`, and specialized option-heavy methods remain the material gaps.

1. **Aggregates and statistics into scalar context** — `median`, `std`, `var`, `quantile`, `first`, `last`, `n_unique`, `mode`, `product`, `arg_min`/`arg_max`, root-level `corr`/`cov`. Mechanism 1 almost throughout. "What's the p90 of this column" is the quick-test gesture the scratch pad exists for.
2. **`.filter(...)` on expressions** — one orthogonal method that replaces the entire SUMIF/SUMIFS/COUNTIF/AVERAGEIF/MAXIFS family: `` `amount`.filter(`region` == "West").sum() ``. The single highest-leverage addition on this list, and the aliases `sumif`/`countif`/`averageif` all signpost to it.
3. **Strings** — the catalog note under Formula UX already names the gap; the working set is `split`, `replace`/`replace_all`, `strip_chars`, `len_chars`, `slice`, `starts_with`/`ends_with`, `extract`, `zfill`, root-level `concat_str` and `format`, plus `str.to_date` for the period-string case. Mechanisms 1–2.
4. **Cumulative and change** — `cum_sum`, `cum_count`, `diff`, `pct_change`, `rank`, `rolling_std`, `rolling_median`, `ewm_mean`. Running totals and period-over-period are the §12.3/§12.4 promise.
5. **Financial** — closed forms by mechanism 3, solvers by mechanism 4. Core to the finance wedge; nothing upstream of Polars provides these.
6. **Business dates** — mechanism 4, riding on the existing date/duration literals.
7. **Percent literals** — `5%` lexes as `0.05`; `` `price` + 5% `` sugars to `` `price` * 1.05 ``. Lexer work, not catalog work, and the duration-literal machinery is the template. This is most of what made Numi's *arithmetic* feel effortless, acquired without any natural-language layer. Unit conversion is deferred (real scope, low relevance to tabular work); live currency is out on the determinism boundary — a hand-maintained rates frame and a relationship is the honest version.

Ordering within the list is leverage over effort: items 1–4 are mostly curation of an engine surface that already exists, 5–6 are the first genuinely new evaluation code, 7 is a parser nicety that can ride along with any lexer touch. The rule from the syntax section holds throughout: the catalog is a discoverability surface over one language, unsupported things fail visibly, and nothing autocompletes as executable before it is.

### Target workflows (finance/accounting wedge)

Acceptance-scenario style, roughly in build order:

1. **Bank/subledger reconciliation** — tolerance and one-to-many matching, persistent match state with provenance, aging exception views. No recurrence needed; best first target.
2. **Budget vs. actuals** — append/refresh import, join on (account, dept, period), variance columns, anti-join exception view.
3. **Scenarios and sensitivity** — named assumption bundles (Base/Upside/Downside), two-variable sensitivity grid object, goal seek. Assumptions-as-objects is the native advantage over Excel here.
4. **Driver-based rolling forecast** — time spine with fiscal calendars (4-4-5, offset year-ends), actual/forecast switch on a close-date assumption, prior-period drivers.
5. **Depreciation/amortization schedules** — cross-join expansion against the time spine, roll-forwards, long-to-wide triangle presentation.
6. **Close package** — debits=credits validation constraints that gate export, templated ERP CSV export, accounting number formats.
7. **Three-statement model / debt schedule** — recurrence plus declared iterative solve (interest-on-average-balance circularity). Deepest moat; last.

### Workflow primitive backlog

The original ranked list became misleading as formula blocks, series,
expansion, recurrence, pivot/unpivot, and basic number/date generators landed.
The outstanding analytical primitives are now:

1. append imports and reusable export templates;
2. fiscal time spines and business calendars;
3. tolerance, one-to-many matching, and persistent reviewed match objects;
4. scenario sets, sensitivity views, and goal seek;
5. named functions as block members;
6. validation constraints, exact decimal/currency semantics, and accounting
   presentation;
7. richer fill inference and string-to-date conversion; and
8. iterative or simultaneous solve, deferred until the workflows above are
   complete.

Product-boundary work such as XLSX interchange, connectors, complete-data
plots, reporting, checkpoints, and the Databoard is tracked by the replacement
threshold above rather than disguised as an engine primitive.

### Scale and undo (one package)

Target: tens of millions of rows on moderate hardware.

- **Immutable base artifacts + edit overlay**: imports become content-addressed Parquet/Arrow artifacts; all edits are overrides (generalized `SetCellOverride`); document state = fold(event log) + base artifacts. *Artifacts and content addressing are in; the edit overlay is not.*
- **Undo = the log**: drop/inverse the last operation; no document snapshots. Checkpoints become cached materializations. *Both halves landed — cached materializations as `Materialization`, undo as inverse operations. What remains is the simplification single-writer allows; see [§28.1](#281-undo-and-redo).*
- **Identity-on-write**: base rows get implicit identity (artifact + ordinal); UUIDs minted only for edited/inserted/referenced rows. Per-row UUIDs at 10M rows do not scale.
- **Paged boundary — landed for frame display:** imported and derived artifact-backed frames expose metadata and pages to the virtualized grid. Complete-data plots, streaming exports, and several global operations remain; the replacement threshold treats any silent partial result as a correctness bug.
- **Volatile functions**: capture `dt.now()`-style results into the event at accept time so replay stays deterministic (cheap insurance; alternative is documenting the sharp edge).

### Formula UX

- **Click-to-build relative references**: pointing at a cell N rows up in *another* column compiles to `.shift(N)`; pointing into the *same* column being defined declares a recurrence. One gesture, two artifacts.
- **Lag/recurrence requires declared ordering** (ideally the time spine). **Landed:** a calculated column containing `.shift(...)`, a cumulative method, a frame-length sequence, or `recur(...)` is refused unless a Sort precedes it in the Wrangle chain, so a later header sort cannot silently reinterpret an accepted formula. Click-to-build creates the same visible Sort and calculation steps and goes through the same guard. Pointing to an earlier row of a different column inserts `.shift(N)`; pointing to an earlier result in the target column opens *Calculate down rows* with separate *First row* and *Each next row* fields. The latter field receives `previous()` when the earlier result is clicked. Optional *Restart for each* stores stable column references through `restart_by=[...]` while the ordinary Wrangle step remains the single persisted authoring surface.
- **Recurrence execution tiers — landed.** Running total/count/min/max continue to use Polars cumulative expressions. General `recur(first, next, restart_by=[...])` calculations deliberately collect the explicitly ordered input and evaluate the native fold one row at a time, with independent previous-result state per restart key. This path is slower by contract and correct by construction; materialization remains the opt-in answer when the result is expensive to revisit.
- **Dual-representation formula bar**: show the pointing gesture and the generated Polars text side by side; that is how users learn the language.
- **References identify themselves in both places.** While any reference-bearing editor is active, each distinct resolved reference gets a transient colour shared by its text and the canvas object, column, or demonstrated row-relative cell it names. Repeated uses keep the same colour; typed, pasted, autocompleted, and clicked references are decorated from the draft rather than from click history. **Landed for formulas:** the bar also projects those references into clickable chips such as *Revenue · previous row*; choosing one selects its exact syntax for replacement. The same rule should extend beyond formulas to joins, chart encodings, relationships, and other editors that point at existing document objects.
- **A calculated cell is an address for its column declaration.** Double-clicking any calculated cell edits the one formula applied to the column and uses that row only as the temporary pointing anchor. The formula bar says that the edit applies to all rows; a one-off calculation goes to Scratchwork instead of becoming a hidden cell exception.
- **Literal data stays literal.** A document-owned cell is edited in the grid with the spreadsheet gestures people already know: double-click or F2 preserves its value, typing replaces it, Enter and Tab commit and move, Delete clears, paste writes a rectangle, and Fill down/right copies values through an editable selection. These actions never manufacture formulas merely because the formula system exists. Source-backed cells explain why they cannot take the edit; calculated cells route to their one column declaration.
- **Range editing before range formulas. Landed.** Rectangular selection supports copy, paste, clear, and literal fill; the top bar edits an owned literal cell directly, routes a calculated cell to its one Wrangle declaration, and explains a source-backed refusal beside the ownership remedies. Multi-cell selections report ephemeral count/sum/average, while a whole-column selection can add Sum/Mean/Count to the frame's durable cross-column profile footer. Positional formula ranges such as `A2:A20` remain deliberately absent: a durable calculation names a column, and a subset is declared by a Filter step rather than by coordinates that change under sort or refresh.
- **Type-aware autocomplete — landed.** Completion is resolved in core at the cursor and filtered by the receiver's inferred dtype. Strings, categories, integers, numbers, dates, and booleans no longer share one misleading method list; namespace prefixes retain an honest empty state when the typed receiver cannot use them.
- **String namespace expansion — mostly landed.** The supported catalog now includes splitting, replacement, stripping, character length, slicing, prefix/suffix checks, extraction, and zero filling for strings, with the applicable subset exposed for categories. `str.to_date` remains the important gap: today a `"2025-12"` period string still cannot become a date without an earlier import transformation.
- **Generated lists and ordered column fill — landed for numbers and dates.** `sequence(start, stop, step)` produces numeric or date lists in Scratchwork. A row-count-bound calculation is the primary authoring model: typing `=sequence(...)` at a cell or column promotes it to the column's visible Wrangle declaration, while *Fill number series…* and the inferred-series gesture are pointing surfaces over that same formula. Numeric fills save `sequence(... frame.len() ...)`; date fills save `sequence(start, periods=frame.len(), step=1d)` (or a calendar step such as `1mo`). `frame.n_rows()` is accepted as the dataframe-style alias for `frame.len()`. Both add an explicit Sort when the chain lacks one, fill the current frame rather than creating a second card, and remain live as its row count changes. Richer fiscal time-spine construction remains future work.

### Formula surfaces: the bar, the pick, and the cell (2026-08-14)

Five decisions from the formula-surfaces session shared one structural dependency: an **active-editor registry** recording *which editor is live, what its draft says, and how to commit it*. That registry, the formula bar, and formula pointing have landed. The subsections below retain the interaction contract and the reasoning that prevents future formula surfaces from becoming independent drafts.

#### The formula bar

One bar at the top of the window. When a formula editor is active anywhere — a column formula, a block line, or a legacy cell override — the bar is a second view of *that editor's* draft: same state, same commit, so there is nothing to synchronize and no way for the two to disagree. With a cell selected, it instead shows that cell: an owned literal is directly editable, a calculated value displays and opens its shared Wrangle declaration, and a source-backed value is visibly read-only with the reason and recovery path. Typing `=` on that cell hands the cursor to a fresh line of `Scratchwork`; with neither an editor nor a cell active, the bar already edits that same destination. Type `4100 * 1.2` and get `line_7`; type `margin = ...` and the line is named — exactly the `x = 10` rule every block line already follows.

- **No ephemeral tier.** The tempting refinement — unnamed bar entries are temporary, named ones persist — is a second lifetime for the same gesture, an invisible mode. Everything the bar produces lands in `Scratchwork`, where a line is already disposable and the drawer is the browsing surface. Delete is cheap; a vanishing result is not.
- **Unify the editors first — landed.** A bar mirroring "whatever is being edited" could not sit on independent commit models; the bar and the card would fight over the draft. The active-editor registry now supplies the shared draft and commit boundary.
- **The implementation sequence was two steps.** Bar-to-Scratchwork established the destination first; mirroring the active editor followed through the registry. The bar remains the home for the richer dual representation described under Formula UX.

#### A new column where you are

Right-click a cell or a header: *insert column here*. It arrives blank and untyped, and becomes what its first content makes it — pasted values (a document-owned frame already stores cells in the `.fw`), a formula (`SetColumnFormula` already converts a column in place), or a generator spec (see [Data entry and column fill](#data-entry-and-column-fill)).

- On an artifact-backed frame, a *calculated* column is legal anywhere — it is a plan layer, not data. A *data* column offers the two landed ownership actions (freeze a copy / take ownership) rather than any positional override layer; the reasoning recorded under the ownership entry stands.
- Mechanical gap: `AddComputedColumn` always appends. It needs `after_column_id`, or the gesture decomposes into `AddColumn` + `SetColumnFormula`.

#### Time-series formulas enter at the cell and land on the column

The gesture starts at a cell — right-click, *formula here*, then point at the cells it reads — because that is where a spreadsheet user's hands already are. The artifact it creates is a **column-level declaration**: pointing N rows up in another column compiles to `.shift(N)`; pointing into the column being defined declares a recurrence (both already specified under Formula UX). What the gesture must never create is a formula stored *in* the cell that spreads over time — per-cell time formulas are the Excel model this document exists to replace, and `Cell.override_formula` stays what it is: a one-row exception, not a time-series mechanism. Rows at the edge that cannot have a value show null, which `.shift` provides for free.

- **Enforce the ordering guard before promoting the functions.** `plan_sorts` already answers whether a sort exists in a frame's lineage; wire it to the `.shift` compile before lag/lead get any more catalog exposure. The guard *is* the feature — without it, promoting `lag` just ships the silent re-sort bug with better discoverability.
- **Recurrence is unchanged**: a separate declared computation type (order by, partition by, starting value, formula), never a circular ordinary formula, with the execution tiers already recorded. Speed is not the concern it appears to be: linear recurrences vectorize via the cumulative closed forms, general ones are a native Rust fold, and a fold over ten million rows is milliseconds. Recurrences are only slow when they are a million cell formulas.

#### The catalog surfaces its own documentation

Every `FormulaFunction` already crosses the wire carrying signature, description, aliases, category, arity, return type, and null behavior — and the UI renders one line of it. Exposing formulas better is therefore mostly rendering work over data already shipped: a help panel showing the highlighted suggestion's full signature and description; parameter hints while the cursor sits inside a call (completion already types the expression at the cursor, so the context exists); and the generated catalog reference browsable in-app rather than only as markdown in `docs/`. One sequencing rule: fill the string-namespace gaps before polishing help around them — help UI over a catalog with holes teaches the holes.

#### Order of work

The historical sequence was the shift guard and help surface, in-place column insertion, the shared editor registry, the formula bar and pick-from-view, then click-to-build and recurrence. The registry, bar, pointing, ordering guard, and recurrence have landed. Remaining help and discoverability work belongs under safe adoption rather than reopening the editor architecture.

### Data entry and column fill

Excel's autofill is the other half of "drag the formula": type `1`, `2`, drag, get a sequence; type `Jan-2025`, drag, get a calendar; type two examples of a reformatted name and Flash Fill infers the rest. Basic number and date generators, row-count-bound calculated fills, and simple series inference have landed. Rich fiscal calendars, blank-fill, cycles, synthetic data, and string-expression synthesis remain the work described below.

The translation is direct and makes the feature *better* than the original: **Excel fills a range, FrameWork fills a column, and a column already has a declaration slot.** A calculated column holds a formula; a filled column should hold a **generator spec** — a small typed declaration, not N materialized cells. That single decision buys everything the rest of this document already relies on: it is one event rather than ten million overrides, it survives refresh and append, it renders in lineage, it is editable after the fact, and it is writable by the AI panel.

#### Generator specs (the declaration)

A new derived-column mode alongside formula and literal, `Generate(spec)`, evaluated lazily into the column's expression plan. The starting set, all of which cover a named Excel behavior:

- **Sequence** — `{ start, step, dtype }`. Covers `1, 2, 3…` and `0.5, 1.0, 1.5…`. The row-number case is `start = 1, step = 1`.
- **Date/time spine** — `{ start, step: interval, calendar? }` with month-end, quarter, business-day, and fiscal variants. This is the same time-spine primitive already ranked in the backlog, applied to a column rather than a frame; build it once.
- **Cycle** — repeat a literal list or a series down the column (`Q1, Q2, Q3, Q4, Q1…`). Absorbs Excel's built-in weekday/month lists without hardcoding locale frames: they are just series.
- **Constant** — a value or a reference to a `ValueObject`, so a filled-down assumption stays an assumption.
- **Blank-fill** — fill only the nulls, leaving existing entries alone. The common real-world gesture on a partially entered column.
- **Random/synthetic** — a distribution plus **a seed captured into the event at accept time**, under the same rule volatile functions already follow. Unseeded randomness makes replay non-deterministic and quietly breaks undo-as-log.

Two spec-level rules:

- **A generator is a query, never a positional capture** — the same rule as series and pick-from-view, now applied to entry. Filling "the twelve rows I selected" does not compile. To fill a subset, **filter the view and fill the column**, and the predicate is materialized into the spec at accept time exactly as pick-from-view does it.
- **Position-dependent generators require declared ordering.** A sequence or date spine is meaningless under an undeclared sort — this is the `shift`/recurrence problem verbatim, so it gets the same answer: refuse to create one without a bound sort column, and point at the time spine. A row-order-dependent fill that silently re-means itself on re-sort is precisely the silent wrongness the row-identity model exists to prevent.

#### Inference (the "it just knew" part)

Excel's magic is that you type two cells and it guesses. Keep the gesture; change what it produces:

- **Infer from the first non-null entries in the column** — a handful of values in, a proposed spec out. `1, 2` → sequence; `2025-01-31, 2025-02-28` → month-end spine; `Q1, Q2` → cycle over a detected list.
- **Propose, never silently apply.** The inferred spec appears as an editable declaration in the formula bar's dual representation, with a preview of the next several values, and is committed by Enter. Excel's autofill is unreviewable and guesses wrong constantly; the whole value of having a declaration is that the guess is legible before it lands.
- **Rank candidates and show the runner-up.** `1, 2` is arithmetic or geometric; `Jan, Apr` is quarterly or every-third-month. One click to switch beats retyping.
- **Flash Fill's analogue is formula synthesis, not a separate engine.** Given a few hand-typed outputs beside an existing column, infer a *string expression* — `str.slice`, `str.split`, `str.to_uppercase`, `str.to_date` — and propose it as an ordinary calculated column. The result is inspectable and refreshes with the source, which Excel's Flash Fill famously does not. This depends on the string-namespace gaps already noted under [Formula UX](#formula-ux); those functions have to exist before anything can synthesize them. Beyond simple slicing and casing, hand the job to the AI panel rather than growing a synthesis engine — it emits the same typed, undoable column operation.

#### Entry ergonomics

Small things that make hand entry tolerable at all, and are mostly free once the mode system exists:

- **Drag-fill and Ctrl+D already have a meaning** in the keyboard canon — "promote to column formula." Extend rather than contend: if the seed entries compile to a formula, promote; if they compile to a generator spec, generate; if to neither, fall back to repeating the literal. One gesture, three outcomes, chosen by what the seed actually is.
- **Type-ahead from the column's existing distinct values** on low-cardinality text columns, which is the entry-side twin of type-aware autocomplete.
- **Validation on entry** — the constraint layer from the close-package workflow, applied as you type rather than at export. A category column with a declared domain rejects a typo at the point it is made.
- **Provenance for hand-entered data.** Hand-typed numbers are the least trustworthy values in any document and today carry no record of where they came from. The formula block's comment lines are the pattern; a filled or hand-entered column should be able to carry the same note, and a generated column should visibly say it was generated — synthetic demo data must never be indistinguishable from imported data.

#### Where it sits

Not on the critical path, and it should not jump ahead of the formula work — but it is unusually cheap once that work lands, because it reuses the time spine, series, materialized-predicate rule, volatile-capture rule, and dual-representation formula bar wholesale. The one genuine prerequisite is shared with everything else: the focus/mode system.

Suggested order: sequence and constant generators (trivial, immediately useful on any new column) → blank-fill → date spine (arrives with the time-spine primitive) → cycle over series (arrives with series) → inference and preview → string-expression synthesis (after the string namespace is filled in) → synthetic/random with captured seed.

### Conditional formatting (final formatting slice) — shipped 2026-08-18

Rules are a subsection of the Format inspector: a line per rule holding its formula and the field it may paint, its stops underneath, and the panel's existing style controls aimed at whichever stop is selected. There is no second set of formatting controls, and no rule-kind picker — what the formula returns is what the rule is.

The slice as built:

- Each rule compiles into one hidden aliased column, all of them batched into a single `with_columns`; the columns are read as style and dropped, never reaching the schema or any downstream frame. They go **above the slice**, so a rule may ask something of the whole column — `x > x.mean()`, the ends of a ramp — and get the column's answer rather than the visible page's. Elementwise rules still let the slice push down into the scan.
- Three readings, checked against the formula's inferred type when the rule is set: **boolean** picks the rows it answers true for, **text** sorts rows into named cases with an optional catch-all, **number** places each row on a ramp. Rewriting a rule's formula re-offers it to the other readings, so retyping a ramp as a question makes it a condition without anyone choosing a kind.
- **Colors are stored once and reflected for dark mode.** A style holds one hex per property, always the light-mode value; the dark counterpart is derived by reflecting lightness in OKLCH about the two themes' papers, keeping hue and chroma. What that preserves is the reason a color was picked — a fill chosen to sit quietly behind text stays quiet, one chosen to shout stays loud, and every palette entry lands within a third of its light-mode contrast against the ink it carries. It reaches the screen as a CSS `light-dark()` pair, so the window follows the system at the moment it changes rather than at the moment React last rendered, and the engine goes on mixing ramp ends in the stored hex without knowing any of it. Reflection is an involution, so a color picked while the window is dark is stored reflected and comes back as the color that was picked.
- **The palette is eight named hues, reused everywhere rather than grown indefinitely.** Fills are those eight soft and eight deepened; text colors are the same eight at two readable weights, plus a near-black and a grey. Eight is about as many as anyone tells apart at a glance, which is why it is both the number of categories a color can distinguish and the number of core swatches on offer. Every fixed swatch names its hue in the tooltip. Arbitrary colors still come from the native color well, and the five most recent custom text and fill colors are kept separately per document in a compact row; custom colors do not become permanent additions to the core palette. Every entry clears WCAG AA against the paper it sits on in both themes, and the deep inks clear AAA.
- **A case list fills itself from the data, and keeps a catch-all.** Nobody types out the six statuses their column already has: the panel asks the engine for the distinct values the formula produces — commonest first, ties broken on the label so the list is the same list for everyone — and hands each one a fill from a fixed sixteen-color palette, two rows of eight with the second row the first deepened. The catch-all stays even when the list covers everything, because the data moves: a live CSV refreshes, a filter opens up, and those rows want a color saying "not one of the named ones" rather than no color, which reads as a rule that stopped working. It is a read (`frame_formula_values`) rather than part of the operation, so the colors are chosen where every other color in the application is chosen and the document stores the plain mapping; refilling after a formula edit keeps the styles of values that survive, drops the ones that no longer occur, and gives newcomers a color nothing on screen is already wearing. Every entry is editable and removable afterwards, and each category may carry any text color and any fill color together; the generated fill is a starting point rather than a decision.
- **The formula computes the answer; the rule only says what the answer looks like.** All three readings work this way, which took two additions to the language to finish. A scale used to be the exception — it carried a *domain*, data-dependent numbers sitting beside the formula rather than in it, and needed a pair of boxes nobody could read. A scale now carries up to two independent color ramps: text and fill. Each has low and high colors at 0 and 1 and may independently add a middle at 0.5, so a red-yellow-green fill can coexist with blue-magenta text without inventing a coupled five-dimensional editor. The formula says where each row lands. `` `Amount`.normalize() `` is a heatmap; `.normalize(0, 100)` pins the ends; `.normalize(center=0)` puts zero at the turn, which is the case a middle exists for; `.clip(...)` first flattens outliers; `` when(`Maxed`).then(100).otherwise(`Amount`).normalize() `` substitutes a value from another column. Not one of those needed a control, and the last two could not be expressed at all before.
- **`normalize()` and chained `when/then` are the two functions that made it possible.** `.normalize()` is where a value sits between two numbers, as a fraction — the column's own smallest and largest by default, two given numbers positionally, or `center=` for a symmetric ramp about a number that matters. Its aggregates ride the same path `x > x.mean()` does, so they see the whole column rather than the page. Chained `when(a).then(x).when(b).then(y).otherwise(z)` is the categorical half: three columns saying whether a day is statutory, a weekend, or neither collapse into one label — *Stat*, *Reg Holiday*, *Work* — and the rule paints that. Nesting each branch inside the last one's `otherwise` said the same thing inside out and got worse with every category.
- Answers arrive resolved as `FrameCellStyle` against stable row ids — on `ComputedFrame` for a frame holding its own rows, on the page for a paged one — so they survive sorting, filtering, and transposition, and duplicated tabs diverge with their own rules.
- Rules apply property-by-property in rule order over direct formatting, later rules winning.
- A rule that cannot run reports itself by id in the panel and is skipped; the frame stays readable, and a plan that will not run with the rule columns is retried without them so a broken rule can never cost somebody their data.
- **Presets** hang off the *Rule* control, offered by the selected column's type: heatmap low-to-high, diverging around zero, above average, top and bottom tenth, negative in red for numbers; highlight true and false for booleans; a color per value for text; weekends, in the future, and older than thirty days for dates; blank cells for anything. Each is an ordinary rule once made — formula on the row, editable like any other — so a preset is a starting point rather than a mode. A Rust test commits every preset formula against a real frame, because a preset the engine will not take is a menu item that puts a red line under a rule nobody typed.
- **A rule reads the whole row and paints one field of it**, which is the same separation the formula scope and the rule scope always had: `` `Weekend` `` in the formula and *Day* in *applies to* colors the day of the week by whether it is one, and nothing about the painted column has to appear in the question.
- **`.cast("categorical")` is refused by name.** A category carries its allowed values, so it is declared on the column rather than computed by an expression — and the generic "not a type this can convert to" read as a denial that categories exist. The message now says where the type is set, and that ordinary text already sorts rows into named values without it, which is what usually prompts the ask.

What is deliberately not built yet:

- **Reordering rules in the panel.** Order is the list's order and later still wins; there is no drag to change it.
- **`.normalize()` costs a pass per page fetch.** Its ends are `min`/`max` over the column, which Polars cannot answer from a slice, so a paged frame reads that column per fetch. Correct, and the cost is the reason `.normalize(low, high)` takes explicit ends.
- **Dates have no reading of their own.** A date formula is refused by all three readings; comparisons and extracted parts work, the raw column does not, and every preset offered for one is a comparison. A month-name category wants `strftime`, which the formula catalog does not expose yet.

### Keyboard canon

Arrows move selection; type-to-replace; F2/Enter edit-in-cell; Ctrl+arrows jump data edges; Shift+arrows extend; Tab/Enter commit-and-move; Alt+= aggregate. Ctrl+D / drag-fill is reinterpreted as "promote to column formula." Requires a real focus/mode system (navigate vs. edit vs. canvas) replacing the single window keydown listener — build early; every workflow sits on it.

### Connectors

The source model has four person-facing doors: **File / object, Database, Web
API, and Script / CLI.** Those are not four storage formats. They are four ways
to describe what should be read, all meeting at one internal boundary:
**source recipe → Arrow batches → query plan or Parquet result.** Script / CLI
is the unrestricted adapter, not the model every other source is forced to
pretend to be.

- **Connection and source are separate.** A machine-local connection holds
  access: URI or executable, endpoint, fixed arguments, and the ordinary
  credential chain. A portable source names the object URI, SQL, API request,
  or script inputs. Refresh policy is not connection state.
- **Cached result is the default and is explicit.** Reading a source produces
  an immutable Parquet artifact. The table's Refresh action replaces that
  artifact through the existing schema reconciliation path. There is no
  on-open or interval choice in connection setup; two tables using the same
  connection remain independently refreshable.
- **The database MVP is ConnectorX extraction, not a pretend live frame.** A
  local connection stores a ConnectorX URI; the table stores its name and SQL.
  Postgres, MySQL/MariaDB, SQLite, and SQL Server are built into the desktop.
  Adding or refreshing a table runs the query eagerly into Arrow and writes the
  same immutable Parquet artifact every other imported table uses. Downstream
  work remains lazy over that artifact. Credentials in the URI never enter the
  `.fw`, while SQL does because it is the portable source recipe.
- **A cache does not decide liveness.** Remote Parquet can be scanned directly.
  A remote CSV may be revalidated by object version and normalized into a
  disposable Parquet cache because rereading text for every projection is
  wasteful. It remains live when every root query validates that cache against
  the source. Disposable query caches belong to application storage; only an
  explicit cached result or snapshot belongs beside the document.
- **Adapters fail where they are configured.** Connection, authentication,
  dialect, and query errors appear inline in the adding dialog or beside the
  table refresh action. FrameWork does not install packages in response to an
  error. Command sources continue to use their normal PATH, environment, SSO,
  cloud profile, or credential chain.
- **The command adapter stays arbitrary and safe.** A local profile names an
  executable, argument vector, and tabular stdout format. `{source}` and
  `{query}` substitutions remain individual process arguments and never pass
  through a shell. Program, fixed arguments, credentials, and PATH never enter
  the `.fw`; collaborators map the portable connection id to their own setup.
  Command stdout may be CSV, TSV, or Parquet, but every persistent result cache
  is Parquet.

### Currency and accounting types

Three separate layers:

1. **Value**: Decimal128, never rounded by display. Guard the formula catalog for decimal-safe expression paths as they are admitted.
2. **Unit**: currency code as column metadata. Mixed-currency aggregation is an error demanding an explicit conversion step (which appears in lineage with its rate as an inspectable assumption). Sibling-column currency codes later for multi-currency ledgers.
3. **Presentation**: typed format object on the view — accounting style (edge-pinned symbol, parens negatives, zero-as-dash, tabular numerals), display decimals, scale. Scaled views label themselves ("shown in $K") in the header.

**Reconciled rounding**: optional display mode where rounded detail foots exactly to the rounded total (largest-remainder allocation), true values untouched. Excel's alternatives are living with footing errors or destructively rewriting values.

### AI chat panel

Bring-your-own connection/MCP. The panel is an MCP *client* speaking to the existing framework-mcp server in-process — agent edits arrive as typed, revisioned, undoable operations in shared history; no second automation surface. Selection-as-context injects the selected object/column/rows into the prompt. Per-connection `information_schema` catalogs give the agent what it needs to write and repair dialect SQL for connectors.

### Present mode (2026-08-13)

The dashboard is the null feature: everything a dashboard is made of — multiple views of the same objects with no data duplication (§20), free canvas layout, live recompute through one dependency graph, connectors that refresh from origin, staleness badging — already exists or is already committed. What Excel and BI users call "building a dashboard" is arranging and restyling things they already have, which is what a canvas *is*. So this is **a mode, not an object kind**: a canvas rendered chrome-less — no inspectors, no toolbars, no edit affordances — read-only, full-screen or in its own window. *"Present these two tabs to my boss"* is: make a canvas, place views (views, not copies — same IDs, same lineage) of the frames and plots onto it, press Present.

- **Refresh is a table action, not a schedule.** Connectors are reusable access configuration; the cached table is replaced only when the person asks. Automation, if ever needed, belongs to an external orchestration surface rather than an on-open or interval setting hidden in a workbook.
- **Parameters become controls.** A `ValueObject` on a presented canvas renders as a slider, dropdown, or date picker instead of an editable card. That single move makes dashboards interactive — drag the discount rate, the graph recomputes — at zero model-layer cost, because a control is just another view of an object that already exists. This is also the natural first consumer of the value-formula work: assumptions built as computed values become the dashboard's knobs for free.
- **It composes with publication rather than fighting it.** Under §29, a published document opened in present mode *is* the distributed dashboard: the recipient pulls the model, their machine reads the same read-only sources, and recompute happens locally. No embedded data extract, no server holding anyone's books — the same custody story as everything else, now with a face on it.
- **Staleness stays visible in present mode.** The "reading old numbers" badge does not get suppressed for the boss. A dashboard that hides its own staleness is lying in the one context where lying matters most.

Sequencing: the chrome-less read-only mode is display-layer work and can ship early — it is the demo of the whole product. Controls ride on computed values.

### Machine learning on tabular data (2026-08-13)

The convergence target: diverse sources in, Polars wrangling, plots, predictions, all live on a presented canvas. The reason it hangs together is that every piece is the same primitive — a node in one dependency graph with stable IDs and visible lineage. A model is one more node kind, not a new architecture. Decision reasoning in the [decision log entry](#ml-training-is-native-rust-onnx-at-the-boundary-2026-08-13); this section is the design.

**The boundary: train natively or import, persist as ONNX, infer natively.**

- **A fitted model is an artifact plus an object.** The weights live as an ONNX file, content-addressed beside the parquets in `.framework/<id>/`. The `ModelObject` carries what the artifact cannot: feature columns *by stable ID*, target, hyperparameters, seed, trained-at date, and the **lineage fingerprint of the training data**. Renames stay safe; lineage cords render from the model card to every input.
- **Predict is a derived-column node.** Inference runs in Rust via `tract` (pure Rust, no system dependencies) as an ordinary recalc-graph node: deterministic, live, cacheable, safe in a shared document — running an ONNX file is interpreting data, not executing code, so the "opening a document executes nothing beyond the Rust engine" line holds for documents containing models.
- **Training is native, seeded, and in-graph.** The estimator surface is deliberately short and decades-stable: OLS/ridge/lasso, logistic regression, k-means, PCA, decision trees, random forest, gradient boosting, and classical forecasting (ETS/ARIMA/seasonal naive). Libraries: **linfa** primary, **smartcore** where it fills gaps, **augurs** for forecasting, **perpetual**/**forust** for pure-Rust gradient boosting — all cargo-packageable, no C++ toolchain, no Python runtime. Deep-learning frameworks (burn, candle) are explicitly out: wrong tool for tabular, wrong maintenance bet.
- **Preprocessing is the wrangle chain, not a pipeline object.** Encoding, imputation, scaling, and train/test splits are derived-frame steps — visible, lineage-tracked, editable — which is most of what sklearn's `Pipeline` exists to bolt on after the fact. This is why the native estimator core is smaller than it looks: the hard 80% of "an ML library" is already built and is better here than there.
- **Import and export are the same format.** ONNX in from anywhere — sklearn via `skl2onnx`, LightGBM, XGBoost — dragged into a document like a CSV, no Python shipped or invoked. Native-trained models export ONNX back out, so models roundtrip between FrameWork and the Python world in both directions with one interchange format. **Pickles are never accepted**: unpickling is code execution, the exact vector the trust model exists to block.
- **Model staleness is a first-class signal.** Because the object records its training-data lineage fingerprint, the existing staleness machinery badges it — *"trained on data 3 refreshes old"* — with retrain as the adjacent action. Native training makes retrain-on-refresh possible in principle; it should still be a deliberate gesture, not automatic, for the same reason caching is offered rather than imposed.
- **The data plane** is Polars → ndarray (`to_ndarray`) at the training boundary only. ndarray is engine plumbing, never user-facing surface (see the [scratchpad decision](#the-scratchpad-stays-one-semantics--ndarray-rejected-2026-08-13)).

The long tail — anything not in the native list — arrives later through the Python plugin runtime under its existing consent rules, and hands its result back the same way: an ONNX artifact and a model object. The tiers differ in where training runs, never in what a model *is*.

---

## 35. Decision log

Direction decisions with their reasoning, so rejected alternatives are not re-litigated from scratch.

### What a launch opens (2026-08-12)

**A launch that is handed no document starts on an empty canvas in a fresh temporary directory, with the Data library raised.** Nothing is carried over between launches and nothing is written to application data.

This replaces a backlog item that said the opposite — *reopen the last document on launch*, on the reasoning that being dropped back on the default working document is disorienting. Reopening it automatically is worse. Under `tauri dev` every Rust edit relaunches the app, so an automatically reopened document has an unattended writer on it; that is precisely how the incident behind the original item happened, a Save As landing on the wrong document. The problem was never *which* document got opened, it was that one got opened without being asked for.

Rejected along the way:

- *Keep one scratch document in application data.* Same trap in miniature: whatever was left on it yesterday greets you today. This is what made every launch open in the Commerce playground, since `document.fw` was seeded from `Document::demo()` on first run and loaded thereafter.
- *Seed the blank document from the demo.* The demo is worth having; it is a sample document, and the library offers it. Being dropped into it is a different thing from choosing it.

The scratch canvas is genuinely throwaway, and the window says so — `get_document_path` reports no path for one, so the header reads "Unsaved" rather than claiming it is saved locally. Save As is what turns it into a document.

### Python, the formula engine, and scratch calculations (2026-08-12)

Decision summary from the discussion on whether to simplify the formula compiler and where embedded Python belongs.

**The formula engine stays pure Rust.** The hand-written tokenizer/parser/compiler plus generated Polars bindings look like a lot of code, but the hand-maintained core is only ~1.2k lines; the 5.2k-line bindings file is regenerated by `tools/generate_expr_bindings.py` and costs nothing to keep. The alternatives were evaluated and rejected:

- *Parse formulas with `ruff_python_parser`* — saves ~350 net lines, loses ownership of error spans. Not worth it.
- *Eval formulas in embedded Python at parse time, serialize the `Expr` across to Rust* — deletes ~6k lines and eliminates semantic drift from real Polars, and dependency tracking survives via `Expr.meta.root_names()`. Rejected on the trust model: formulas execute **implicitly** when a document opens, and Python `eval` cannot be sandboxed. A shared document must never be a code-execution vector. (Also requires exact polars version lockstep between the Python wheel and the Rust crate — plan serialization is not stable across versions.)

**Python is still coming — as the plugin language, plus scipy/numpy data-science modules on top.** The trust line that makes this safe: plugins are **explicitly installed** (user consent, VS Code extension model), unlike formulas which run implicitly. Architecture guardrails agreed:

- Run plugin Python in a **subprocess**, not in-process pyo3: crash isolation, killable, and a place to hang OS sandboxing later. Data crosses as **Arrow IPC**, which is version-stable — no polars lockstep needed on the data plane.
- A plugin invocation is a **node in the recalc graph with declared inputs/outputs**: it gets a snapshot of declared frames, returns frames/values, engine validates returned schema before splicing in. Plugins never mutate engine state directly. Mark plugin nodes potentially nondeterministic for caching.
- **Documents must never smuggle code into plugins**: documents may call declared plugin functions with values, never embed Python source that a plugin evals. Holding this line means opening a shared document executes nothing beyond the Rust formula engine, even with Python plugins installed.

**Scratch calculations (the Excel-scratchpad job) get built in Rust, in two tiers:**

1. *Grow the existing formula language first* — it already has `Expr::Value { object_id }` (canvas values), the dependency graph, and recalc. Scalar cells holding formulas, plus let-bindings / multi-step formulas that desugar to chained scalar cells, likely cover most of the scratchpad need with no new language. (Overlaps with "Computed values" in the design backlog.)
2. *Only if users need loops/conditionals*: embed a sandboxed-by-design language — **Luau** (via `mlua`, built for untrusted scripts) is the leading candidate; Rhai the pure-Rust fallback. RustPython rejected: Python syntax without numpy or a hardened sandbox buys neither of the things Python was wanted for.

**Revisit later:** CPython compiled to WASI under wasmtime is deny-by-default sandboxed and could eventually make implicit-execution Python acceptable even in shared documents. Numpy-on-WASI support is still maturing — check back ~2027.

### Sharing is publication, not collaboration (2026-08-13)

**Multi-writer live editing is off the frame. A document is shared by publishing it — commit, push, pull — and it carries no data.** §29 holds the resulting model; this entry holds why, so the alternatives are not re-litigated.

The existing event journal is genuinely well-built for what it does — per-writer append-only directories, immutable files, `create_new` plus hard-link, so no two machines ever share an append target. It is not the problem. The problem is that replaying operations without transformation cannot converge: causal delivery orders causally-*related* events, and concurrent ones apply in arrival order, which differs per machine. Two replicas end with the same version vector, different documents, and no signal that anything went wrong. That is transport-independent — a real server would not fix it — and the fix is a CRDT covering structural operations like column insert and frame delete, which is a large project undertaken to support a case that publication handles.

Rejected along the way:

- *Sync through a consumer cloud drive (Drive, Dropbox, OneDrive).* Viable for asynchronous handoff, useless for anything live: propagation is seconds on Dropbox and tens of seconds to minutes on Drive and iCloud, and latency sets the width of the window in which concurrent edits happen. It also makes every replica re-download the parquet, which is minutes on any of them.
- *Bring-your-own-data — same model, different ledger per person.* Solves custody, but collaborators cannot jointly observe a result. Every conversation about the document is about numbers that differ. Superseded by the shared read-only source, where identical data plus identical formulas means identical figures.
- *A server holding the authoritative document, the Sheets/Excel/Numbers model.* It is what makes their collaboration good, and it is unavailable here for a reason that has nothing to do with engineering: a Sheet is millions of cells and fits in a server; a FrameWork document fronts multi-million-row general ledgers. Holding the authoritative document means holding somebody's books, with the custody, compliance and bandwidth that implies.
- *Inlining small hand-typed frames as part of the model.* Considered, then rejected: a 50-row mapping as a JSON array merges badly under git's line-based merge, where the same frame as a CSV diffs cleanly and reviews in a pull request.
- *Wiring undo to git.* Different altitudes. Undo is ephemeral, fine-grained and personal; a commit is durable, coarse and shared. Conflating them gives either a commit per keystroke or undo granularity set by whenever someone last committed.

What this buys, beyond correctness:

- **Undo simplifies, and loses two whole-document clones.** Worked through in [§28.1](#281-undo-and-redo): the skip loop goes, `apply_history`'s rollback clone goes, and `prepare_event`'s per-operation `Store` clone goes with the journal that made publication irreversible.
- **The journal becomes dead weight.** It is replication machinery with no remaining job. Delete it, or repurpose its events as the commit unit — semantic operations make a better `git add -p` than textual hunks do.
- **Distribution was the valuable half anyway.** A published model that consumers pull and recompute against their own data is a dashboard that updates itself, and it is one-way, so no merge problem exists to solve.

**Build the file, not the tool.** The missing readable identifiers described here have since landed. External git can therefore work for anyone comfortable with it, while internal versioning remains a UI over a format that is already reviewable rather than a rescue of one that is not. Embedding gix or libgit2 buys merge, history, remotes and auth outright; the application owns commit, pull, log and diff, and a terminal escape hatch owns rebase and bisect.

### ML training is native Rust, ONNX at the boundary (2026-08-13)

**Model training runs in the Rust engine; ONNX is the only model interchange format, in both directions; the Python plugin runtime is the long tail, not the core.** The design is in [§34 Machine learning](#machine-learning-on-tabular-data-2026-08-13); this entry holds why, because the first instinct was the opposite.

The initial recommendation was training-via-Python-plugins, riding the plugin architecture already decided for scratch-adjacent work. Reversed on three grounds, in order of weight:

- **Shared documents must recompute.** Publication (§29) promises that a recipient recomputes everything locally from the same sources. A plugin-trained model breaks that promise on any machine without the plugin installed; a native model keeps it. This is decisive on its own — the model column in a published dashboard cannot be the one column that goes dead on pull.
- **The stability argument cuts against Python, not for it.** The case for sklearn is its breadth and maturity; but the estimators tabular work actually uses — linear models, trees, forests, boosting, classical forecasting — are decades-old and unmoving. A frozen target is exactly what a curated native implementation is good at. Breadth is what the plugin tier is for, later.
- **Packaging Python is a real cost with a controllable deadline.** It will be paid for the plugin ecosystem eventually, by users who opt in. Paying it up front, for the flagship analytical feature, inverts that choice.

On gradient boosting specifically: an earlier draft called pure-Rust GBM immature and proposed binding LightGBM's C++ or deferring to import-only. Corrected — **forust** and its successor **perpetual** are pure-Rust, seeded, cargo-packageable boosters, and one of them takes the native slot. ONNX *import* remains the escape hatch for anyone who wants LightGBM itself: train outside, drag the model in, the staleness badge keeps the retrain loop honest.

Rejected along the way:

- *Training in Python plugins as the primary path* — see above; survives as the long-tail tier, handing back the same ONNX-artifact-plus-object every other tier produces.
- *Pickle import* — unpickling is arbitrary code execution; a shared document must never be a code-execution vector. No exceptions, including "just this one model from our own data scientist."
- *Deep-learning frameworks (burn, candle) for the tabular core* — wrong tool for the data shape, wrong maintenance bet for the team size.

### The scratchpad stays one semantics — ndarray rejected (2026-08-13)

**No ndarray (or any second array semantics) in the scratchpad or the formula language. One language, one alignment model, everywhere.** Raised because Polars is opinionated and occasionally strange, and a numpy-like surface felt friendlier for quick work. Rejected on two grounds:

- **It violates the block's own founding rule.** "One language, and the whole of it" — nothing learned in scratch fails to transfer to a column formula — is load-bearing for the on-ramp story. A second semantics inverts it: scratch habits would *actively mislead* everywhere else, which is worse than a missing feature.
- **Positional alignment is the banned failure mode.** ndarray's defining behaviors — pairing arrays by ordinal, broadcasting across shapes — are precisely what "no implicit positional broadcast" exists to prevent. Admitting them in one surface re-imports the silent-wrongness class the row-identity model was built to kill.

The legitimate complaint underneath ("Polars is weird") is addressed by the work already specced, all sugar over one semantics: the alias registry teaching Excel vocabulary, percent literals, expansion functions, friendlier errors. If a real linear-algebra need materializes, it arrives as functions in the one language with explicit alignment, never as a namespace with different rules. ndarray itself lives on as engine plumbing at the ML training boundary (`to_ndarray`), invisible to users.
