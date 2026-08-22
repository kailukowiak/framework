# House rules for agents working on FrameWork

Read this before writing any interface code. It exists because the same
mistake keeps arriving from the same direction, and it is a mistake about
what this application is.

## What this application is

FrameWork is a successor to the spreadsheet, and **Excel's simplicity is the
bar it is measured against**. A spreadsheet shows you forty numbers at 11px
in a grid and spends nothing on presenting them. That is not an aesthetic
preference — it is the product. Density is what lets someone hold a whole
model in view, and every pixel spent announcing a value is a pixel not
spent showing another one.

So the instinct to make a thing look *considered* — a generous card, a big
confident number, a status chip, a labelled field per input — is wrong here
in a specific, repeatable way. It produces cards that look designed and hold
almost nothing.

## Rules

**1. The content is the interface. Chrome is the enemy.**
Before shipping a card, count what it spends per unit of content. A card
holding one number should not be 150px tall. If a card would repeat a
control for every row, line or field it holds, it is the wrong shape: reach
for one editable surface instead of N controls. The formula block is the
worked example — see `BlockCard` in `src/App.tsx`, and the comment above it.

**2. Do not use display type on canvas cards.**
`--text-display` (26px) is for the document, not for a value. A value on a
card belongs at `--text-base` or `--text-sm`, the same size as everything
else. A number rendered four sizes larger than its own name is backwards:
the name is what makes the number mean anything.

**3. A thing's name is at least as prominent as its value.**
Never a muted, small name over a large bold value. Same size, and the name
no greyer than the value. In a spreadsheet the header row is not smaller
than the cells.

**4. No control that restates a gesture already available.**
If evaluation is live, there is no Execute button. If the field commits on
blur, there is no Save. If the row can be selected and deleted, there is no
per-row × . A button that only tells you something is possible is chrome.

**5. Errors go where the thing is, as text.**
Inline, in the gutter or under the field that failed, in the ordinary type
size. Not a boxed alert with an icon and a heading — that spends a card's
worth of space saying one sentence.

**6. Help text is a last resort, and never per-field.**
A placeholder that shows the syntax (`x = 10`) teaches more than a sentence
explaining it, and costs nothing when it is not needed.

**7. Cards that hold open-ended content resize; cards that hold one thing
size themselves.**
Do not make a fixed 340×220 card for something that grows.

## Before you ship a piece of interface

Re-read what `ProjectSpec.md` says the feature is **for**, not just what it
should do, and check the thing you built against that purpose. The formula
block was specified to solve a density problem — "forty scratch calcs is
forty cards and a mess" — and the first implementation gave every line its
own name field, delete button, help text and Execute button, which was forty
cards again, stacked inside one card. It satisfied the feature list and
contradicted the reason the feature exists.

Ask: *if this card held twenty of whatever it holds, would it still be
usable?* If not, the shape is wrong, however good one row of it looks.

## Codebase map — read this instead of re-deriving it

The same orientation pass keeps getting repeated at session start. The
shape below is stable; function names are anchors, line numbers are not.

**Document format.** A `.fw` file is JSON: `document.objects` is a flat
list of kinds `container`, `value`, `frame`, `block` (Scratchwork), and
`text`; `document.views` holds canvas layout only. A frame carries
`columns`, literal `rows` (empty for computed frames), a wrangle `steps`
chain, `display` (view-layer only), and at most one row source:
`sourceFile` (imported), `artifact` (frozen parquet), `connector` (live),
or `derivation`.

**framework-core (`crates/framework-core/src/`).**
- `model/frame.rs` — `FrameObject`, `Column`, `Row`/`Cell` (with
  `override_formula`), `FrameDisplay`. `owns_its_rows()` is the single
  "can I type here" question: true only with no derivation, artifact,
  source file, or generator. Also `FrameGenerator` (a frame grown from a
  `sequence(...)` rule), `EntryColumn` (hand-entered values on a computed
  frame, stored by key so they survive regeneration), and
  `CrosstabDisplay` (a long frame shown wide, display-layer only).
