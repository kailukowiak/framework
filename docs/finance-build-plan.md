# Finance build plan: functions, time spine, evaluate-under-overrides

Status: proposed 2026-09-11, drafted from a design conversation with Kai.
Implementation update 2026-09-12: the recurrence precision fix and phase 1a
(`pv`, `fv`, `pmt`, `ipmt`, `ppmt`, `nper`, `npv`, `xnpv`) are implemented.
Phase 1b adds `irr` and `xirr`, including namespace and receiver calls;
Price a deal now solves its return and checks the XNPV residual.
Implementation update 2026-09-12: phase 1 is complete with `rate`, `mirr`,
`effect`, `nominal`, `sln`, `db` and `ddb`, including namespace and receiver
calls. Only the grouped-IRR stretch remains planned. Iterations and
brackets are retained internally, but exposing convergence diagnostics in the
dependency trace is deferred: that trace has no runtime-result payload today.
The bounded root-search policy is documented in the function reference; it
does not promise to discover every root or beat Excel on every input.
The branch/worktree sequencing notes below
record the original plan; the plan and lessons have since merged to main.
Lives on the `finance` branch in its own worktree so it can proceed beside
the in-flight ML batch on `main`. Refines the *Target workflows
(finance/accounting wedge)* and *Workflow primitive backlog* sections of
[ProjectSpec.md](../ProjectSpec.md); those sections should point here once a
phase lands, not before.

The plan covers the finance audience: modellers, FP&A, corporate finance.
The accountant audience gets its own plan (exact Accounting type,
reconciliation with reviewed matches, validation) and is out of scope here
except for the boundary decision below.

## Boundary decision: Currency and Accounting are separate types

Decided 2026-09-11. `DataType::Currency` stays as it is: f64-backed, every
function in the catalog available, the type a model is built on. A separate
`Accounting` type will carry Decimal128 with a declared scale, dimensional
rules, visible rounding steps and allocation, for ledgers and close work.

Two consequences for this plan:

- Nothing here waits on Polars decimal coverage. Every phase below runs on
  Currency and Number.
- The conversion rules are fixed now so neither plan designs around them:
  Accounting to Currency is an explicit cast; Currency to Accounting is a
  visible rounding step with a declared mode, shown in the chain the way a
  Read type override is.

## Phases

Each phase stands alone, ships something a finance person would notice, and
lands with a tutorial document and an MCP smoke scenario. Build order is
leverage over effort, with one exception noted under *Sequencing*: phase 1
has no overlap with the ML batch and can start immediately; phases 2 and 3
touch the same files the ML batch is changing and branch from the commit
that lands it.

| Phase | Rough size | Unlocks |
|---|---|---|
| 1. Financial function pack | 1 to 2 weeks | DCF, loan and lease schedules, deal IRR |
| 2. Period-aware time spine | 3 to 4 weeks | driver-based forecast, budget vs actual by fiscal period |
| 3. Evaluate under overrides | 2 to 3 weeks engine, plus UI | scenario tables, sensitivity grid, goal seek |
| 4. Present mode with controls | 1 to 2 weeks | interactive model handoff |
| 5. Declared iterative solve | later | three-statement model, debt schedule |

### Phase 1: financial function pack

Implemented 2026-09-12: the full pack below is in the catalog with
`finance.` and receiver spellings, Excel argument order, and inline errors;
only the grouped-IRR stretch remains future work.

**Purpose.** Remove the day-one disqualifier. "Does it have XIRR" is the
first question a finance person asks, and today the answer is no: nothing
under `pmt`, `npv`, `irr` or `xirr` exists in the catalog. This phase does
not change what kind of model can be built; it makes the first model
possible without leaving for Excel.

**Scope.**

- Closed forms as expansion functions (ProjectSpec mechanism 3, a catalog id
  whose compile arm emits a composed Polars tree): `pmt`, `ipmt`, `ppmt`,
  `pv`, `fv`, `nper`, `npv`, `xnpv`, `effect`, `nominal`, and the
  depreciation trio `sln`, `db`, `ddb`.
- Solvers as native scalar functions (mechanism 4, plain Rust over a
  collected series in the scalar context, where `lookup` already lives):
  `irr`, `xirr`, `rate`, `mirr`. Use a bracketed root finder (Brent, or
  bisection with a Newton step inside the bracket), never bare Newton from a
  guess. Excel's XIRR fails on ordinary cash flows; a solver that converges
  or says exactly why it cannot is a small differentiator and a large trust
  gain.
