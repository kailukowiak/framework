# Module decomposition

A plan for splitting `crates/framework-core/src/lib.rs`, and an assessment of whether
the same treatment is worth applying to the TypeScript side.

> **Executed.** The Rust half of this landed: `lib.rs` went from 16,067 lines to 41,
> across 52 files, with the largest hand-written module at 911 lines. The tree below
> is the plan as written, not a map of what shipped — it was drafted at `lib.rs`
> 14,677 lines, so every line estimate here is low, and the shape drifted in two
> places worth knowing about. There is no `model/view.rs`: the unification that
> followed deleted `TableView`, `TableViewStyle`, and `ViewSortKey` outright and moved
> display state onto `TableObject` (see [architecture.md](architecture.md)). And the
> `impl` blocks split further than planned — `engine/` grew `build.rs`, `cache.rs`,
> `table.rs`, and `values.rs` alongside `plan.rs` and `compute.rs`. Read
> `crates/framework-core/src/lib.rs` for the tree that exists.
>
> The TypeScript assessment below is likewise a snapshot of its moment. Two things
> it names are gone: `viewFiltering.test.ts` and `viewStyling.test.ts`, and the
> `resetDemo` IPC call they reached through — the Reset demo affordance was removed
> once a launch stopped opening the demo document at all. The browser-preview adapter
> described there is gone too: after it was split out, its 2,117 lines of fixtures,
> operations, and recalculation code were deleted rather than kept as a second,
> non-canonical engine. FrameWork now runs its interface through Tauri only.

## Why

```text
crates/framework-core/src/lib.rs   14,677 lines (523 KB)
  ├── implementation                9,544
  └── #[cfg(test)] mod tests        5,133   (72 tests)

src/App.tsx                         4,702 lines
src/lib/api.ts                      1,534 lines
```

Size alone is not a defect. The concrete costs here are: every `cargo test` recompiles
5,133 lines of test body as part of the lib crate; `cargo test --test <slice>` cannot
select a subject area; a rust-analyzer edit anywhere in the file reanalyzes all of it;
and a reader looking for the join validation rules has no path to them except search.

The decomposition below is behavior-preserving. It moves items between files, adds
`pub(crate)` where a call now crosses a module boundary, and re-exports the public surface
from the crate root so that no consumer changes. It renames nothing and changes no
signature. It adds exactly two accessors to the public API — `Store::document` and
`Document::table` — for reasons argued in step 1; nothing existing is removed or altered.

## framework-core

### Step 1 — Move the tests out first

This is the single largest win and the least risky change, so it should land on its own
commit before any implementation file moves. It also gives the module split a safety net
that lives outside the code being moved.

`#[cfg(test)] mod tests` compiles *into* the crate and can reach private items — including
private items of ancestor modules, which is what the current block quietly depends on. A
file under `tests/` compiles as a separate binary that links only the public API. So the
question is how much of the 72 tests' 5,133 lines actually needs in-crate access.

| | Tests | Lines | What it needs |
| --- | ---: | ---: | --- |
| Already portable | 13 | 638 | nothing — public API only |
| One accessor away | 46 | 3,197 | `Store::document` |
| Two accessors away | 5 | 687 | `Store::document`, `Document::table` |
| Genuinely in-crate | 8 | 587 | private internals (below) |

The blocker for 51 of 72 tests is a single private field. `Store` holds
`document: Document` privately and exposes no borrow of it — only `view()`, which returns
a computed `DocumentView` projection. The tests read `store.document.objects`,
`store.document.views`, `store.document.revision` and so on 191 times, and — checked
explicitly — **never mutate through it**.

So step 1 has a precondition, and it is two lines:

```rust
impl Store {
    pub fn document(&self) -> &Document { &self.document }
}

impl Document {
    pub fn table(&self, table_id: &str) -> Result<&TableObject, CoreError> { … }  // was private
}
```

Both are defensible on their own merits rather than as test accommodations. A read-only
borrow cannot violate the invariants `Store`'s private fields exist to protect — those
live in `version_vector`, `undo`, `redo`, and `sorted_page_cache`, none of which a
`&Document` can touch — and a document store that will not lend out its document is an odd
omission next to an already-public `DocumentView`. `Document::table` is a keyed lookup a
caller can already write by hand over the public `objects: Vec<DataObject>`; publishing it
adds convenience, not capability.

