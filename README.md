# FrameWork

[![CI](https://github.com/kailukowiak/framework/actions/workflows/ci.yml/badge.svg)](https://github.com/kailukowiak/framework/actions/workflows/ci.yml)

FrameWork is an early desktop prototype of a free-form canvas for structured data. It is aimed first at dataframe-literate analysts who repeatedly reach for Excel or Google Sheets because they want to see and manipulate their data directly.

The current vertical slice proves the most important parts of the concept:

- dense calculation blocks and table objects on a movable canvas;
- table cells identified by table, row, and column IDs rather than global cell objects;
- categorical columns with persisted, ordered allowed values and dropdown editing;
- calculated columns whose parsed expressions hold stable object/column references;
- keyboard and pointer autocomplete for table columns and canvas values;
- safe renaming of referenced values and columns;
- visible, one-cell formula overrides;
- an always-visible append row plus direct literal-column insertion and type editing;
- table summaries;
- exact left/inner joins with enforced unique lookup keys, match previews, and further Wrangle transformations on the joined result;
- CSV/TSV paste-to-table creation;
- dependency-safe row, column, and object deletion with toolbar and keyboard undo/redo;
- automatic local persistence in the native app;
- a local, structured MCP API for AI agents and scripts;
- a FrameWork context menu for canvas and table actions;
- circular column dependency rejection.

## Download

[**Download the latest release**](https://github.com/kailukowiak/framework/releases/latest) — no account, terminal, or toolchain required.

| Your system | File to pick |
| --- | --- |
| macOS (Apple Silicon: M1 or later) | the one ending in **`.dmg`** |
| Windows | the one ending in **`-setup.exe`** |
| Ubuntu / Debian / Mint | the one ending in **`.deb`** |
| Fedora / RHEL / openSUSE | the one ending in **`.rpm`** |
| Any other Linux | the one ending in **`.AppImage`** |

These builds are not code-signed yet, so the first launch takes one extra
confirmation — an unsigned application is simply one that has not paid for a
certificate, and you only do this once. On macOS, drag FrameWork to
Applications, let the first launch be refused, then open **System Settings →
Privacy & Security** and click **Open Anyway**. On Windows, click **More info**
and then **Run anyway** on the blue SmartScreen box. On Linux the `.deb` and
`.rpm` install through the usual software centre; an `.AppImage` needs to be
marked executable first (right-click → Properties → Permissions).

Installed builds register `.fw` as a FrameWork document type, so double-clicking
a document opens it.

## Run it from source

Requirements: Node.js, npm, and the Rust toolchain — the channel is pinned by
`rust-toolchain.toml`, so rustup provisions it for you.

On Linux the webview and two of connectorx's backends additionally need system
packages. On Debian and Ubuntu:

```sh
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev libssl-dev libkrb5-dev libgtk-3-dev \
  build-essential curl wget file patchelf
```

`libssl-dev` and `libkrb5-dev` are the two that are easy to miss: connectorx
reaches OpenSSL for TLS and, through its SQL Server backend, GSSAPI for
Kerberos. macOS and Windows need no equivalent step, because both resolve
against the platform's own stack — Security.framework and GSS.framework on
macOS, Schannel and SSPI on Windows.

```sh
npm install
npm run tauri dev
```

FrameWork runs through Tauri; there is no browser-only preview. The application uses the canonical Rust core and opens ordinary, cross-platform `.fw` document files. Installed builds register `.fw` as a FrameWork document type, so a document can be opened from Explorer, Finder, or a Linux file manager.

A launch that is handed a document opens it. A launch that is not starts on an empty canvas in a temporary directory and raises the Data library, so nobody lands in a document they did not ask for — including under `tauri dev`, where every Rust edit relaunches the app. That scratch canvas is genuinely throwaway: Save As is what turns it into a document.

The Commerce join playground is one of the sample documents the library offers. It includes Transactions, Customers, Products, Sales reps, and Regions; lookup keys are already enforced, while a few fact rows deliberately do not match so left and inner join behavior is easy to compare.

The Excel import workbook sample is the interchange counterpart: its source
`.xlsx` has two sheets and three defined Excel Tables, including two separate
tables on one sheet. The generated `.fw` sample contains all three as static,
cached-value imports and carries their Parquet artifacts into every fresh
working copy.

The desktop Data library recursively discovers local `.fw` examples in `.framework-samples/`. That folder is git-ignored, and sample documents open as fresh working copies so experiments never modify the originals. Regenerate the canonical and deterministic synthetic library with `cargo run -p framework-core --example generate_sample_documents`; dataset notes and sources live in [examples/datasets/README.md](examples/datasets/README.md).

The `.fw` file is a versioned JSON snapshot, not a macOS package. The desktop and MCP adapter write immutable, per-writer operation events beside it under `.framework/<document-id>/events/` and merge causally ready events synchronized by a shared drive. The desktop refreshes the open document automatically; MCP rescans before each request. This is the shared-drive transport foundation; live peer collaboration and full concurrent-conflict semantics are later stages.

## AI and scripting access

FrameWork includes a local MCP server so an AI client can inspect and edit the semantic document model directly—without clicking the UI or using browser automation.

Build the server, then enable **Model Context Protocol** in FrameWork Settings. That
machine-local switch is off by default and the same section shows setup commands for
Codex, Claude Code, and generic stdio MCP clients.

```sh
cargo build -p framework-mcp
codex mcp add framework \
  -- /absolute/path/to/target/debug/framework-mcp \
  --document /absolute/path/to/analysis.fw
```

Restart the AI client after adding the server, then ask it to inspect the FrameWork document. The server exposes structured tools for values, tables, rows, columns, cells, formulas, summaries, and history. See [docs/automation-api.md](docs/automation-api.md) for the tool contract, configuration options, and current collaboration limits.

## Verify it

```sh
npm run build
cargo test --workspace
npm audit
```

## Repository map

```text
crates/framework-core/   Canonical document, operations, formulas, computation
crates/framework-mcp/    Local AI/programmatic MCP adapter
src-tauri/               Thin desktop command and persistence adapter
src/                     React canvas and inspectors
docs/                    Product and architecture decisions
ProjectSpec.md           Full product vision, living roadmap/backlog (§34), decision log (§35)
```

## Current formula language

FrameWork formulas are Polars expressions. The only syntax substitution is that an exact backtick reference replaces `pl.col(...)`:

```text
`Quantity` * `Unit price`
(`Weight` / (`Height` ** 2)).round(2)
`Birthdate`.dt.year()
`Amount`.sum().over(`Category`)
when(`Amount` > 100).then("High").otherwise("Low")
```

The calculated-column name supplies the resulting alias. Normal precedence, parentheses, literals, comparisons, keyword arguments, expression lists, method chains, and Polars namespaces are supported. Formula inputs autocomplete exact columns, block values, tables, root functions, and expression methods; use the arrow keys and `Tab` to insert one. Suggestions are filtered by the receiver's inferred type, and Command/Ctrl+Return commits an edit.

Expressions are parsed by FrameWork and compiled directly to Rust Polars `Expr` objects—formula text is never evaluated as Python. Polars performs type checking, null propagation, aggregation/window behavior, and execution, and its errors are shown to the user. The current exposed surface includes numeric and trigonometric methods, null handling, strings, calendar dates, horizontal functions, conditionals, aggregations, shifts, grouped windows, and rolling windows. See [docs/formula-function-catalog.md](docs/formula-function-catalog.md).

Table cards can be resized and use an internally scrolling, virtualized row viewport, so React only mounts the visible rows plus a small overscan buffer. Tables use paged lazy reads and support filtering, projection, sorting, aggregation, pivots, and exact many-to-one joins backed by enforced unique lookup keys. Joined results remain ordinary Wrangle inputs rather than terminal reports.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you shall be dual licensed as above, without any additional terms or conditions.