- Excel names as catalog aliases so typing `PMT` finds `pmt`, the way
  `sumif` already signposts `.filter(...).sum()`.
- The NPV convention, documented at the function: `npv` matches Excel and
  discounts the first flow by one period, because the alias promises Excel
  behaviour and a silently different number is worse than a documented
  quirk. `xnpv` discounts by actual dates and is what the docs recommend.
- Stretch, only if it fits: `irr` and `xirr` as grouped aggregates so a
  Summarize step yields one IRR per deal. Excel cannot do this without
  helper ranges. This needs the solver to run per group, which is an apply
  over a list column rather than a scalar native; cost it before committing.

**Design notes.**

- Cash flows arrive as a column and, for the dated forms, a date column:
  `` xirr(`Flows`.`amount`, `Flows`.`date`) `` in Scratchwork. Sign
  convention follows Excel (outflows negative).
- A solver result carries its convergence facts (iterations, bracket, final
  residual) into the formula trace so a wrong answer can be inspected, not
  only reported.
- Argument order follows Excel where Excel has the function, so an alias
  is a real alias and not a signpost to a different signature.

**Owned paths.** `crates/framework-core/src/formula/catalog.rs`,
`formula/compile.rs` (new arms; consider a `formula/financial.rs` for the
solvers), `docs/formula-function-catalog.md` and the generated catalog, a
tutorial under `tutorials/`, an MCP smoke scenario under `tools/mcp-smoke/`.

**Gates.**

- Expected values come from an independent source, never from the
  implementation under test: a hand-verified amortisation table, published
  textbook examples, or a pinned reference library run outside the crate.
  Same discipline as [the ML build lanes](ml-build-plan.md).
- Every closed form matches Excel to the cent on the documented examples,
  including the NPV first-period convention.
- `xirr` converges on the cases where Excel's does not, and refuses with a
  named reason (no sign change, empty bracket) rather than returning a
  number.
- Every alias resolves in autocomplete and the generated catalog docs.
- Tutorial: [Price a deal](../tutorials/price-a-deal/README.md). Its
  section 5 is this phase's acceptance target; the answer key is rebuilt
  with those formulas when they compute.

### Phase 2: period-aware time spine

Slice 1 landed 2026-09-12 on the timespine branch: the frame-level period
declaration (`FramePeriod` + `SetFramePeriod`, validated unique per
partition with no missing dates), monthly `period_index(date, fy_start=1)`,
and `prior(expr, n=1, fy_start=1)` as a left self-join on `index - n`
within the declared partitions. `prior` outside a calculated column is
refused naming the frame and the fix; a missing earlier period reads blank.
Slice 2 landed 2026-09-12 on the same branch: the fiscal-calendar date
functions `fiscal_year`, `fiscal_quarter`, `fiscal_period`, `period_start`,
`period_end` and `add_periods`, each with `finance.` and receiver spellings.
They read the date itself, so they need no declaration and work in
Scratchwork; `fy_start` follows the slice 1 literal-or-named-value rule, and
`add_periods` accepts a per-row count with EDATE month-end clamping.
Slice 3 landed 2026-09-12 on the same branch: the declaration-gated window
aggregates `ytd(expr, fy_start=1)`, `ttm(expr)` and
`same_period_last_year(expr)` as self-joins on the period index — range
joins with per-row sums for the first two, the prior machinery at offset
twelve for the third. Only `ytd` takes `fy_start`; the twelve-period forms
are index-relative and year-start-invariant. Windows sum the periods
present and blank only a window with no readable value; every call in a
step derives from the pre-pass input snapshot and joins back by the
declaration's natural keys, keeping the cost linear in the number of calls.
Slice 3 also closed two cross-surface gaps the slices exposed: the
`finance.` namespace spelling lifts like the root call, and the grid's
calculated column accepts period-relative formulas (typed from the value
they read) with the same save-time declaration refusal as the chain.
Slice 4 landed 2026-09-12 on the same branch and closes the phase:
document calendars (`Calendar` with year start, week pattern, year-end
rule, year labelling, weekend and inline holidays; add, update, remove
and set-default operations with exact undo; MCP tools), `fiscal_week`,
`workday` and `networkdays` as native scalars, and a `calendar` keyword
on the fiscal date functions that reads the week table under a retail
pattern. Bare calls read the document default, and an explicit `fy_start`
still wins over the calendar's. Years are numbered by the calendar year
they end in unless the calendar says otherwise — the tutorial's answer
key demanded it, and the NRF labels its calendars by the start, so the
convention lives on the calendar rather than in the function. The
Driver-based forecast tutorial walks the whole phase: its section 7
replaces the hand arithmetic with the bare calls, adds the trailing
twelve months, checks an NRF retail week, and repeats the deletion test
against the answer key. Calendar creation stays MCP-only (the start file
carries both calendars); declaring a period is one choice in the frame
menu. Retail blocks do not yet reach the period declaration or
`period_index` — `prior` and the windows stay monthly — and holidays stay
an inline list rather than a frame.
**Purpose.** Nearly every finance model is a monthly or quarterly series
with period-relative logic, and a finance person's first FrameWork document
is a forecast. Today the spine is a date `sequence` generator and the only
prior-period tool is a positional `.shift`, which silently depends on sort
order, which is exactly what ProjectSpec §13 refuses. This is also where
FrameWork can be better than Excel rather than merely equal: Excel has no
notion of a period, every model rebuilds it from column headers, and every
SUMIFS over dates is a re-derivation.