- `model/derivation.rs` — `FrameDerivation { source_frame_id, join, steps }`
  and the `FrameStep` enum (Filter, WithColumns, Select, Summarize, Join,
  Sort, Union, Expand, Pivot, Unpivot, Comment). Expand is the
  cross-product for-each and accepts a `frame_id` only. Pivot and union
  outputs are baked at save time by design; the doc comments there carry
  the argument.
- `formula/` — `ast.rs` (`Expr`; `validate_list_placement` is what
  quarantines `sequence()` to Scratchwork or `frame.len()`-bound fills),
  `compile.rs` (Expr → Polars; `Expr::Value` inlines value objects,
  results, *and named block lines*, so Scratchwork names work inside frame
  formulas), `catalog.rs` (every function with aliases;
  `formula_function_catalog()` is public and feeds
  `DocumentView.formula_functions`), `complete.rs` (autocomplete).
- `engine/` — `plan.rs` (builds LazyFrames; positional row ids render as
  `source:{frame}:{index}` / `derived:{frame}:{index}`), `frame.rs` (step
  execution), `compute.rs` (`DocumentView` assembly; `evaluate_to_series`
  evaluates any scalar or list expression on a one-row probe frame).
- `operation/` — `kinds.rs` is the canonical `Operation` enum; `prepare/`
  validates, `apply/` mutates (`DerivedFrameReadOnly` lives in
  `apply/cells.rs`), `invert.rs` is undo.
- `store.rs` (`Store`, `view()`) and `validate.rs` (cross-cutting checks
  such as the cell-override rules).

**framework-mcp (`crates/framework-mcp/src/main.rs`).** Tools are
`#[tool]` methods in one `#[tool_router]` impl; args are
`schemars::JsonSchema` structs; reads take `self.lock()`, writes go
through `mutate()` with `expectedRevision`. `describe_operations` serves
TypeScript generated from the canonical `Operation` enum, and
`apply_operation` accepts that exact union. Headless testing:
`cargo test -p framework-mcp`.

**Invariants that answer recurring questions.**
- Derived and imported frames refuse edits; a cell override is a formula
  recorded against an owned row, and is refused on artifact-paged frames.
  The one typed exception is an entry column: values stored by the row's
  key, joined back on at read (`AddEntryColumn` / `SetEntryValue`).
- A generator frame turns a `sequence(...)` rule into rows
  (`AddGeneratorFrame` / `SetFrameGenerator`); `Expand` still takes a
  `frame_id`, and pointing it at a generator frame is the sanctioned
  cross product. Sequence bounds may reference values and are folded at
  compile time, so the frame regrows when the value changes.
- Multi-column unique keys are supported (`uniqueKeys` on the frame), and
  an entry column requires one over exactly its key columns.
- Values, results, and block lines are all `Expr::Value` references,
  inlined at compile time; the document refuses cycles.
- Pivot on a date column is allowed; `RefreshFramePipeline` re-bakes
  pivot/expand/union outputs against current data, preserving surviving
  output ids.

## Frame ownership and calculated columns

**Ownership is not materialization.** A frame is document-owned when its
literal rows and columns live in the `.fw` document: frames created or pasted
in FrameWork, and imported frames after *Adopt data*. Freezing or caching a
frame makes a static artifact; it does not make that artifact's rows directly
editable. Badges such as *base* and *static* describe lineage and refresh, not
permission to post arbitrary data.

Literal row and column insertion therefore belongs only on document-owned
frames. Live imports, frozen snapshots, and other artifact-backed frames may
still take calculated columns: a formula is a plan layer over the data, not a
mutation of the artifact.

There is one authoring surface for a new calculated column: the transformation
chain in **Wrangle**. The frame context menu puts *Add calculated column* near
the top, appends a `withColumns` step at the bottom of the existing chain, and
opens that formula there. Do not add another calculated-column dialog to
Selection or recreate a Frame tab. The creation gesture first saves
`null.cast("number")`, which gives the engine a typed Number column whose cells
render blank, then focuses the formula so it can be replaced. This makes the
new column visible immediately without introducing a literal-data path on a
live frame.

Editing that formula from the grid or a future top formula bar belongs to the
shared active-editor registry. Do not make the top cell, formula bar, and
Wrangle editor independent drafts that can disagree.

## Live computation and positional identity

