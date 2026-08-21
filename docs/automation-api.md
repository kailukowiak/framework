# AI-native automation API

FrameWork exposes its semantic document model through a local Model Context Protocol (MCP) server. An AI client can work with named values, tables, rows, columns, formulas, and summaries directly. It does not need to inspect pixels, click the desktop UI, or browse a web page.

## Design contract

- `framework-core` remains canonical. The server calls the same typed operations, parser, dependency checks, evaluator, undo/redo history, and JSON persistence as the desktop.
- Reads return structured JSON with stable IDs and the current document revision.
- Names are convenient selectors; stable IDs are the unambiguous selectors clients should retain.
- Formula references canonically use exact backtick names, such as `` `Quantity` ``, `` `Safety Factor` ``, or `` `Imported data`.`Amount` ``. Backticks replace `pl.col(...)`; unquoted data names are rejected.
- `inspect_document` returns the native Polars expression catalog, including stable IDs, names, aliases, signatures, categories, return types, and null behavior. Formulas support normal operators, lists, keyword arguments, method chains, and namespaces. Computed cells return a tagged `typedValue` (`null`, `number`, `string`, `boolean`, or `date`). The current expression surface is documented in [formula-function-catalog.md](formula-function-catalog.md).
- Rows can be selected by stable ID or a 1-based number.
- Mutations accept `expectedRevision`. When supplied, a stale write is rejected with a revision conflict.
- Revisions increase for writes, undo, and redo. This prevents a document from returning to an old revision number after its contents change.
- Tools are annotated read-only or mutating for clients that use tool approval policies.

## Available tools

| Tool | Kind | Purpose |
| --- | --- | --- |
| `inspect_document` | read | Discover the document, revision, objects, values, tables, columns, and stable IDs. |
| `get_table` | read | Read rows, cells, formulas, computed values, errors, overrides, and summaries. |
| `complete_formula` | read | Get type-aware column, root-function, and method suggestions at a cursor position. |
| `describe_operations` | read | Return the complete generated `Operation` contract, including every nested input type. |
| `apply_operation` | write | Apply any public desktop operation using its canonical serialized form. |
| `create_block` | write | Create an empty formula block, optionally at supplied canvas coordinates. |
| `set_block_source` | write | Replace a formula block's complete source text; named lines can be referenced from other formulas. |
| `create_table` | write | Create a table from a string grid with a header row. |
| `set_value` | write | Change an existing legacy scalar value by name or ID. New scalar work belongs in formula blocks. |
| `delete_object` | write | Delete an unreferenced scalar, table, or text object. |
| `add_row` | write | Append a literal row, matching supplied values by column name or stable ID. |
| `delete_row` | write | Delete a row by stable ID or 1-based row number. |
| `set_cell` | write | Change a literal cell by table, row, and column. |
| `add_literal_column` | write | Insert a non-calculated column, optionally after a named or stable-ID column. |
| `delete_column` | write | Delete an unreferenced column; dependent formulas block the change. |
| `set_column_type` | write | Change a column's declared text, number, currency, percentage, boolean, or date type. |
| `set_column_categories` | write | Make a column categorical and set its ordered allowed values. |
| `add_calculated_column` | write | Append a calculated column to the frame's Wrangle chain using semantic references. |
| `set_cell_override` | write | Add, replace, or remove one cell's formula exception. |
| `add_summary` | write | Add sum, mean/average, or count to a column. |
| `undo` / `redo` | write | Navigate accepted operation history. |

The named tools are a focused, human-friendly subset. `describe_operations` and
`apply_operation` provide automatic coverage of the complete public mutation enum,
including renaming, existing-formula edits, imports, joins, pipelines, display state,
formatting, plots, layout, tabs, lists, containers, materialization, and packaging. The
catalog is generated from the same Rust enum as the desktop's TypeScript binding, so a new
operation appears after recompilation without another MCP implementation. The generic
surface expects stable IDs and the exact serialized inputs; prefer a named tool when one
already resolves human names for the same job.

This is full *operation* parity, not yet full programmatic application parity. File-picker
workflows that prepare artifacts still need deterministic path-taking query/service
commands, and the read side does not yet expose every desktop query, dependency graph,
pipeline preview, rendered screenshot, or layout inspection result.

The intended agent flow is:

1. Call `inspect_document`.
2. Call `get_table` only for tables relevant to the task.
3. Use stable IDs and the returned revision when writing.
4. Use a named task-level tool where one fits; otherwise call `describe_operations` and
   pass one returned `Operation` shape to `apply_operation`.
5. Read back a changed table after formula work to check computed values and errors.

## Build and connect Codex

Build the local server:

```sh
cargo build -p framework-mcp
```

In FrameWork, open **Settings → Model Context Protocol** and enable access on
this machine. The same section shows a command for the document currently open
and the server executable when the app can find it. Access is off until this
switch is enabled, and turning it off makes an already-running server refuse
its next tool request.

Register it with Codex using absolute paths:

```sh
codex mcp add framework -- \
  /absolute/path/to/target/debug/framework-mcp \
  --document /absolute/path/to/analysis.fw
```

Claude Code uses the same stdio command:

```sh
claude mcp add framework -- \
  /absolute/path/to/target/debug/framework-mcp \
  --document /absolute/path/to/analysis.fw
```

For other stdio-capable MCP clients, use the configuration shape shown in
Settings: `command` is the absolute `framework-mcp` executable and `args` is
`["--document", "/absolute/path/to/analysis.fw"]`.

Codex desktop, CLI, and IDE configurations are shared. Start a new task after registering the server so it can discover the tools.

For project-scoped configuration, copy `.codex/config.toml.example` to `.codex/config.toml`, replace the placeholder paths, and trust the project when prompted. The example uses Codex's conservative `prompt` approval mode; the tools also advertise whether they are read-only or mutating.

Other MCP-capable clients can launch the same executable over standard input/output. The server also accepts a path directly:

```sh
target/debug/framework-mcp --document /absolute/path/to/analysis.fw
```

Running that command manually will appear to wait; this is expected because an MCP client normally owns the process and communicates with it over JSON-RPC on standard input/output.

## Persistence and current concurrency limit

If the configured document does not exist, the server starts with the demo document and writes the file on its first mutation. `framework.fw` is ignored by Git by default. If an existing file is unreadable or malformed, startup fails without replacing it. Legacy bare JSON documents remain readable, while `.fw` files use the versioned FrameWork wrapper and create their collaboration-data directories beside the document.

For `.fw` documents, MCP writes immutable operation events and imports new shared-drive events before each request. This gives MCP and the desktop the same causal merge and deduplication path. Legacy explicit `.json` documents retain the old single-writer snapshot behavior and must not be edited by multiple processes.

The event journal does not yet define every concurrent structural conflict or make undo/redo collaborative. The next collaboration milestones are explicit conflict semantics, a single-owner local service with change subscriptions, and live event transport over iroh.

## Verification

The server has direct tests for structured inspection, persistence, name-to-ID formula creation, and stale-revision rejection:

```sh
cargo test -p framework-mcp
```

The complete project verification remains:

```sh
npm run build
cargo test --workspace
```