**Scope.**

- A calendar object on the document: fiscal year start month; week pattern
  of calendar months, 4-4-5, 4-5-4 or 5-4-4; the 52/53-week year-end rule
  (last day of month, last given weekday, nearest given weekday); an
  optional holidays frame. A document has a default calendar and may name
  others. Operations: add, set, remove, set default; exact undo.
- Calendar-aware date functions: `fiscal_year`, `fiscal_quarter`,
  `fiscal_period`, `period_start`, `period_end`, `period_index`,
  `add_periods`, `workday`, `networkdays`. Each takes an optional calendar
  argument and defaults to the document's. `period_index` is the one that
  matters: an integer period number since a fixed epoch makes every period
  offset exact arithmetic rather than date math, and is what the
  period-relative functions join on.
- A declared period column on a frame, with optional partition keys,
  validated unique per partition and period. This is §13 applied to time:
  the same explicit declaration a recurrence needs for order. Recommended
  shape: a frame-level property (like a dictionary's enforced key), not a
  chain step, validated in `validate.rs`; settle this at the start of the
  phase.
- Period-relative functions that refuse to run without that declaration,
  with an error naming the frame and how to declare it: `prior(expr, n)`,
  `same_period_last_year(expr)`, `ytd(expr)`, `ttm(expr)`. They compile to
  a self-join on `period_index - n` within the partition keys, a window
  over fiscal year, or a rolling sum by period index with an integer
  window, never to a positional shift. If the declared column is validated
  dense and the chain is sorted by it, a shift is a legitimate optimisation
  the compiler may choose; the user never spells it.
- Actual/forecast cutover is a recipe on existing pieces, not a new object:
  a generator spine, actuals joined by period key, a close-date value that
  scenarios can switch, and entry columns for typed overrides keyed by
  period. It earns the phase's tutorial rather than a feature.

**Design notes.**

- Polars already has `month_start`, `month_end`, `offset_by`, `range` and
  `rolling_window` enabled. Its `business` feature (business-day counting
  and offsets with a week mask and holidays) is not enabled; verify what
  0.55.2 provides under it before hand-writing `workday` and
  `networkdays`, since a Polars expression beats a scalar native for these.
- 4-4-5 and its siblings are arithmetic over week numbers plus the
  year-end rule; a generated internal periods table joined on date is the
  simplest correct implementation and makes the fiscal functions the same
  code path as `lookup`. Hand-rolled arithmetic can replace it later if
  cost demands.
- A crosstab display already shows periods across; the phase adds nothing
  to presentation.
- The period declaration and the calendar id both travel in the `.fw`;
  calendars are document objects, so they replicate under the existing
  operation model with no special case.

**Owned paths.** `crates/framework-core/src/model/` (new `calendar.rs`,
period declaration on `frame.rs`, document fields on `document.rs`),
`operation/kinds.rs` and its prepare/apply/invert arms, `engine/plan.rs`
and a new `engine/period.rs`, `formula/catalog.rs` and `compile.rs`,
`validate.rs`, the generated TypeScript bindings, the frame inspector and
a small calendar editor in `src/`, the MCP operation catalog, a tutorial,
a smoke scenario.

**Gates.**

- A 4-5-4 calendar matches a published retail calendar (the NRF calendar
  is public) for every week of three consecutive years including a
  53-week year.
- Fiscal functions with an offset year start (say, February) put the
  right dates in the right quarter across the year boundary.
- The §13 test: every period-relative function returns identical results
  on a frame with gaps and shuffled row order as on a dense, sorted copy.
- `ytd` resets at the fiscal year boundary under an offset calendar;
  `ttm` spans it.
- A frame without a period declaration gets an error naming the frame
  and the fix, and no other operation on that frame is refused.
- Tutorial: [Driver-based forecast](../tutorials/driver-forecast/README.md).
  Its section 7 is this phase's acceptance target, including the deletion
  test that separates "the row above" from "the period before".

### Phase 3: evaluate under overrides

**Purpose.** One engine primitive that unlocks three features finance
people ask for by name: a scenario comparison table, a sensitivity grid,
and goal seek. Scenario bundles landed 2026-09-02; the Calculation Matrix
is already most of a sensitivity grid's surface. What is missing is the
ability to compute a result under a set of assumption values without
changing which scenario the document is reading.

**Scope.**

- The primitive: compute any named result (a block line, a summary, a
  frame cell) with a map of value-object overrides, without activating a
  scenario. `Document::effective_value_raw` is already the single answer
  to "which number is this", so this is plan building parameterised by an
  ephemeral scenario. Cache by lineage fingerprint, which already includes
  effective values, so only frames downstream of the overridden values
  recompute and overriding a value nothing reads costs nothing.
- Scenario comparison: an `under(scenario, expr)` function in the formula
  language, plus a *Compare scenarios* gesture that builds an ordinary
  frame with scenarios across and chosen outputs down. Today you switch
  the active scenario to see each one; side by side is what gets
  presented.
- Sensitivity grid: the Calculation Matrix with each axis bound to a value
  object. The axis formula supplies the candidate list (a written list or
  a `sequence`), and the body is evaluated under both substitutions. Show
  the evaluation count and elapsed time on the card and allow cancel, per
  the trust-at-scale rule; if cancellation has not landed for long
  computations by then, cap the grid and say so.
- Goal seek: a `solve` function in Scratchwork taking an expression, a
  target, the value to vary and a bracket, returning the input as a scalar
  with its convergence facts in the trace. An *Apply* action on the line
  turns the answer into an ordinary `SetValue`, so undo and history get it
  free. One variable only; simultaneous solve is phase 5.
- A tornado chart falls out of the same primitive and the plot layer: for
  each assumption, evaluate at its low and high, plot the swing. Optional.

**Design notes.**

- `under` accepts either a scenario name or an inline map of value
  overrides; the matrix and goal seek use the map form internally, so
  there is one evaluation path.
- The evaluation must be side-effect free: no materialisation, no
  snapshot, no change to `active_scenario`. A sensitivity grid over a
  model that reads a million-row frame is expensive by nature; the
  primitive reports cost rather than hiding it.
- The solver is the phase 1 root finder, reused. Goal seek is the same
  bracketed search over a function whose evaluation happens to be a
  document recompute.

**Owned paths.** `engine/compute.rs` and `engine/plan.rs` (the parameterised
build), `model/scenario.rs`, `model/calculation_matrix.rs` (axis binding),
`formula/` (`under`, `solve`), the matrix card and a compare-scenarios
gesture in `src/`, MCP, a tutorial, a smoke scenario.

**Gates.**

- Property test: for every scenario in a document, `under(scenario, x)`
  equals activating that scenario and reading `x`, for a sample of
  results across frames, blocks and summaries.
- Every sensitivity grid cell equals the corresponding `under` call.
- Goal seek lands within tolerance, reports iterations, and refuses with
  a reason when the bracket does not contain a root.
- Overriding a value that nothing reads produces no recompute (measured
  by the cache, not by timing).
- Tutorial: [Scenarios, sensitivity and goal seek](../tutorials/scenarios-and-sensitivity/README.md).
  Its section 5 is this phase's acceptance target.

### Phase 4: present mode with controls

Already specified under *Present mode* in ProjectSpec. Sequenced after
phase 3 because a slider over a scenario value beside a live sensitivity
grid is the demo of the whole product. No new engine work: a control is a
view over an existing value object, and moving it is an ordinary
operation.

### Phase 5: declared iterative solve

Interest on average balance, the debt sweep, and the other circularities
a three-statement model carries. A named solve step with declared
unknowns, a tolerance, an iteration cap and a convergence report, visible
in the chain and in the trace. Never Excel's silent "enable iterative
calculation". Last, as ProjectSpec already says, because the workflows
above are worth more and this one is easiest to get subtly wrong.

## Findings from building the lessons (2026-09-11)

The three lessons were generated through `Store::apply` against the current
catalog, with every checkpoint asserted against numbers computed
independently in Python. Building them surfaced four things worth fixing
regardless of the phases above; the READMEs name each one as product
feedback rather than a lesson error.

- **A recurrence keeps the type of its first value.** `recur(400000, …)`
  seeded from an integer literal or integer column silently truncates every
  step to a whole number, so an amortisation schedule loses its cents and a
  compounding forecast drifts. The lessons work around it by seeding from a
  currency-typed column or a `400000.0` literal. The engine should promote
  the recurrence to the wider of the seed's and the step's types, or refuse
  with a message.
  **Fixed in 1a:** seed and step resolve a common type before evaluation,
  including partition restarts and integer column seeds.
- **A Calculation Matrix body cannot read a table that has calculated
  columns unless it is materialised**, while a block line reading the same
  column can. The rate scan in the first lesson is a block for this reason.
  Either the matrix should get the block's evaluation path or the error
  should say why the two differ.
- **Generated Polars methods demand their optional arguments.**
  `.dt.total_days()` and `.cum_sum()` fail with "expects fractional" and
  "expects reverse"; the working spellings are `.dt.total_days(False)` and
  `.cum_sum(False)`. Defaults for generated optional arguments would remove
  a class of first-formula failures.
- **Date minus date works, but only by falling through to Polars.** The
  duration it produces has no display type of its own, which is fine once
  `.dt.total_days` is applied and surprising before it. Phase 2's period
  functions make most such subtractions unnecessary; a `days_between`
  alias would cover the rest.

## Sequencing against the ML batch

The ML batch on `main` (uncommitted as of 2026-09-11) touches
`engine/build.rs`, `compute.rs`, `frame_chain.rs`, `plan.rs`,
`model/document.rs`, `model/frame.rs`, `operation/kinds.rs`, the
prepare/apply/invert arms, `store.rs`, the bindings, the MCP catalog and
`src/App.tsx`. It touches nothing under `formula/` or `data/`.

- **Phase 1 starts now** on the `finance` branch from the current `main`.
  Its footprint is `formula/`, docs, a tutorial and a smoke scenario.
  Merge-back conflicts are limited to `CHANGELOG.md` and possibly the
  generated catalog doc.
- **Phases 2 and 3 wait for the ML batch to commit**, then continue on the
  same branch after rebasing onto that commit. Both need `kinds.rs`,
  `document.rs`, `plan.rs`, `compute.rs` and the bindings, which is the ML
  batch's footprint; starting them earlier means a large hand-merge of the
  `Operation` enum, which the house rule says to serialise.
- Tutorial scenario tests go in a new test file rather than
  `tests/tutorials.rs`, which the ML batch is editing; the bundled-tutorial
  discovery test picks up new tutorial folders without edits.

## Worktree and merge-back

- The worktree is `../FrameWork-finance`, branch `finance`. It has its own
  `target/` and `node_modules`; the first build is cold, and Polars is most
  of it. Do not share `CARGO_TARGET_DIR` between the trees: the e2e bundle
  path and the ts-rs export are per-tree, and sharing crosses them.
  `.cargo/config.toml` resolves the bindings directory relative to the
  tree, so `cargo test` regenerates the right `src/lib/bindings`.
- Bringing `main` in: rebase the branch onto `main` after each ML commit
  that matters, resolve `kinds.rs` by hand, and regenerate the bindings
  with `cargo test` rather than resolving generated files.
- Merging out: a merge commit onto `main`, matching the house style
  (`Merge finance: <what landed>`), after `cargo test --workspace`,
  `npm test`, `npm run lint:rust` and `npm run test:e2e` when UI changed.
  Move the changelog drafts below into `## Unreleased` at merge time so
  the branch never edits that section.
- Remove the worktree after the last phase merges, or keep it for the
  accounting plan.

## Changelog drafts

Kept here until merge so `CHANGELOG.md` does not conflict on every
rebase. Written for the person using FrameWork, per the changelog's own
rule.

- *Phase 1:* (draft at merge)
- *Phase 2:* (draft at merge)
- *Phase 3:* (draft at merge)