With those two in place, 64 tests and 4,522 lines move to `tests/` unchanged apart from a
`use framework_core::*;` and a shared helper module.

The remaining eight reach deeper, into items I would *not* widen:

| Test | Private item |
| --- | --- |
| `unsupported_fw_versions_are_rejected` | `FrameworkDocumentFile` |
| `perf_probe_import_view_and_page_costs` | `materialized_for_view`, `compute_tables`, `compute_canvas_views` |
| `formula_references_use_exact_backticks_and_support_table_qualification` | `parse_formula_for_table`, `column_dependencies` |
| `exact_backtick_references_disambiguate_names_and_escape_backticks` | `parse_formula_for_table` |
| `polars_methods_render_canonically_and_evaluate` | `POLARS_FORMULA_FUNCTIONS` |
| `calculated_columns_are_batched_by_dependency_layer` | `calculated_column_layers` |
| `anti_and_semi_joins_reject_lookup_side_output_columns` | `validate_join_derivations` |
| `polars_dtypes_map_onto_framework_column_types` | `framework_type_from_polars` |

These stay in-crate as `#[cfg(test)] mod tests` blocks — but placed beside the module
they exercise once step 2 lands, not in one shared block. That is what unit tests are
for, and it is the right outcome: a private function's test should sit next to it.

The line between the two accessors above and these eight is worth stating, because it is
the judgment the whole step rests on. `Store::document` and `Document::table` are things a
caller would reasonably want. `POLARS_FORMULA_FUNCTIONS`, `calculated_column_layers`, and
`materialized_for_view` are internal mechanism, and exporting them to relocate a test file
would be letting test placement design the public API.

```text
crates/framework-core/tests/
├── common/mod.rs            demo_store, table_named, temporary_test_directory
├── persistence.rs           .fw format, legacy JSON, save-as, artifact copying
├── collaboration.rs         event journal, merge, replay, version vectors
├── history.rs               undo/redo, dependency-safe deletion, atomic multi-cell
├── table_views.rs           tabs, orientation, styles, resize, detach/move
├── view_filter_sort.rs      display filters, view sort, materialize-a-view
├── paging.rs                sorted page cache, paged reads, grouped aggregates
├── formulas.rs              references, calculated columns, Polars syntax, dates
├── derived_tables.rs        aggregates, transformations, linked tables
├── joins.rs                 unique keys, anti/semi, refresh
├── columns.rs               categoricals, column formats, types
└── import_export.rs         CSV/TSV/Parquet import, artifacts, CSV export
```

Twelve files averaging ~380 lines. `cargo test --test joins` becomes possible, and the
files link and run as independent binaries.

One note on `perf_probe_import_view_and_page_costs`: it is a timing probe with `println!`
output rather than an assertion-bearing test, and it is the reason three private
compute entry points are reachable from the test block. It belongs in `benches/` or
behind an `#[ignore]`, and deciding that is a prerequisite for cleanly placing the rest.

### Step 2 — Split the implementation

The 9,544 remaining lines already have visible seams. The largest are two match
statements: `Document::prepare_operation` (1,277 lines, 46 arms) and
`Document::apply_replicated` (1,017 lines, 36 arms), which together are 24% of the
implementation.