**Live data is not the unsafe part; an unstable row address is.** Keep these
as separate questions everywhere — parser, completion, click handling, copy,
help text, tutorials, and tests.

- A whole-column expression such as `sum()`, `mean()`, `mode()`, a grouped
  aggregate, or a declared ordered query is semantic. In Scratchwork it reads
  the current base or derived frame and recomputes when upstream rows change.
  Do not require a snapshot, materialization, or frozen answer merely because
  the frame is live or derived. A downstream Scratchwork line that reads that
  aggregate remains live too.
- Do not use liveness as a general operation gate. Filters, lookups, calculated
  columns, ordered queries, aggregates, and other semantic operations all work
  against live frames. A refresh making a filter empty or a lookup invalid is
  an honest result or inline formula error, not evidence that the operation
  should have been prohibited.
- A clicked profile statistic is the same semantic expression written by a
  pointing gesture. It stays referenceable on live and derived frames; closing
  the profile drawer changes no dependency.
- A specific-cell reference is the sole ownership exception. It is available
  for an internal, document-owned dataset and unavailable for an imported,
  live, or derived result whose current row is only a position. Do not let this
  exception disable column, profile, key-based, or other semantic references.
- Cell reference authoring must remain Excel-easy: with Scratchwork active, a
  click inserts the reference. Never ask the user to type an internal row id.
  The current centralized gate permits the ordinal spelling only when stored
  and displayed row order coincide. Supporting an internal frame through a
  sort or filter means carrying its stable literal row id in the expression;
  it does not mean persisting the displayed ordinal or adding more UI.

Caching and freezing are explicit history/performance choices, not correctness
requirements for Scratchwork. Tests for any new formula surface must cover both
sides of this rule: live semantic formulas update after an upstream edit, and
a live-frame cell click is refused without disabling any non-cell operation.

## Working conventions

- Structural lint warnings are design feedback, not background noise. Run
  `npm run lint` for TypeScript and React work and `npm run lint:rust` for Rust
  work. Do not add or worsen a warning. In particular, a file-size or
  function-size warning is a prompt to find the responsibility that should
  move, not permission to compress the same work or silence the rule. Generated
  bindings and catalogs are excluded at their source; do not broaden those
  exclusions to handwritten code. Existing oversized files are debt to ratchet
  downward: touching one should leave it no larger unless the task genuinely
  cannot be separated, in which case explain why in the code or handoff.
- There is no browser preview for this app, but there is something better:
  `npm run test:e2e` launches the real desktop binary and drives it over
  WebDriver (see "The last twenty percent" below). Verification should match
  the risk: use `cargo test --workspace`, `npx tsc --noEmit`, and `npm test`
  for logic; use a focused frontend interaction test where the behavior is
  wholly inside React; extend a native e2e workflow when behavior crosses the
  UI, Tauri commands, engine, persistence, or history. Visual density, native
  menu wiring, and true operating-system focus remain manual checks for Kai
  after `npm run tauri dev`. Do not use Computer Use for testing unless Kai
  explicitly asks for it.
- The MCP surface is testable without launching the app or enabling MCP in
  Settings. Run `cargo test -p framework-mcp` for focused work. Its
  `describe_operations` catalog is generated recursively from the canonical
  Rust `Operation` enum, and `apply_operation` accepts that exact serialized
  union through the normal validation, history, persistence, and collaboration
  path. When a public operation is added or changed, extend the catalog/parity
  tests so agents can exercise it headlessly; do not add a second hand-written
  operation model to MCP.
- `tools/mcp-smoke/` is the agent lane on top of those deterministic tests:
  `run.sh <scenario> [model]` hands a cold model a scenario prompt and the
  server, then grades the run on the document it produced — structural facts
  plus behavior driven back through the same tools — never on the agent's
  self-report, which inherits every bug in the tools it was written with. It
  measures discoverability, which no scripted test can; it costs a model run,
  so it is on-demand, not CI. Its README carries the grading rules and the
  scenario recipe.
