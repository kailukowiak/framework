# FrameWork language design conversation

**Status:** thinking document, not a language specification

**Origin:** conversation beginning with “Did I, along with LLMs (mostly LLMs), just make a new programming language?”

**Purpose:** preserve the reasoning, tensions, and open questions before more syntax is added

## Why this conversation happened

FrameWork already has many of the components normally associated with a small programming language:

- formulas with their own syntax;
- scalar, column, list, and table-shaped values;
- types such as integers, numbers, strings, booleans, dates, and categories;
- name resolution for tables, columns, and formula-block lines;
- parsing, type checking, autocomplete, argument help, and errors;
- a growing standard library of numeric, string, date, aggregate, window, filtering, casting, and sequence operations;
- compilation into Polars expressions and lazy plans;
- a reactive dependency graph stored in a workbook;
- visual authoring surfaces that create the same operations as text.

That makes the answer to the original question “yes, in an important sense.” FrameWork has become a typed, reactive, table-oriented domain-specific language embedded in a spreadsheet interface.

However, recognizing that a language exists does **not** imply that FrameWork should pursue novelty for its own sake or grow into a general-purpose programming language. The more important question is what minimum language is required to let spreadsheet work remain simple at the beginning and continue scaling after Excel becomes painful.

## The product thesis

The working product thesis from the conversation is:

> Many Excel users chose Excel because it has an exceptionally quick initial learning curve. Some of those users later become experts and are constrained by the same model that made the beginning easy. FrameWork should be more pleasant and powerful for beginners, then permit substantially more complexity without demanding it up front.

This suggests a **gentle ramp without Excel's eventual cliff**.

The language is therefore infrastructure, not the product. Most people should encounter direct manipulation first:

1. import or paste a table;
2. sort, filter, format, or transform through familiar gestures;
3. add a calculated column by pointing at inputs;
4. inspect the resulting operations in Wrangle;
5. use longer formulas or local names only when the calculation requires them;
6. open a notebook or source representation only when it becomes useful.

Complexity should be available, not ambient.

## The intellectual family tree

FrameWork does not need to invent every concept it uses. The useful influences identified in the conversation are:

### Excel

- immediate recalculation;
- direct manipulation and pointing to references;
- formulas that can begin very simply;
- a visible result next to the calculation;
- a functional and reactive dependency model, despite the grid presentation.

### Pluto.jl

- the document is a dependency graph rather than a sequence of commands;
- definitions recalculate their dependants automatically;
- textual order is not execution order;
- results live beside their definitions;
- parameters can become interactive controls;
- hidden mutable state is discouraged;
- the document is simultaneously program, interface, and output.

Pluto was an important inspiration for FrameWork itself. This makes the notebook/block direction an extension of an existing product idea rather than an unrelated advanced editor.

### Julia, DataFrames.jl, and DataFramesMeta

- expressive data manipulation;
- functions and transformations as composable values;
- split-apply-combine;
- useful column selectors;
- local bindings that make complex calculations readable;
- chain-oriented dataframe work.

### dplyr

- a compact and understandable grammar of table verbs;
- strong names for common operations: filter, select, mutate, group, summarize, arrange, and join;
- expressions evaluated in the context of the table being transformed.

### LINQ

- typed, composable transformations;
- deferred execution;
- a clear conceptual model of one transformation producing the input to the next;
- reusable queries rather than manual intermediate data.

### Pandas and Polars

- familiar fluent dot syntax;
- expressions over whole columns rather than copied cell formulas;
- dataframe-shaped operations;
- in Polars specifically, the lazy execution engine FrameWork already compiles into.

The tentative description that emerged was:

> Excel's grid and approachable reactivity, Pluto's dependency-driven notebook model, dplyr/Julia/LINQ table composition, and Polars execution.

## Why not simply use Julia as the workbook language?

Julia is an excellent general-purpose technical language, and its dataframe ecosystem contains many ideas worth borrowing. The conversation nevertheless leaned against adopting Julia as FrameWork's core language or runtime.

The deciding issue is not whether Julia is expressive enough. It is whether FrameWork can completely understand a workbook operation.

FrameWork needs to be able to:

- render an operation as a Wrangle step;
- construct the same operation through clicks;
- provide type-specific autocomplete and contextual help;
- preserve stable table and column identity through renames and source replacement;
- explain lineage;
- apply undo, collaboration, and validation at meaningful structural boundaries;
- compile to a lazy Polars plan;
- reconstruct readable text from visual edits;
- open a shared workbook without executing arbitrary code.

Arbitrary Julia can produce a table without exposing a structure that FrameWork can reliably project back into those surfaces. Embedding Julia would also add another runtime, package environment, deployment concern, and execution semantics beside the existing Rust/Polars engine.

The preferred direction is therefore:

- borrow proven syntax and semantics from Julia and its helper packages;
- keep a small FrameWork-owned AST that maps exactly onto workbook concepts;
- consider Julia, Python, SQL, or WASM later as explicit, permissioned, opaque extension nodes with declared inputs and outputs;
- never let a normal `.fw` formula become an implicit arbitrary-code execution boundary.

FrameWork should remain a product for an Excel user who may gradually discover that they are programming, not a Julia IDE whose users sometimes want a grid.

## Functional semantics with fluent syntax

Dot notation can look object-oriented without requiring object-oriented semantics. The proposed interpretation is functional composition written in execution order.

```text
`Amount`
  .filter(`Region` == "East")
  .sum()
  .round(2)
```

Conceptually, that is equivalent to nested functions:

```text
round(sum(filter(`Amount`, `Region` == "East")), 2)
```

The fluent form is easier to read from top to bottom and gives autocomplete a valuable type boundary after every dot. The receiver's type determines which methods are valid next.

Examples of useful typed transitions are:

```text
StringColumn.str.to_uppercase()  -> StringColumn
DateColumn.dt.year()             -> IntegerColumn
NumberColumn.sum()               -> Number
Table.group_by(...)              -> GroupedTable
GroupedTable.summarize(...)      -> Table
```

After `group_by`, autocomplete should offer grouped-table operations, not arbitrary string methods or unrelated table operations. Fluent syntax is therefore also a discoverability mechanism.

### The dot is the pipe

*(from a follow-up conversation)*

The attraction of a pipe operator — `x |> f(a)` — is that it threads a value through any function, including user-defined ones, in reading order. The attraction of dot chaining is that the receiver's type gates autocomplete. These are not two features; they are one operation with two lookup rules, and the proposal is to have exactly one of them, defined so that it does both jobs:

