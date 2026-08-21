# Native Polars formulas

FrameWork formulas are native Polars expressions with one ergonomic substitution: exact backtick references replace `pl.col(...)`. The calculated-column name supplies the final alias.

```text
`Weight` / (`Height` ** 2)
sum_horizontal([`Q1`, `Q2`, `Q3`]) * `Multiplier`
`Birthdate`.dt.year()
`Amount`.rolling_mean(window_size=30, min_periods=5)
`Amount`.sum().over(`Category`)
`Amount`.filter(`Region` == "West").sum()
`Date text`.str.to_date().dt.month()
when(`Amount` > 100).then("High").otherwise("Low")
when(`Stat`).then("Stat").when(`Weekend`).then("Reg Holiday").otherwise("Work")
`Amount`.normalize()
`Amount`.normalize(center=0)
```

The Rust parser resolves backtick references to stable object and column IDs, then compiles the syntax tree to Polars 0.55 `Expr` objects. Formula text is not Python and is never passed to `eval`. Polars owns expression typing, null behavior, broadcasting, aggregation, window semantics, and execution errors.

## Syntax

- Exact references: `` `Column` ``, `` `Canvas Value` ``, and `` `Table`.`Column` ``.
- Literals: numbers, strings, `True`, `False`, `None`, and `null`.
- Operators: `+`, `-`, `*`, `/`, `//`, `%`, `**`, comparisons, `&`, `|`, and `~`.
- Structure: normal precedence, parentheses, lists, positional arguments, keyword arguments, and method chains.
- The calculated-column name is the alias. Calling `.alias(...)` inside a formula is rejected.

## Exposed expression surface

The catalog returned by the core and MCP is also used for autocomplete.

| Family | Current functions and methods |
| --- | --- |
| Horizontal | `sum_horizontal`, `mean_horizontal`, `min_horizontal`, `max_horizontal` |
| Generators / row order | `sequence(stop)`, `sequence(start, stop, step)`, `table.len()`; `recur(first, next, restart_by=[columns])` with `previous()` inside `next` |
| Conditional/null | `when().then()` — chained as many times as you like — `.otherwise()`, `coalesce`, `.is_null`, `.is_not_null`, `.fill_null`, `.filter(predicate)` → `.sum()` / `.mean()` / `.count()` |
| Numeric | `.abs`, `.sign`, `.round`, `.round_sig_figs`, `.truncate`, `.floor`, `.ceil`, `.sqrt`, `.cbrt`, `.pow`, `.exp`, `.log`, `.log1p`, `.normalize`, `.clip`, `.clip_min`, `.clip_max`, `.floor_div` |
| Trigonometry | `.sin`, `.cos`, `.tan`, `.cot`, `.arcsin`, `.arccos`, `.arctan`, `.arctan2`, `.sinh`, `.cosh`, `.tanh`, `.arcsinh`, `.arccosh`, `.arctanh`, `.degrees`, `.radians` |
| Aggregation/window | `.sum`, `.mean`, `.quantile`, `.min`, `.max`, `.count`, `.len`, `.null_count`, `.shift`, `.over` |
| Rolling | `.rolling_mean`, `.rolling_sum`, `.rolling_min`, `.rolling_max` |
| Dates | `date`, `.dt.year`, `.dt.iso_year`, `.dt.quarter`, `.dt.month`, `.dt.week`, `.dt.weekday`, `.dt.ordinal_day`, `.dt.is_leap_year`, `.dt.days_in_month`, `.dt.date`, `.dt.month_start`, `.dt.month_end`, `.dt.offset_by` |
| Strings | `.str.to_uppercase`, `.str.to_lowercase`, `.str.to_date()` (strict `YYYY-MM-DD`), `.str.contains` |

`sequence` is an advanced Scratchwork formula. Its stop is excluded:
`sequence(1, 8, 2)` produces `1, 3, 5, 7`, while
`sequence(2026-01-01, 2027-01-01, 1mo)` produces a monthly date spine. It is
a typed list rather than a row number, so it can be inspected, copied, or
folded with `.sum()` and friends without being silently matched to table rows.
Inside a sorted Wrangle chain, `sequence(1, table.len() + 1)` is the explicit
exception: it fills the current table with `1…N` in the order declared before
that calculated-column step. FrameWork refuses it when no sort says what row
position means.

`recur` is the explicitly sequential exception to the otherwise vectorized
column engine. Inside a sorted Wrangle chain,
``recur(`Opening`, previous() + `Change`, restart_by=[`Account`])`` evaluates
top to bottom and carries a separate previous result for each account. It is
normally authored through the visual *Calculate down rows* step rather than
typed as a wrapper. `previous()` is rejected outside that context, and the
first-row expression cannot use it.

Unsupported methods fail visibly instead of falling back to another evaluator. The catalog is a discoverability surface, not a separate formula language: names, arguments, and behavior are intended to track Polars.

## Generated expression surface

The table above is the hand-written core. Beyond it, [formula-function-catalog.generated.md](formula-function-catalog.generated.md) lists a much wider, code-generated surface covering most of the remaining Polars 0.55.2 `Expr` methods and namespaces (root functions, and the `str`/`dt`/`list`/`arr`/`struct`/`cat` namespaces). It is produced by `tools/generate_expr_bindings.py` from the vendored `polars-plan` source and compiled by `crates/framework-core/src/generated_expr_bindings.rs`; both files carry regeneration instructions in their headers. Methods that take closures/UDFs, IO/serialization/meta/plugin methods, and `alias` are excluded by rule; anything else the generator can't bind with certainty (an options-struct or enum argument, for example) is left out and recorded with a reason in `tools/expr_bindings_spec.json` rather than silently misbound.

## Safety boundary

FrameWork constructs typed Polars expressions from a parsed AST. It does not expose imports, attribute access outside known expression namespaces, filesystem or network I/O, Python callbacks, UDFs, plugins, NumPy execution, or arbitrary code evaluation. Expensive operations can later receive cost limits at the compiler/executor boundary without changing saved formula syntax.

Whole-table and shape-changing operations—filtering table rows, sorting, joins, group-by output, pivots, and reshaping—belong to derived-table `LazyFrame` plans rather than calculated columns. `.filter(predicate)` is narrower: it filters one expression so an aggregate can answer a conditional total, average, count, minimum, or maximum without changing the table itself. Nested list/array/struct dtypes and additional Polars namespaces can be added directly to the compiler as FrameWork gains display and editing support for those result types.