- Anything someone using FrameWork would notice gets a line under
  `## Unreleased` in `CHANGELOG.md`, in the same commit as the change. Not
  refactors, tests, CI, or comments — a behaviour, an interface, or a fix that
  changes what happens when they use the app. This is not bookkeeping: the
  release workflow lifts the section matching the tag into the GitHub release
  body and, through `src/lib/updates.ts`, into the in-app update offer, and it
  refuses to build a release whose version has no section. So an entry left
  unwritten is not a tidy file with a gap in it — it is either a release that
  cannot describe itself or one that never builds. Write it for the person
  using the app: what is different now, not which module moved. Cutting a
  release renames `## Unreleased` to the version being tagged and opens a
  fresh one above it.
- Cutting a release is two things in one commit and then a tag.
  `npm run version:set 0.1.4` writes the number into all five places it is
  spelled — package.json, the Cargo workspace, tauri.conf.json, the MCP
  server's advertised version, and the changelog, whose `## Unreleased`
  becomes `## 0.1.4` with a fresh empty one above it — and refreshes both
  lockfiles. Commit that, push it, and only then `git tag v0.1.4 && git push
  origin v0.1.4`. The tag is what starts the build, and the workflow refuses
  a tag whose number disagrees with tauri.conf.json or that has no changelog
  section, so a tag pushed onto the wrong commit fails loudly rather than
  publishing a release that announces the previous version. The release is
  drafted, not published: check the assets on the Releases page, then click
  Publish, which is the moment installed copies start being offered it.
- Comments in this codebase explain *why*, at length, in prose. Match that.
  A rule that took an argument to arrive at should carry the argument.

## The last twenty percent: proving the interface does what it says

The recurring failure mode of work on this app is not broken logic — the
engine and the component logic are well tested — it is the seam between
them: a feature whose unit tests pass and whose interface, launched for
real, does not do the thing. That gap used to be found by Kai, by hand,
intermittently, which means late. The e2e harness exists to find it in the
same session the code was written.

`npm run test:e2e` builds the e2e frontend (`--mode e2e`, which is how the
bundle declares itself — `import.meta.env.VITE_FRAMEWORK_E2E`), produces a
debug `FrameWork.app` through the Tauri CLI with the `e2e` cargo feature,
and runs `e2e/specs/*.e2e.ts` against the launched app — real WKWebView,
real Rust engine, and real persistence, driven by WebDriver-synthesized input.
The harness launches
the bundle's inner executable, never a bare `cargo build` binary: an
unbundled executable has no Info.plist identity and its WebKit helpers
proved unreliable — webviews that sat at about:blank and crash reports —
which cost a full day to diagnose. (macOS has no external WebDriver for
WKWebView; the `e2e` feature embeds the server in the app instead. That
feature must never be enabled in a default or release build, and the
plugin registration in `src-tauri` must stay behind it.)
`npm run test:e2e:only` reruns specs without rebuilding — useful while
writing a spec, but a green run against a stale bundle proves nothing, so
never quote it as verification after changing app code.