```text
crates/framework-core/src/
├── lib.rs                     ~120   crate docs, module tree, flat `pub use`, Id + consts
├── error.rs                   ~50    CoreError
├── store.rs                   ~460   Store — the façade: load/save/apply/undo/redo/pages
├── persist.rs                 ~140   FrameworkDocumentFile, .fw envelope, write_replacing
│
├── model/                            plain data — no logic beyond accessors
│   ├── mod.rs                 ~30
│   ├── document.rs            ~170   Document, DataObject, ValueObject, TextObject,
│   │                                 CanvasView, DocumentView, ComputedCanvasView
│   ├── table.rs               ~260   TableObject, Column, Row, Cell, CellUpdate, Summary,
│   │                                 DataType, ScalarValue, ColumnFormat
│   ├── derivation.rs          ~200   TableDerivation, TableJoin, JoinOutput, DerivedExpression,
│   │                                 DerivedSort, ViewSortKey, UniqueKeyConstraint, *Input
│   ├── view.rs                ~180   TableView, TableCellStyle, TableViewStyle, TablePage,
│   │                                 TableQueryPlan, ComputedTable, ComputedCell
│   ├── artifact.rs            ~90    DataArtifact, ArtifactFormat, ConnectorRecipe, Materialization
│   └── plot.rs                ~60    PlotObject, update_plot_field_titles
│
├── operation/
│   ├── mod.rs                 ~20
│   ├── kinds.rs               ~450   Operation (46) + ReplicatedOperation (36)
│   ├── prepare/                      impl Document::prepare_operation, split by family
│   │   ├── mod.rs             ~90    the dispatch match; arms delegate
│   │   ├── objects.rs         ~300   Add*/Import*/Refresh*/Rename*/Delete*/RenameDocument
│   │   ├── views.rs           ~280   Move/Resize/Collapse/tab lifecycle/filters/sort/style
│   │   ├── cells.rs           ~200   SetCell(s), AddRow, DeleteRow, SetCellOverride
│   │   ├── columns.rs         ~250   Add/Delete/Rename/SetType/Categories/Format/Formula/Summary
│   │   └── derivation.rs      ~230   Derived/Linked/Join tables, UniqueKey, Materialization
│   └── apply/                        same six-way split, ~1,030 total
│
├── formula/
│   ├── mod.rs                 ~20
│   ├── ast.rs                 ~330   Expr, BinaryOperator, Formula, FormulaFunction,
│   │                                 validation, column_dependencies, render
│   ├── lexer.rs               ~230   Token, tokenize, ReferenceName, FormulaReference
│   ├── parser.rs              ~350   Parser
│   ├── compile.rs             ~620   Expr::to_polars, root calls, method chains, when/then
│   ├── catalog.rs             ~790   FormulaFunctionDefinition, POLARS_FORMULA_FUNCTIONS,
│   │                                 formula_function_catalog
│   ├── complete.rs            907    moved verbatim from formula_complete.rs
│   └── generated_bindings.rs  2,661  moved verbatim; generated, never hand-edited
│
├── engine/
│   ├── mod.rs                 ~20
│   ├── compute.rs             ~270   compute_tables, compute_canvas_views,
│   │                                 materialized_for_view, fingerprints, relink_artifacts
│   ├── plan.rs                ~400   materialize_table_lazy/frame, view_filter_predicate,
│   │                                 apply_view_sort, table_view_lazy, find_table_view
│   ├── page_cache.rs          ~95    SortedPageCache and its key/entry types
│   ├── build.rs               ~230   build_table, build_imported_table, build_artifact_table,
│   │                                 build_table_with_types
│   ├── table.rs               ~480   impl TableObject — layers, dependencies, accessors
│   └── values.rs              ~390   scalar formatting/parsing/comparison, type inference,
│                                     polars_value_at, framework_type_from_polars
│
├── data/
│   ├── mod.rs                 ~15
│   ├── import.rs              ~270   read_import_frame, create_data_artifact,
│   │                                 normalize_delimited_artifact, sha256_file
│   └── export.rs              ~60    export_table_csv
│
├── validate.rs                ~300   unique keys, joins, cell overrides, style targets,
│                                     categories, acyclicity, recursion
│
└── collaboration/
    ├── mod.rs                 ~15
    ├── paths.rs               ~40    CollaborationPaths
    └── journal.rs             ~260   EventJournal, MergeResult, EventId, OperationEvent,
                                      envelope validation
```

Forty-seven files, one of them generated. Excluding the generated bindings, the largest is
`formula/catalog.rs` at ~790 lines — and it is a single const table, the kind of file
whose length costs a reader least. No hand-written logic file exceeds ~620.

The layering is meant to be readable as a dependency order: `model` depends on nothing,
`formula` on `model`, `engine` on both, `operation` on all three, and `store` on
everything. If a `use` ever needs to point upward, that is the signal that something
landed in the wrong module.