> `x.f(args)` means `f(x, args)`, for every catalog function and every function or pipeline defined in the workbook.

This is uniform function call syntax in its closed-world form (D and Nim, not C++'s abandoned open-world version — a single catalog avoids the lookup ambiguities that killed it there). What it buys:

- User pipelines become dot-callable with no `.pipe` adapter: `Orders.Clean(minimum = 10_000).group_by(...)`. The chain never changes style when it crosses from built-in verbs into workbook definitions, and the earlier open question about whether `.pipe` is necessary answers itself: it is not.
- The multiline leading-dot chain already *is* the pipeline aesthetically. The pipe-feel is a formatting convention, not an operator.
- Autocomplete after the dot offers everything whose first parameter accepts the receiver — built-ins and the workbook's own definitions in one type-gated list.
- Receiver-first order matches gesture order. Direct manipulation is "select the thing, then choose the action"; `Orders.` followed by a verb list is the same move in text. Verb-first and pipe-first syntaxes match how programmers compose functions, not how spreadsheet users act.

Why not dots *and* `|>` together: two spellings of the same call violates the canonical-formatting rule — one way to write each operation, so reading the projection is the tutorial. The pipe also visually announces a programming language to exactly the audience that should not feel they are in one, and pipe culture invites point-free style and bare function references — the front door for first-class functions, which the fence bans. Dots create no such pressure; `.f` without parentheses is simply a syntax error.

Two supporting conventions:

- Built-in verbs are lowercase (`filter`, `group_by`); workbook definitions are Capitalized (`Clean`, `PostedOrders`), matching how tables are already named. Collisions between user names and catalog names are then visible at a glance and resolvable by rule.
- The catalog records each function's canonical rendering. Most verbs render fluent; a few read better prefix (`if(...)`, constructors). Typing accepts any familiar form — `SUM(`, `round(x, 2)`, `x.round(2)` — and the formatter normalizes on commit, so saved documents always read one way. Aliases teach; the canonical form is what persists.

`.str` and `.dt` are catalog namespaces, not field reads: they group completions and exist only in fluent position.

#### Addendum: receiver-first does not mean dotting every function

The strongest version of the rule above is slightly too broad. The useful principle is:

> `x.f(args)` is the canonical composition syntax whenever `f` declares a natural receiver compatible with `x`. It lowers to `f(x, args)`. Functions without a natural receiver retain canonical prefix syntax.

This keeps the dot as the one pipeline mechanism without forcing an arbitrary first argument to become the “object” in every expression.

Transformations with a clear subject read naturally as fluent calls:

```text
Orders
  .filter(...)
  .with_columns(...)
  .group_by(...)
  .summarize(...)

`Amount`
  .fill_null(0)
  .round(2)
  .sum()
```

Constructors, conditionals, and symmetric combinators usually do not have a natural receiver and should remain prefix calls:

```text
sequence(1, 12)
if(condition, yes, no)
coalesce(a, b, c)
concat_str([first, last], separator = " ")
sum_horizontal([revenue, cost])
```

Forms such as `1.sequence(12)`, `condition.if(yes, no)`, or `first.concat_str(last)` would technically fit a universal receiver-first rewrite but communicate the operation less honestly. One calling convention underneath does not require one visual shape for every kind of function.

This refinement preserves the main simplification:

- there is no separate `|>` grammar;
- reusable workbook pipelines need no `.pipe(...)` adapter;
- direct manipulation maps mechanically to `receiver.operation(arguments)`;
- Wrangle and long-form text use the same chained structure;
- autocomplete after a dot is gated by the receiver's type and shape;
- built-in transformations and workbook-defined pipelines share one resolution path.

#### Response: agreed — and none of this needs deciding early

Two notes on the addendum, from the follow-up discussion.

First, "has a natural receiver" should be a **catalog fact**, one field per function, rather than a grammar rule. It is the same single bit three consumers need: the formatter (canonical fluent versus prefix), autocomplete (whether to offer the function after a dot), and documentation. Polars reached the same conclusion about conditionals from the other direction: `when/then/otherwise` exists because `condition.if(yes, no)` has no honest subject.

Second, the future-proofing question — settle canonical forms now, or learn from use and break later — has a structural answer: **later is nearly free, because documents persist expression trees, not formula text.** Text is regenerated from the tree on every view, so canonical-form choices are parser and renderer *policy*, not storage format. Changing them later means teaching the parser to accept both spellings (it already accepts prefix and fluent side by side) and changing what the renderer emits; no stored document breaks, because stored documents get re-spelled on open. The live proof in the codebase: renaming a value rewrites every rendered reference to it — chain formulas, text-card holes — without those documents being edited, because references are ids and spelling is projection.

The one discipline that keeps this true, and the only thing to enforce *now*: **no new feature may persist formula text as its sole truth.** Parse at save, store the tree, render back on view. The two places that keep typed text (block-line sources, broken holes) are deliberately tolerant scratch surfaces, and both keep a parsed form beside the text. Hold that line and syntax remains policy forever; lose it and every future spelling change becomes a document migration.

The current implementation is already close to this model. The parser represents a dot expression as a receiver, method path, and arguments, while completion parses and types the receiver before offering methods. The remaining architectural work is to make receiver compatibility explicit rather than infer it from catalog categories and namespaces.

A future catalog entry should be able to describe:

```text
name: filter
receiver: Table
arguments:
  predicate: BooleanColumn
returns: Table
canonical_style: fluent
```

```text
name: sequence
receiver: none
arguments:
  start: Number
  stop: Number
  step: Number
returns: List<Number>
canonical_style: prefix
```

Receiver metadata is not free implementation work, but it replaces several heuristics FrameWork already needs for accurate autocomplete, contextual argument help, UI generation, and block type checking. The saving is architectural: methods, pipes, reusable pipelines, and visual transformation adapters do not grow into separate composition systems.

Adopt this as a design constraint without immediately refactoring the compiler. The useful groundwork is:

1. record accepted receiver types and shapes in the catalog;
2. record each function's canonical rendering style;
3. resolve built-in and workbook-defined callables through one typed interface;
4. lower fluent calls to receiver-first calls during resolution or compilation;
5. test the model across columns, tables, grouped tables, constructors, combinators, namespaces, and one workbook-defined pipeline;
6. defer canonical multiline formatting until block or long-form editing begins.

The practical benefit is therefore not a large deletion of compiler code today. It is avoiding three future composition systems and making receiver-aware autocomplete the organizing principle for all of them.

## Three proposed language layers

The conversation separated three kinds of program that should share one language and type system.

### 1. Expressions

Expressions calculate scalars and columns.

```text
let revenue = `Quantity` * `Unit Price`
let cost = `Quantity` * `Unit Cost`
let profit = revenue - cost

profit / revenue
```

They should be typed, deterministic, and side-effect-free. Local names should be immutable and scoped to their block.

### 2. Table expressions

Table expressions produce a table from another table or source.

```text
Orders
  .filter(`Status` == "Paid")
  .with_columns(
    revenue = `Quantity` * `Unit Price`,
    month = `Order Date`.dt.month_start()
  )
  .group_by(`Region`, `month`)
  .summarize(
    revenue = `revenue`.sum(),
    orders = table.len
  )
  .sort(`month`, `Region`)
```

Wrangle should be a visual editor for this same structure, not a separate transformation system.

### 3. Workbook declarations

Workbook declarations determine which visible objects exist.

```text
parameter growth_rate: percentage = 0.08

source Orders = parquet("orders.parquet")

table MonthlyRevenue:
  ...

value ForecastTotal:
  Forecast.`Revenue`.sum()

plot RevenueChart:
  line(MonthlyRevenue, x = `Month`, y = `Revenue`)
```

This is how the workbook could eventually control its own construction without acquiring a VBA-like macro model. The program declares the desired workbook and the reactive engine maintains it.

## Blocks as the unit of authorship

Blocks may resolve an important tension between one huge formula and dozens of tiny UI elements.

A block has one public result and may contain private intermediate bindings:

```text
table MonthlyRevenue:
  let posted =
    Orders.filter(`Status` == "Paid")

  let calculated =
    posted.with_columns(
      gross = `Quantity` * `Unit Price`,
      net = gross - `Discount`,
      month = `Date`.dt.month_start()
    )

  calculated
    .group_by(`month`, `Region`)
    .summarize(
      revenue = `net`.sum(),
      orders = table.len
    )
    .sort(`month`, `Region`)
```

`posted` and `calculated` improve the implementation without creating permanent workbook tables. A user could later promote one to a visible table if it becomes meaningful outside the calculation.

The same idea applies to a calculated column:

```text
column `Interest Expense`:
  let opening_debt = `Debt`.shift(1)
  let average_debt = (opening_debt + `Debt`) / 2
  let period_rate = `Interest Rate` / periods_per_year

  average_debt * period_rate
```

Only `Interest Expense` becomes a table column. The helper values remain local and do not pollute the workbook schema.

### Multiline arrived as layout, not grammar

*(from a follow-up conversation; implemented)*

The first concrete step toward long-form formulas turned out to need no
syntax at all. The lexer already treats a newline as whitespace, so the only
question was what a newline *means* in a surface made of lines — and the
answer is indentation:

> A physical line that begins with whitespace and says something continues
> the line above it. Alt+Return writes exactly this shape: a newline and two
> spaces.

In a scratchpad, newline at the margin still means "next calculation" — the
key that makes a line was not borrowed. But a formula can now breathe:

```text
revenue = `Amount`
  .filter(`Region` == "East")
  .sum()
```

is one line, one name, one answer in the gutter. The rule lives once in the
core and is mirrored by the editors; the gutter spans a line's physical
rows, and alternating bands under the text make where one calculation ends
and the next begins legible without reading the indentation.

This de-risks the block-grammar question considerably: the leading-dot
chain style every example in this document uses is now something the
product itself writes, and whatever `let` or declaration syntax comes later
inherits a working multiline convention instead of having to invent one.

The small set discussed was:

```text
parameter
source
column
table
value
plot
function
pipeline
solve
```

This is exploratory vocabulary, not an accepted grammar.

## A Pluto-like long editor

The long editor may be better conceived as a dense reactive notebook than as one enormous source file.

Each declaration block could show its result immediately beneath or beside it:

- a scalar value;
- a compact table preview;
- a plot;
- a local error;
- dependency or lineage information when requested.

There should be no Run button. Editing a valid declaration updates its dependants. Textual position should organize the document for readers but should not determine execution order.

The workbook could then have several projections of one program:

- **Canvas:** spatial presentation;
- **Grid:** dense table interaction;
- **Wrangle:** visual pipeline editing;
- **Notebook:** declaration blocks with inline results;
- **Source:** uninterrupted text for advanced editing.

The notebook and source views should not be mandatory. They are later surfaces over the same model that beginners already manipulate through the grid and menus.

## One canonical structure, not synchronized copies

The text, Wrangle steps, and grid gestures must not be stored as independent programs. That would create drafts that disagree and transformations that cannot round-trip.

The desired direction is:

```text
text edits ---------+
                    |
Wrangle gestures ---+--> lossless syntax tree --> typed dependency graph --> Polars plan
                    |
grid gestures ------+
```

The lossless tree would need to preserve:

- comments;
- formatting choices worth retaining;
- source spans and cursor locations;
- stable identities for declarations and important operation nodes;
- references to semantic table and column identities;
- enough structure to regenerate both readable source and Wrangle rows.

This is one of the hardest design areas. It should be addressed before building a substantial bidirectional text editor. Treating text as a blob and trying to recover structural identity later would make renames, collaboration, undo, and comment preservation much harder.

## Blocks as transaction boundaries

A block could also provide a natural editing and error boundary:

1. the user edits a textual expression or visual node;
2. FrameWork parses and type-checks the draft block;
3. if it is valid, the block's graph is replaced atomically;
4. if it is invalid, the error stays local and the last valid result remains available;
5. unrelated blocks continue to calculate.

This would prevent a partially typed six-step transformation from half-rebuilding a workbook.

The exact stale-result behavior needs careful product design. A retained result must never look current without a visible indication that its defining draft is invalid.

## A small canonical table vocabulary

Julia, dplyr, LINQ, Pandas, and Polars often provide several names for similar ideas. FrameWork should use those names as search aliases rather than save multiple equivalent languages.

The tentative canonical verbs were:

```text
.select(...)
.rename(...)
.filter(...)
.with_columns(...)
.group_by(...)
.summarize(...)
.sort(...)
.distinct(...)
.join(...)
.stack(...)
.limit(...)
```

Possible aliases used for discovery include:

- `mutate` and `transform` -> `with_columns`;
- `aggregate` and `combine` -> `summarize`;
- `arrange` and `order_by` -> `sort`;
- `where` and `subset` -> `filter`.

Excel names should work similarly. Typing `SUMIF` can surface the composable `.filter(...).sum()` form without adding a second canonical implementation.

## Reusable pipelines without macros

A future reusable pipeline could represent a transformation waiting for a table:

```text
pipeline PostedOrders(minimum: number):
  .filter(
    `Status` == "Paid",
    `Amount` >= minimum
  )
  .with_columns(
    month = `Date`.dt.month_start()
  )
```

It could then be applied explicitly:

```text
table EnterpriseOrders:
  Orders
    .pipe(PostedOrders, minimum = 10_000)
    .group_by(`month`)
    .summarize(revenue = `Amount`.sum())
```

This could eliminate copied Wrangle chains while remaining typed, inspectable, and deterministic. The syntax and whether `.pipe` is necessary remain open questions.

## Self-controlling without VBA

The desired form of control is declarative and reactive:

```text
parameter scenario: category = "Base"

table Forecast:
  Actuals.forecast(
    months = 24,
    growth = Assumptions[scenario].growth
  )

plot ForecastChart:
  line(Forecast, x = `Month`, y = `Revenue`)
```

Changing `scenario` changes the forecast and plot because they depend on it. No code selects sheets, clicks Refresh, loops over cells, or manipulates UI state.

General imperative facilities should be resisted in the core language:

```text
for every cell: set its value
click("Refresh")
select_sheet("Results")
show_dialog(...)
eval(user_code)
```

Those features recreate hidden state, order-sensitive execution, security problems, and UI-dependent programs.

Common macro jobs should instead map to explicit concepts:

- repetition -> sequences and table transformations;
- running calculations -> shifts, cumulative expressions, and windows;
- circular finance -> an explicit `solve` or `iterate` boundary;
- conditional calculation -> typed conditional expressions;
- reuse -> functions and pipelines;
- import and refresh -> source declarations;
- goal seeking and optimization -> explicit solver declarations;
- scheduled exports or external actions -> a separate permissioned automation layer.

## Progressive disclosure without separate modes

The conversation did not favor a reduced “beginner language” and a separate advanced language. That would teach concepts that stop transferring as the user grows.

Instead, the same operation should be progressively revealed:

### Direct manipulation

The user right-clicks a string column and chooses **Convert to date**.

### Wrangle

The user later sees a row such as:

```text
Convert Date to date
```

### Formula or block

An advanced user may inspect:

```text
Orders.with_columns(
  date = `Date`.str.to_date()
)
```

There is one underlying operation and three levels of presentation.

This progression should not require a “beginner mode” that creates weaker workbooks. Beginners should create the same durable structures as advanced users.

## What beginners should not have to confront

Ordinary work should not require:

- type annotations;
- imports or package management;
- lambdas for routine transformations;
- execution buttons;
- mutable variables;
- table-declaration boilerplate;
- programming terminology in common errors;
- understanding the dependency graph before benefiting from it.

Types can normally be inferred, references can be inserted by clicking, and contextual autocomplete can teach the next valid operation.

## What advanced users should be able to reach

Without leaving the workbook, the model should eventually be able to expose:

- multiline formulas;
- immutable local names;
- reusable pipelines and functions;
- explicit table declarations;
- typed parameters;
- window and sequential calculations;
- scenario and sensitivity models;
- goal seeking and optimization;
- explicit iterative financial models;
- notebook and source representations;
- extension nodes for work that genuinely belongs in another language.

The presence of these capabilities should not add ceremony to a beginner's first table.

## Product and language admission tests

Before adding a construct, ask:

1. Is this a real workbook concept or an attempt to recreate a general-purpose language?
2. Can the common case be created through direct manipulation?
3. Can autocomplete introduce it at the moment it becomes relevant?
4. Can Wrangle or another visual surface explain it?
5. Can an advanced user inspect and edit a faithful textual representation?
6. Does it compose with the existing dependency graph?
7. Is its type and shape behavior predictable?
8. Is ordering explicit when the result depends on row order?
9. Is it deterministic and safe to execute when a document opens?
10. Can a beginner completely ignore it until it is useful?

A construct that fails several of these tests may belong in a permissioned extension rather than the workbook language.

## The level of programmability

*(from a follow-up conversation)*

The fence question — how much language is enough — has a specific answer: FrameWork's language should be a **total derivation language**, not a scripting language. Every program is a pure, terminating derivation from sources and parameters to tables, values, and plots. Every formula provably halts; every workbook opens safely; every operation can be scheduled by the dependency graph and explained by a surface.

SQL is the existence proof that this level is sufficient. Non-recursive SQL with window functions — roughly relational algebra plus windows — has run the world's reporting for decades, and in all that time nobody has wanted to write an application in it. That is not a failure of SQL; it is the property FrameWork wants: a language with no IO, no runtime of its own, and no functions-as-values is useless outside its host **by construction**. The skill transfers — filter, group, and join are universal literacy — but the artifact does not. Python, Julia, R, and Rust stay better at scripting on purpose; FrameWork never competes there.

The compact description of the target: **SQL-class power, Excel-class approachability, Pluto-class reactivity.** Each pair already exists in some product; the triple does not, because the triple requires the projection architecture described above, and every prior attempt fumbled one leg. Power Query picked nearly this level and proved it works — the step UI is loved — but M is a full functional language nobody writes voluntarily, projected one way only. DAX has the power but made context implicit, which is its cliff. Airtable and Notion formulas picked too little language and expert users hit the ceiling in a month. Excel itself cannot move without breaking forty years of documents.

One rule selects the level and settles most future scope arguments:

> **Nothing enters the language that the UI cannot write.**

Every construct must be producible by some gesture, and the text view is what a user sees when they look behind the gesture. Filter, group, windows, `let` blocks, parameters, and a bounded `solve` all pass — a menu or dialog can emit each one. Unbounded loops, recursion, and first-class functions fail — no gesture produces them — so they stay out. The rule also fixes the learning loop: do it by hand, read what it wrote, type it next time. For that loop to work, the pretty-printer must be canonical — exactly one way to write each operation, no formatting freedom — so reading the projection is the tutorial.

### The fence, precisely

The line is **bounded versus unbounded**, not "loops versus no loops."

Out, permanently: iteration whose trip count depends on data values — `while`, user-defined recursion — along with first-class functions, IO inside formulas, `eval`, mutable state, and UI automation. When work genuinely needs those, the answer is an explicit permissioned extension node with declared inputs and outputs, not a bigger formula language. The extension node is what makes "no" sustainable.

In: conditional *expressions* — per-row `if` with no filter attached, which columns already have — and **comprehension over finite workbook sets**: the columns of a table, a written list, an integer range. "For each numeric column, round it; otherwise leave it alone" is not a loop in the banned sense. The set of columns is known when the plan is built, so the construct is a static expansion into per-column expressions, not iteration at runtime. dplyr spells it `across(where(is.numeric), ...)`; Polars spells it with selectors. FrameWork already has the beginning of the notation in the unpivot column list — `` `Jan`, `Feb`, starts_with("Q"), except(`Region`) `` — and a type selector such as `numeric()` is the natural extension. Columns do not need to become runtime array values for this; the comprehension resolves at plan time.

This construct removes danger rather than adding it. Excel's answer to "apply this to all forty quarter columns" is copy-paste, its most error-prone gesture. SQL's answer is string templating bolted on top — dbt's Jinja loops exist almost entirely to iterate over columns. Schema comprehension belongs in the language precisely so that neither workaround appears.

The question it raises is expansion time. The unpivot list resolves its selectors when the step is saved — a step keeps meaning what it meant — while the appeal of "all numeric columns" is partly that it could adapt when the schema changes. The precedent says bake at save and re-run the selector on edit; whether any construct earns live expansion needs a deliberate decision, not a default.

### Tables are a finite set too

The same argument covers applying one transformation over several tables. SQL makes that convoluted — a stored procedure assembling dynamic SQL, or dbt's Jinja — not because iteration over tables is dangerous but because a database is an *open world* with no plan-level enumeration of its tables; looping over them requires a metalanguage that manipulates queries as strings. A workbook is a *closed world*: the set of tables in it, exactly like the set of columns in a table, is finite and known when the plan is built. Comprehension over a closed world is a static expansion; over an open world it is metaprogramming. FrameWork has the closed world, so it gets the construct cheaply.

Table comprehension decomposes into constructs already proposed rather than adding a new primitive: a named pipeline defines the transformation once, and comprehension applies it over a finite set — of tables now as well as columns. At three literal tables, three one-line applications are arguably clearer than a loop; the comprehension form earns its keep when the set is selector-driven (`tables(starts_with("Orders_"))`) or large.

It also resolves the pipeline typing question from the reuse section. When every application site is concrete — and in a closed world they all are — a pipeline needs no polymorphic type system: expand first, then check each application against that table's actual schema, and report errors per application ("Clean fails on Orders2024: no column `Status`"). Monomorphization, the Julia move, rather than generalization, the ML move.

Two boundaries keep it honest:

- The comprehended set must be schema- or workbook-bounded, never data-bounded. "One table per distinct Region value" has a trip count that depends on data — the pivot problem at workbook scale — and if it is ever admitted it gets the pivot treatment: outputs resolved and baked when the step is saved, never live.
- A comprehension that declares several tables must keep stable identities for its outputs across re-expansion, keyed by source table rather than by position, or every downstream reference breaks when the set is edited. This is the same identity problem pivot outputs already answer.

### One ergonomic law: context is always explicit

At this level the historical killer is not missing power but implicit context — DAX measures silently re-evaluating under ambient filter context is the cliff that makes an otherwise approachable tool hard. In FrameWork, aggregation happens because the user wrote `group_by` and `summarize` in a visible chain, never because a value re-aggregates under filters it cannot see. If measures are ever demanded, the answer is a visible chain that produces them, not a second evaluation mode.

## Two languages with a deliberate boundary

*(from a follow-up conversation)*

FrameWork needs two different kinds of programmability, and they should not be forced into one language.

### The unnamed workbook language

The data-manipulation language discussed throughout this document remains unnamed. It exists **inside** a workbook and describes values, columns, tables, transformations, parameters, plots, and bounded solves.

Its contract is deliberately constrained:

- declarative and reactive;
- pure and terminating;
- deterministic unless volatility is explicitly represented;
- safe to evaluate when a document opens;
- completely understood by the type checker and dependency graph;
- constructible through FrameWork's UI;
- projectable as formulas, Wrangle steps, blocks, and eventually long-form text;
- unable to perform arbitrary IO, launch processes, access secrets, or automate the interface.

This is the language a workbook is **made of**. It is not the language used to operate FrameWork from outside.

### Python is the helper and automation language

Python is the explicit external language for automating work **with** FrameWork:

```python
from framework import Workbook

with Workbook.open("month-end.fw") as workbook:
    workbook.replace_source("Ledger", "ledger-2026-08.parquet")
    workbook.refresh()
    workbook.export_table("Trial Balance", "trial-balance.xlsx")
    workbook.save()
```

An explicitly run Python program may use ordinary programming facilities that do not belong in an automatically evaluated workbook:

```python
for company in companies:
    download_ledger(company)
    update_workbook(company)
    export_reporting_package(company)
    email_controller(company)
```

Python already has loops, filesystems, HTTP clients, scheduling, secrets, email libraries, package management, and mature debugging. Recreating those facilities inside the workbook language would add risk while producing a worse scripting language.

The security boundary is execution consent:

- opening an `.fw` document evaluates only the safe internal language;
- running `python close.py` explicitly authorizes arbitrary Python execution;
- an `.fw` document may call only declared safe workbook functions, never smuggle Python source to be evaluated on open.

A companion script may live beside a workbook without becoming part of it:

```text
monthly-close/
├── close.fw
├── close.py
├── sources/
└── exports/
```

### Export the product's operations, not its file format

The Python library and CLI must not edit `.fw` JSON directly or acquire a second mutation model. They should expose the same canonical operations and queries used by the desktop and MCP:

```text
Python SDK ─┐
CLI ────────┼─> shared query and operation service
MCP ────────┘        │
                     ├─ validation
                     ├─ history
                     ├─ persistence
                     ├─ collaboration events
                     └─ artifact management
```

The existing MCP operation path proves the important part of this architecture: public mutations derive from the canonical Rust `Operation` enum and travel through normal validation, history, persistence, and collaboration. A CLI and Python SDK should reuse that path rather than hand-maintain parallel request types.

The CLI can provide ordinary shell entry points:

```text
framework inspect month-end.fw
framework tables month-end.fw
framework refresh month-end.fw
framework export month-end.fw "Trial Balance" --output trial-balance.xlsx
framework apply month-end.fw operation.json
framework validate month-end.fw
```

The Python SDK can have two layers:

- generated low-level parity with canonical Rust queries and operations;
- handwritten high-level conveniences that lower into those same operations.

```python
orders = workbook.table("Orders")
orders.add_calculated_column(
    "Revenue",
    "`Quantity` * `Unit Price`",
)

monthly = orders.derive("Monthly Revenue")
monthly.group_by("Month").summarize(
    revenue="`Revenue`.sum()",
)
```

The fluent Python API is convenience, not another execution engine and not the canonical representation of the workbook.

### Reliability requirements for external automation

A thin Python wrapper is straightforward. A trustworthy automation product also needs:

- transactions or atomic operation batches;
- full preflight before persistence;
- expected-revision checks for conflicting writers;
- structured formula and validation errors;
- explicit artifact preparation and refresh behavior;
- dry-run support;
- useful undo boundaries;
- stable generated schemas rather than handwritten duplication;
- a persistent session for multi-step work rather than one new process per method.

Those concerns belong to the shared automation service and benefit MCP, CLI, and Python together.

### The resulting division of responsibility

| Inside `.fw` | Outside `.fw` |
| --- | --- |
| Unnamed FrameWork data language | Python, CLI, and MCP |
| Declarative | Imperative |
| Reactive | Explicitly executed |
| Pure and bounded | General-purpose |
| Safe on document open | Trusted by the person running it |
| Calculates workbook contents | Automates workflow around workbooks |
| Tables, values, transformations, plots | Files, networks, loops, scheduling, exports |

This boundary makes saying “no” to general-purpose language features sustainable. Advanced users are not trapped; they have a real escape hatch in a language already better suited to automation. At the same time, shared workbooks remain portable, inspectable, reactive, and safe to open.

## Hard problems that remain open

### Grammar

- Should blocks use indentation, braces, `begin`/`end`, or another delimiter?
- Which functions render prefix rather than fluent in canonical form, and where exactly does the formatter normalize — on commit, on blur, on save?
- What are the exact shadowing rules when a workbook definition collides with a catalog name?
- How are named arguments, local names, and final block results distinguished?
- How closely should saved syntax follow current formula syntax?

### Shapes and evaluation contexts

- What are the exact scalar, column, list, table, grouped-table, and perhaps record types?
- Which scalar values broadcast across columns?
- When does an aggregate change a column into a scalar?
- When does a column selector feeding a transformation (`numeric()`, `starts_with("Q")`) expand — at save, following the unpivot list precedent, or live with the schema?
- How are row context and aggregate context explained without dplyr-style ambiguity?
- What does filtering a column mean compared with filtering a table?

### Names, scopes, and identity

- How do table names, column names, block names, local names, parameters, and functions resolve?
- Which references must be qualified?
- How does the syntax tree preserve stable node identity across arbitrary textual edits?
- How are duplicate display names represented and diagnosed?
- How are comments and formatting preserved after a visual edit?

### Reactive behavior

- What exactly happens while a block is syntactically invalid?
- How are cycles reported?
- What syntax and execution contract encloses an intentional iterative solve?
- Which functions are volatile or nondeterministic, and how are their results captured?

### Visual/textual round-tripping

- Which textual programs can Wrangle represent completely?
- Can local table bindings be shown as nested Wrangle sections without becoming visible tables?
- What happens when text uses a construct for which no focused visual editor exists?
- How do selections, cursor locations, folded sections, and collaborative edits survive projection changes?

### Workbook construction

- Which layout and formatting properties belong in declarations?
- Should source declarations be portable descriptions or machine-local connections?
- How do text-created tables and plots receive initial canvas placement?
- How much document presentation should be declarative before it becomes UI automation in disguise?

### Reuse and extensions

- What is the difference between a scalar `function` and a table `pipeline`?
- Are user-defined functions always inlined into the known AST?
- When a comprehension declares several tables, what are the outputs named, and how do their identities survive the set being edited?
- How are versions and compatibility managed after formulas are saved in real documents?
- What is the smallest honest boundary for Julia, Python, SQL, or WASM extensions?

## Proposed principles to carry forward

These are the strongest points of convergence from the conversation:

1. **The language is infrastructure, not the product.**
2. **The grid remains the front door.**
3. **Simple work must be at least as immediate as Excel.**
4. **Complexity is available, not ambient.**
5. **There is one semantic model, progressively revealed.**
6. **Dot notation expresses functional composition, not mutable objects.**
7. **The dependency graph, not textual order, determines evaluation.**
8. **Blocks may contain immutable local names and have one public result.**
9. **Wrangle and text are projections of the same structured program.**
10. **Workbook declarations replace most macro use cases.**
11. **Ordering and iteration must always be explicit.**
12. **Opening a workbook must not execute arbitrary user code.**
13. **Borrow established ideas and vocabulary before inventing new ones.**
14. **Search aliases can teach Excel, Julia, dplyr, LINQ, and Polars vocabulary without creating several canonical languages.**
15. **The core remains deliberately smaller than a general-purpose language.**
16. **Nothing enters the language that the UI cannot write.**
17. **Bounded comprehension is in; unbounded iteration is out.**
18. **Context is always explicit — no ambient filter or evaluation modes.**

## A possible north star

> FrameWork makes simple work simpler than Excel and complex work possible without leaving the workbook. Every operation can begin as a gesture, remain understandable as a visual transformation, and grow into composable text only when the user needs it.

An alternative technical description is:

> A typed, functional, reactive table language with fluent syntax, presented first as a spreadsheet and later as a notebook when the work demands it.

Neither sentence is a final product tagline. Together they capture the product promise and the underlying mechanism.

## Recommended next step

Do not implement `let`, workbook declarations, or a long-form editor directly from this note. First turn the open questions into a small language proposal with executable examples.

A useful first design exercise would specify one realistic workbook at three levels:

1. the beginner creates it entirely through grid and menu gestures;
2. the intermediate user edits its Wrangle representation;
3. the advanced user edits its block/notebook representation.

The exercise should prove that all three representations produce the same typed operation graph. It should include:

- an imported source;
- a string-to-date conversion;
- a calculated column;
- a filter and sort;
- a grouped summary;
- a parameter;
- a window calculation with declared ordering;
- a plot;
- a local binding that does not become a visible column;
- one deliberate error and its behavior;
- a rename performed once through each authoring surface.

Only after that example round-trips on paper should the grammar and lossless syntax-tree architecture be settled. This keeps the language grounded in the product's actual interaction model rather than allowing syntax design to become its own project.

## Visualization and BI languages

*(from a follow-up conversation)*

The discussion above mostly approached FrameWork through programming languages. Visualization and BI systems are an equally important comparison because many of them tried to make analytical programming accessible through direct manipulation.

Power BI, Tableau, Qlik, Looker, and visualization grammars have not simply failed. Several are commercially successful and technically powerful. The narrower failure is that they generally did not produce one coherent end-user programming model that remains understandable from data preparation through calculation, visualization, interaction, and maintenance.

The recurring problem is not insufficient expressive power. It is **too many invisible contexts and semantic seams**.

### Power BI: calculation location becomes part of the language

Power BI currently documents five places to create calculations:

| Kind | Language | Evaluation | Context | Storage |
| --- | --- | --- | --- | --- |
| Power Query custom column | M | Refresh | Row | Table/query |
| Calculated column | DAX | Refresh | Row | Model |
| Calculated table | DAX | Refresh | Model | Model |
| Measure | DAX | On demand | Filter | Model definition |
| Visual calculation | DAX | On demand | Visual | Visual |

The official comparison is in [Power BI's calculation-options documentation](https://learn.microsoft.com/en-us/power-bi/transform-model/desktop-calculations-options).

This means “calculate profit” begins with a decision about architecture, timing, storage, interactivity, and evaluation context. Similar expressions behave differently because they were authored in different places.

FrameWork should resist accumulating separate source, table, model, chart, display, and scratch formula systems. A value may have a different **shape** or **scope**, but it should not silently acquire different fundamental semantics because of the editor in which it was created.

### DAX: the important program is often outside the formula

DAX's distinctive power is filter context. A measure such as:

```text
Sales = SUM(Orders[Amount])
```

does not have one fixed result. Its result depends on fields placed in a visual, slicers, report filters, relationships, cross-filter direction, current groups, and context modifications made by other expressions.

`CALCULATE` can add filters, overwrite existing filters, remove filters, and transition row context into filter context. These rules are explicit in [Microsoft's `CALCULATE` documentation](https://learn.microsoft.com/en-us/dax/calculate-function-dax), while the distinction between row and filter context is introduced in [DAX basics](https://learn.microsoft.com/en-us/power-bi/transform-model/desktop-quickstart-learn-dax-basics).

This yields a characteristic debugging problem: the visible formula is short, but the difficult question is “which rows are `Orders` right now, and why?” Much of the program is environmental state rather than source near the calculation.

FrameWork should not conclude that all context is bad. Every expression has a scope, table, grain, and perhaps group. The lesson is that meaningful context should be:

- structurally represented;
- inspectable beside the expression;
- traceable through lineage;
- predictable from the expression's typed receiver;
- explicit when it changes grain, ordering, filtering, or aggregation.

### M: a promising table language confined to one phase

Power Query M is close to several FrameWork ideas. It is a functional, table-oriented language with lazy evaluation and immutable intermediate bindings:

```text
let
    Source = ...,
    Filtered = Table.SelectRows(Source, ...),
    AddedRevenue = Table.AddColumn(Filtered, ...)
in
    AddedRevenue
```

Its [`let` expression](https://learn.microsoft.com/en-us/powerquery-m/m-spec-let) strongly resembles the proposed FrameWork block with private intermediates, and the [M language overview](https://learn.microsoft.com/en-us/powerquery-m/) describes a broad table and data-mashup language.

The larger product problem is that M stops at a phase boundary. It prepares data before the model; DAX then owns model calculations; the visual layer adds another context. Most users manipulate a generated list of steps, while direct M editing acts more like an escape hatch than a fully bidirectional representation.

The lesson is not to reject M's design. It is to avoid confining a good table language to “preparation” and introducing a second language for the rest of the workbook.

### Tableau and VizQL: the gesture can be the language

Tableau's important move was that users usually do not type VizQL. Placing fields on Rows, Columns, Color, Size, Detail, and filters constructs a visual query. The [Polaris/VizQL paper](https://www.tableau.com/sites/default/files/2023-01/Tableau-CACM-Nov-2008-Polaris-Article-by-Stolte-Tang-Hanrahan.pdf) describes a formal visual specification connected to database queries.

This suggests an important FrameWork principle:

> A visual operation succeeds when the visual structure is a faithful editor for the underlying program, not a wizard that emits code the user can no longer manipulate visually.

Tableau also illustrates how complexity returns when calculation layers accumulate: calculated fields, table calculations, level-of-detail expressions, filter-order rules, and implicit aggregation do not all share the same scope or grain.

The gesture-as-query idea is worth adopting; the proliferation of contexts is not.

### Vega-Lite: a small declarative visual algebra

[Vega-Lite](https://vega.github.io/vega-lite/docs/) describes a chart through mappings from data fields to visual channels such as position, color, size, mark, facet, scale, and interaction. A compiler supplies axes, legends, scales, and other details.

Its JSON syntax is not an appropriate primary interface for spreadsheet users, but its architecture is valuable. FrameWork does not need hundreds of unrelated chart commands. It can use a small visual algebra:

```text
plot RevenueByRegion:
  let chart_data =
    Orders
      .group_by(`Region`)
      .summarize(revenue = `Amount`.sum())

  bar(
    data = chart_data,
    x = `Region`,
    y = `revenue`
  )
```

The same block could appear visually as:

```text
Data       Orders
Group      Region
Measure    Sum of Amount -> revenue
Mark       Bar
X          Region
Y          revenue
```

The local `chart_data` table need not become a permanent canvas table. It remains inspectable inside the plot block and can be promoted if it becomes meaningful elsewhere.

### Why visual analytical languages commonly hit a ceiling

Several failure patterns recur:

- **Hidden program state.** Results depend on selections, filters, relationships, visual placement, or execution history that cannot be understood from the calculation itself.
- **Multiple calculation universes.** Preparation, modeling, measures, visual calculations, and presentation each acquire separate rules. Users must learn where to write a calculation before they can write it.
- **One-way code generation.** The UI generates text or a query, but editing that representation exceeds what the UI can understand. Beginners and advanced users effectively work in different products.
- **Visual node explosion.** Boxes and arrows can be pleasant for five operations and unusable for fifty. Chrome and wiring consume more space than the calculation. Dense Wrangle rows and expandable blocks are a better fit than a general node graph.
- **Implicit grain.** The user cannot easily tell whether an expression operates over source rows, transformed rows, groups, model relationships, aggregated visual cells, or displayed marks.
- **Premature semantic modeling.** Some BI systems require relationships, dimensions, measures, aggregation rules, and governance before the user can answer an elementary question. This serves centralized reporting but destroys spreadsheet immediacy.
- **Escape hatches become the real authoring environment.** If routine work repeatedly requires DAX, M, SQL, custom visuals, or scripts, the friendly surface is not actually a complete authoring system.
- **Debugging is secondary to initial creation.** Many tools optimize for quickly producing a chart rather than explaining where a number came from, which rows were included, at what grain it was calculated, which filter removed something, or why a total differs from the visible rows.

### A tentative FrameWork visualization model

A plot can keep four concepts structurally distinct without giving them separate languages:

```text
data -> transformation -> visual encoding -> interaction
```

#### Data and transformation

Every plot consumes an identifiable table expression. A plot may use an existing table or define local transformations inside its block.

```text
let chart_data =
  Orders
    .filter(`Status` == "Paid")
    .group_by(`Month`, `Region`)
    .summarize(revenue = `Amount`.sum())
```

#### Visual encoding

The plot maps known columns to channels:

```text
line(
  data = chart_data,
  x = `Month`,
  y = `revenue`,
  color = `Region`
)
```

Aggregation and grain should not be hidden merely because a field was dropped on an axis. If direct manipulation introduces an aggregation, the generated plot block should expose it as a real transformation.

#### Interaction

Hover, pan, zoom, and temporary selection can remain presentation state because they do not change workbook calculations.

If a selection affects another calculation, it should cross into the semantic graph as an explicit reactive value. Illustratively:

```text
selection selected_regions =
  RevenueChart.selection(`Region`)

table SelectedOrders:
  Orders.filter(
    `Region`.is_in(selected_regions)
  )
```

The syntax is unresolved. The principle is that cross-filtering should not silently redefine every measure in the document. A semantic interaction must have a visible dependency edge and a comprehensible home.

### Visualization-specific questions

- Should plots be pure consumers of tables, or may every plot contain local table transformations?
- When a user drags a numeric field into a plot, where is the inferred aggregation represented?
- Can every visual aggregation be inspected, edited, and promoted as a table expression?
- Which plot properties belong to the semantic AST and which are presentation only?
- Which interactions are ephemeral view state, which are saved presentation state, and which become reactive workbook values?
- How do selections avoid introducing cycles when downstream tables feed the selecting plot?
- Should filtering a plot ever alter its source table, or should it create or edit a local plot-data branch?
- Can the plot editor and plot block round-trip without degrading to a property inspector with hundreds of fields?
- How are chart defaults kept strong enough for beginners while remaining explicit when they affect meaning?
- How does lineage explain a mark on the screen all the way back to source rows?

### Visualization principles to carry forward

1. **Do not create a visualization calculation language separate from the formula language.**
2. **A plot consumes an inspectable table expression at a known grain.**
3. **Dragging a field is a programming gesture and should create a durable, readable node.**
4. **Aggregation, grouping, filtering, ordering, and windowing are semantic operations, not formatting.**
5. **Visual encoding and data transformation are distinct parts of one block.**
6. **Local chart data is allowed without forcing every intermediate onto the canvas.**
7. **A meaningful interaction becomes an explicit dependency; incidental interaction remains presentation state.**
8. **The visual editor and textual block must remain bidirectional.**
9. **Debuggability and grain visibility matter more than minimizing the initial number of clicks.**
10. **Avoid invisible context as program state.**

## Why the graveyard is not a verdict

*(from a follow-up conversation, prompted by the worry that smarter teams than ours have failed to make analytical tools people love)*

The worry is real and worth answering precisely, because read carefully the graveyard says something more specific than "this is impossible."

### BI tools never attempted this product

Power BI and Tableau did not fail commercially — one is the most successful BI product ever shipped, the other sold for $15.7B. What they failed at is being loved, and at displacing Excel for the work Excel actually does. The tell is the oldest joke in the industry: the most-used feature of every BI tool is Export to Excel.

But that means they lost the modeling surface without ever attacking it. BI tools sit at the end of the pipeline — they consume a finished dataset and publish conclusions to viewers. Excel lives in the middle: it is where a person cleans, models, tests assumptions, and thinks. There is no failed attempt at FrameWork among the BI tools; there is a category that chose the reporting job and won it.

### Unloved is structural, not accidental

Four deliberate choices, each traceable to who pays for BI:

- **Bought by organizations, chosen by no one.** The roadmap accretes procurement features — governance, certification, row-level security — rather than thinking features.
- **Authors split from viewers.** Most people who touch the product can only look at it, and nobody loves glass. Excel is loved because every recipient of a file is one keystroke from being an author.
- **First-chart speed over explanation.** Optimizing initial creation over "where did this number come from" (catalogued in the previous section).
- **A wall where Excel has a ramp.** Power BI's first day is data models, relationships, and measure-versus-column; the beginner's curve is inverted.

These were incentive failures, not intelligence failures. Microsoft cannot cannibalize Excel, and Power BI's semantic seams are literally organizational seams — Power Query, the SSAS tabular engine, and the visuals layer were separate products stapled together. Nobody smart was ever paid to build the personal thinking surface. FrameWork's structural choices — personal file, one surface for author and consumer, lineage first-class, gesture-first ramp — invert each of the four, and can only be made from outside those incentive structures.

### The sharper warning: gravity, not design

The products that did attack Excel directly, with arguably better models, form the real graveyard: Javelin (product-of-the-year awards over Excel and Windows in 1985), Lotus Improv (genuinely beloved, dead by 1996), Quantrix (alive, niche), and the modern cohort — Causal (acquired into an FP&A suite), Airtable (pivoted away from analysis), Sigma (growing, but warehouse-tethered). Almost none died of bad design. They died of **gravity**: the .xlsx file network, muscle memory, and the cliff of "rebuild everything to switch."

That relocates the risk. The recurring unsolved problem is not the design problem this document works on; it is distribution against Excel's gravity — a different problem, which the design can at least refuse to worsen:

- file-based and personal, adopted one user at a time (already true);
- useful single-player on day one, with no server, model, or governance prerequisite;
- the first five minutes happen on the user's existing data — paste and import from Excel are sacred paths;
- sell thinking, not governance.

### What is different now

Two conditions the 1991 and 2015 attempts lacked:

1. **Polars-class local engines.** A laptop now covers the analytical scale that used to force people out of personal tools and into org-owned BI. The workloads that left Excel can come back to a personal tool.
2. **LLM legibility.** A canonical semantic graph with a faithful text projection is precisely the representation an AI assistant can read and write safely. None of the graveyard products had one; none of the incumbents can retrofit one across their calculation seams. The training-data gap (models know Excel and SQL, not `.fw`) is mitigated by the alias-and-borrowed-vocabulary principle already adopted.

Excel's own trajectory — LET, LAMBDA, Python-in-Excel — is the incumbent conceding demand for exactly this while demonstrating that it can only bolt it on.

### The posture

The base rate for this category is bad, and confidence is not the correct response — clarity is. The graveyard is a checklist, not a verdict: every catalogued failure has a named cause, and this document's job is to ensure that if FrameWork fails, it fails for a new reason rather than a documented one.