**A sandboxed agent shell cannot launch this app — escalate, don't debug.**
The desktop binary is a GUI app: at startup AppKit registers the process
with the macOS window server, and a sandboxed exec environment (the
ChatGPT/Codex app's Seatbelt profile is the known case) denies that
registration, so the process dies instantly with SIGABRT inside
`RegisterApplication` — before the WebDriver server exists. The signature:
wdio reports the session never starting, and a crash report in
`~/Library/Logs/DiagnosticReports/Retired/framework-desktop-*.ips` shows
`RegisterApplication` under `+[NSApplication sharedApplication]`. This is
environmental, not an app bug — the same bundle launches 100% reliably
from an unsandboxed shell, and no code change can fix it (a GUI app cannot
opt out of window-server registration). When `npm run test:e2e` dies this
way, run it with escalated permissions instead: in Codex, request
escalation for this one command (build steps run fine sandboxed; only the
launch needs the escape). Burning a session debugging this crash as if it
were a product bug is the failure mode this paragraph exists to prevent —
it has already cost multiple sessions.

**The rule: test at the narrowest layer that can prove the behavior, and keep
at least one native e2e path across every important product seam.** Do not
turn every CSS or component change into a slow desktop test. Conversely, a
feature that depends on UI → Tauri → engine → recompute/persistence/history is
not proved by a mocked browser test. The existing specs show the shape:
`scratchwork.e2e.ts` enters formulas and watches answers move;
`tutorial-formula-clicks.e2e.ts` resets an isolated tutorial library through
the Data library's own two-click confirm, opens the Start workbook, authors a
cross-object formula, edits a cell, and watches the live total move and undo.
A spec that only checks an element exists does not prove that seam.

Conventions that keep the suite honest:

- Select by accessible name (`aria-label`, visible text), not by test ids.
  If something cannot be reached that way, it is missing an accessible
  name — add the `aria-label`, which is product work, not test plumbing.
- Synchronize on observable state (`waitUntil` gutter text changes, an
  element appears), never on sleeps. Scratchwork evaluation is live; the
  gutter *is* the signal.
- Native file dialogs cannot be automated. Flows must be drivable through
  dialog-free paths — the scratch canvas, sample documents, and the
  tutorial workbooks. Do not add hidden test-only commands to route around
  a dialog; if a flow is untestable without one, that is a design
  conversation, not a workaround.
- Tutorial specs receive a private temporary library through
  `FRAMEWORK_E2E_TUTORIAL_DIRECTORY`; they must never read, reset, or edit the
  tutorial workbooks in the person's Documents folder. The override exists
  only in an `e2e`-feature binary. Reset the private copies through the UI when
  a spec depends on their contents: that keeps the product's advertised
  learning flow in the test without treating user data as a fixture.
- The e2e shell runs menu-less, using the app's in-page accelerator path:
  WebDriver's synthesized keys never reach a native menu, so under one the
  menu-owned accelerators (⌘J, ⌘Z, ⌘N …) would be undrivable. In the e2e
  build the window's own keydown handler takes them — a code path the app
  already carries — and `hasNativeMenu()` learns which shell it got from
  the build-time flag, not from guessing. The native menu wiring itself is
  the one seam WebDriver cannot cover; it stays with Kai's hands.
- One spec file, one app. The service launches a single app instance per
  wdio run, so `test:e2e:only` runs each spec file in its own invocation —
  a file gets a fresh scratch document and an open Data library, and its
  tests share state deliberately, in order. Do not write a spec file that
  depends on another file having run.
- The embedded driver proves integration, not literal human input. Its known
  gaps and compensations are already written — copy them, don't rediscover
  them: multiline text goes through `setValue` (typed Enter inserts no
  newline); cell editing is click + F2 (double-click never reaches React);
  after typing in a formula editor, the one `execute(blur())` in
  `tutorial-formula-clicks.e2e.ts` stands in for the native focus transfer
  synthesized clicks cannot perform — while the editor is active, frame
  clicks are formula-pointing, not selection. If a bug could depend on those
  differences, report the remaining manual check to Kai. Use Computer Use only
  when Kai explicitly requests it, not as part of the normal test workflow.
- Specs live in `e2e/specs/`, helpers in `e2e/lib/`, and both typecheck
  with `npx tsc -p e2e/tsconfig.json`. `npm run lint` covers `e2e/` too.

## The interaction tier: mounted components, generated fixtures

Between the logic tests and the e2e workflows sits the tier for interface
*behavior that never leaves React*: mount the real component with Testing
Library (`// @vitest-environment jsdom`, files named `*.test.tsx`, run by
`npm test`), act like a person, and assert the operation the interface
emits or the state it shows. `src/BlockCard.test.tsx` and
`src/DatasetDialog.test.tsx` are the exemplars; `src/test/support.ts` is
the only door to backend data.

The rule that keeps this tier from growing back into the 2.5k-line
preview engine this repo once deleted: **nothing in a test double
computes.** Fixtures are `DocumentView`s generated by framework-core
(`cargo run -p framework-core --example generate_ui_fixtures` — committed,
regenerated, never hand-edited, selected by name because ids re-mint), and
`serveInvoke` replays only explicitly-handed answers, refusing unmocked
commands by name. If a test needs the answer to *change* after an
operation, it is asking an engine question — write it in Rust, or as an
e2e workflow. The moment someone proposes teaching the double to
recalculate, the answer is no; that is the same mistake wearing a smaller
font.

The division of labor, stated once: the interaction test proves "typing
this emits `setBlockSource`"; the Rust test proves what `setBlockSource`
means; the e2e spec proves the emitted operation actually reaches Rust
and comes back rendered. No tier restates another's claim.