### Mechanics that make this safe

Four properties of the current code make this a mechanical refactor rather than a design
exercise, which is why it is worth doing now rather than after the next feature:

**Almost every struct field is already `pub`.** Exactly seven private fields exist, five
on `Store` and two on `EventJournal`. Both types move whole into a single module
(`store.rs`, `collaboration/journal.rs`) alongside every method that touches them, so the
split needs no accessors and no visibility change. Every other type — all of `model/` —
is transparent already.

**`impl Document` may be split across modules.** Rust allows inherent `impl` blocks for a
type anywhere in the defining crate. `Document` stays in `model/document.rs` while its
methods live in `operation/prepare/`, `engine/compute.rs`, and `validate.rs` — with no
trait, no newtype, and no signature change.

**The public surface is flat and small.** Consumers write
`use framework_core::{Document, Operation, Store, …}`. Preserving that is one block of
`pub use` in `lib.rs`. The three consumers — `src-tauri`, `framework-mcp`, and the
`generate_sample_documents` example — should not need a single line changed, and that is
the acceptance test for the refactor.

**The visibility churn is bounded and known.** 57 private free functions and 70 private
methods exist today. Only those whose callers land in a different module need
`pub(crate)`; the compiler enumerates them exactly. Resist the reflex to reach for `pub`
when `pub(crate)` is what the error is asking for.

### Ordering

1. Add `Store::document` and publish `Document::table`. Two lines, no behavior change.
   Land it separately so the API addition is reviewed as an API addition.
2. Move the 64 now-portable tests to `tests/`. Nothing else changes; `cargo test` should
   report the same 72 tests passing, 64 of them from new binaries.
3. Extract leaf modules with no inbound crate dependencies: `error`, `collaboration`,
   `model`, `engine/page_cache`, `engine/values`, `data`. Each is its own commit.
4. Extract `formula/`, including moving `formula_complete.rs` and
   `generated_expr_bindings.rs` under it.
5. Extract `engine/` and `validate`.
6. Split `operation/`, taking `prepare` and `apply` one family at a time. This is the
   only step with real diff volume, and it is last so it lands on a codebase where
   everything else is already stable.
7. Redistribute the eight remaining in-crate tests to their modules.

After each step: `cargo test`, `cargo clippy`, and confirm `git diff --stat` on
`src-tauri/` and `crates/framework-mcp/` is empty.

## The TypeScript

The answer is yes for `api.ts`, qualified yes for `App.tsx`, and the two are worth doing
for different reasons.

### src/lib/ is already the pattern

The repo has already solved this problem once. `gridNavigation.ts` (279 lines) has
`gridNavigation.test.ts` (275). `pagedWindow`, `columnFormatting`, `viewSorting`,
`formulaReferences`, `formulaFunctionCatalog`, `tableVirtualization`, `parseGrid`, and
`gridClipboard` follow the same shape: a pure module beside a colocated test file, no
React, no IPC. Nine such pairs exist, totalling ~2,020 lines. That is the target the two
large files should be pulled toward.

Two test files broke the pattern in a way that was itself diagnostic.
`viewFiltering.test.ts` (160 lines) and `viewStyling.test.ts` (67) had no module named
`viewFiltering.ts` or `viewStyling.ts` — they imported `resetDemo` and `applyOperation` from
`api.ts` and, under vitest's Node environment, actually exercised the browser-preview mock.
They were tests named after a concept that had no home, reaching their subject through an
IPC façade. The proposed split below briefly gave that code a home before the adapter and
its tests were removed altogether.

### api.ts — split by responsibility, not by size

Twenty-two exported functions occupied the first ~235 lines. The remaining ~1,300 lines were
`applyMockOperation` and its supporting cast (`recalculateMock`, `recalculateDerivedMock`,
`validateMockUniqueKeys`, `sortMockViewRows`, `makeMockDocument`, and about fifteen more):
a second, thinly tested, in-memory reimplementation of the core's operation semantics for
the browser preview.

`architecture.md` already says this is "not canonical product logic." The file layout
should say the same thing:

```text
src/lib/api.ts              ~240   the Tauri IPC surface only
src/lib/preview/
├── index.ts                ~60    the adapter boundary api.ts falls back to
├── operations.ts           ~800   applyMockOperation, by operation family
├── recalculate.ts          ~300   view/derived recalculation, filters, sorts
└── fixtures.ts             ~200   makeMockDocument and the sample grids
```

This was the higher-value TypeScript change and the cheaper one. It made the real IPC
boundary readable at a glance and put a name on the shadow implementation. The resulting
separation also made the larger architectural problem plain enough to remove: these pure
`(DocumentView, Operation) => DocumentView` transforms still reproduced the Rust core and
had only the indirect coverage in `viewFiltering.test.ts` and `viewStyling.test.ts`.

The deletion eliminated the ongoing risk this assessment identified: parallel semantics
that had to track `operation/apply/` by hand, with nothing that failed when they drifted.

### App.tsx — split, but do not expect tests to follow

4,702 lines holding roughly forty components:

| Component | Lines |
| --- | --- |
| `App` | 959 |
| `TableCard` | 618 |
| `TableInspector` | 280 |
| `FieldsAsRowsTableCard` | 265 |
| `FormulaEditor` | 184 |
| `CanvasObject` | 175 |
| `DerivedTableCreator` | 143 |
| `PlotInspector` | 127 |
| remaining ~32 | ≤ 107 each |

A reasonable split:

```text
src/components/
├── App.tsx                  ~400   shell: document state, event subscriptions, dialogs
├── canvas/                         CanvasObject, LineageCords, ValueCard, PlotCard, VegaChart
├── table/                          TableCard, FieldsAsRowsTableCard, TableViewTabs,
│                                   TablePageControls, GridCell*, EditableDerivedHeader
├── inspector/                      Inspector, Table/Plot/Value inspectors, ViewSortEditor,
│                                   TableViewEditor, ColumnFormatEditor, QueryPlanPanel
├── formula/                        FormulaEditor, FormulaField, FormulaCreator,
│                                   FormulaErrorDetails
└── dialogs/                        New/Dataset/Join/Insert
```

Two honest caveats, because the Rust argument does not transfer intact:

**Extracting components does not produce tests.** `devDependencies` has `vitest` but no
`jsdom` and no `@testing-library/react`, and `vite.config.ts` declares no `test.environment`
— so vitest runs in Node with no DOM. Rendering any of these components in a test requires
adding both dependencies and a config block first. That may well be worth doing, but it is
a separate decision with its own cost, and splitting the file does not advance it.

**The testable extraction is logic, not components.** `App.tsx` currently holds pure
helpers — `styleTargetForSelection`, `effectiveTableCellStyle`, `mergedTableCellStyle`,
`resolveGridContext`, `gridRangeForFocus`, `rawGridValue`, `displayedTableRows`,
`plotRows`, `defaultPlotSpec`, `observedKeyStats`, `vegaType`. Those are `src/lib/`
material: they need no DOM, they are testable today, and moving them is the change that
actually buys coverage. Doing that first also shrinks the components, which makes the
directory split afterward a smaller diff.

So: move the pure helpers to `src/lib/` with colocated tests first, then split the
components for navigability, and treat DOM testing as its own decision.

`src-tauri/src/lib.rs` (939 lines) is a thin command adapter and does not need this yet.

## What I would not do

**Split `framework-core` into multiple crates.** Nothing here justifies separate
compilation units, and a crate boundary would force real `pub` API decisions on
internals that are currently free to move. Modules give the same navigability at none of
that cost. Revisit only if compile times become the binding constraint.

**Split by artifact type rather than subject.** A `types.rs` / `impls.rs` / `helpers.rs`
layout groups things that change independently and separates things that change together.
The tree above puts `TableDerivation` beside its joins and its inputs because they are
edited in the same sitting.

**Break up `formula/catalog.rs` or the generated bindings.** Long is not the same as
complex. A const table and a generated file are both read by search, not by scrolling.

**Do this alongside a feature.** Every step above is behavior-preserving, and that is the
property that makes it reviewable — `cargo test` and an empty diff in the consumer crates
are the whole review. Mixing in one behavior change forfeits that.
