#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Generate FrameWork's formula-compiler bindings for the Polars 0.55.2 Expr surface.

This reads `pub fn` method signatures (with their leading `///` doc comments)
straight out of the vendored `polars-plan` crate source under
`~/.cargo/registry/src/*/polars-plan-0.55.2/src/dsl/`, classifies each
argument into a small set of generic shapes FrameWork's formula compiler can
already build from a parsed AST (expr / list-of-expr / string / int / float /
bool / optional-of-those), and emits three generated artifacts:

  - crates/framework-core/src/generated_expr_bindings.rs  (Rust dispatch + catalog frame)
  - docs/formula-function-catalog.generated.md            (generated method frame, included from the hand-written doc)
  - src/lib/formulaFunctionCatalog.generated.ts            (TS mirror of the same frame)

Methods whose signature does not fit the generic shapes (closures/UDFs,
options structs, enums, IO/serialization/meta/plugin surfaces, `alias`, and
anything else the classifier cannot map with certainty) are recorded in the
spec as "deferred" with a reason, and are not emitted into the dispatcher.
Deferring is fine; silently misbinding an argument is not.

Usage:
    tools/generate_expr_bindings.py [--polars-src DIR] [--spec-out FILE]

Regenerate after bumping the pinned `polars` version by re-running this
script from the repository root; it re-derives everything from source, so
there is nothing else to hand-edit.
"""

from __future__ import annotations

import argparse
import dataclasses
import glob
import json
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# -----------------------------------------------------------------------
# Sources
# -----------------------------------------------------------------------

# (relative path under polars-plan src/dsl/, namespace id, rust accessor)
# namespace id matches the catalog prefix used elsewhere in the codebase
# ("root" for free Expr instance methods, "str"/"dt"/"list"/"arr"/"struct"/"cat"
# for the namespace structs). rust accessor is the method used to obtain the
# namespace wrapper from an Expr (e.g. `expr.str()`), or None for root.
NAMESPACE_SOURCES: list[tuple[str, str, str | None]] = [
    ("mod.rs", "expr", None),
    ("bitwise.rs", "expr", None),
    ("statistics.rs", "expr", None),
    ("random.rs", "expr", None),
    ("arithmetic.rs", "expr", None),
    ("expr/mod.rs", "expr", None),
    ("string.rs", "str", "str"),
    ("dt.rs", "dt", "dt"),
    ("list.rs", "list", "list"),
    ("array.rs", "arr", "arr"),
    ("struct_.rs", "struct", "struct_"),
    ("cat.rs", "cat", "cat"),
]

# Free (non-method) `pl::` functions we also widen. (relative path, function names to include)
ROOT_FUNCTION_SOURCES: list[tuple[str, list[str] | None]] = [
    ("functions/horizontal.rs", ["all_horizontal", "any_horizontal"]),
    ("functions/concat.rs", ["concat_str", "format_str"]),
    ("functions/repeat.rs", ["repeat"]),
    ("functions/correlation.rs", ["cov", "pearson_corr", "spearman_rank_corr"]),
]

# Methods that must never be bound even if the signature would classify
# cleanly — these are excluded by the project's explicit rules rather than
# by signature shape.
EXCLUDED_NAMES = {
    "alias",  # the calculated-column name supplies the alias
    "to_titlecase",  # gated behind polars' `nightly` cargo feature (unstable rustc); not worth
    # depending on a nightly toolchain for one string method.
}

EXCLUDED_NAME_PREFIXES = (
    "map_",  # UDF/callback surface
    "apply",  # UDF/callback surface (apply, apply_many, ...)
    "try_map_",
    "cache",  # internal plan hint, not a value-producing expression op
)

EXCLUDED_NAME_SUBSTRINGS = (
    "udf",
    "python",
    "serialize",
    "deserialize",
)

# Files that are entirely IO/serialization/meta/plugin/internal-plumbing and
# are skipped outright rather than scanned method-by-method.
_SKIPPED_FILES_NOTE = (
    "meta.rs (meta namespace), udf.rs (python/UDF plugin), "
    "serializable_plan.rs, python_dsl/, scan_sources.rs, builder_dsl.rs "
    "(LazyFrame builder, not Expr), selector.rs (selectors, not bindable "
    "as calculated-column methods), datatype_expr.rs (DataTypeExpr, a "
    "different expression type), name.rs (mostly closures/renaming)."
)

# Method/function ids already implemented by hand in crates/framework-core/src/lib.rs
# (compile_polars_root_call / compile_polars_method). The generator must not
# re-bind these: the hand-written arms are matched first and take precedence,
# so a generated entry here would just be dead code plus a duplicate catalog row.
EXISTING_HANDWRITTEN_IDS = {
    "root.sum_horizontal",
    "root.mean_horizontal",
    "root.min_horizontal",
    "root.max_horizontal",
    "root.coalesce",
    "root.when",
    "root.date",
    "expr.abs",
    "expr.sign",
    "expr.round",
    "expr.round_sig_figs",
    "expr.truncate",
    "expr.floor",
    "expr.ceil",
    "expr.sqrt",
    "expr.cbrt",
    "expr.pow",
    "expr.exp",
    "expr.log",
    "expr.log1p",
    "expr.clip",
    "expr.clip_min",
    "expr.clip_max",
    "expr.floor_div",
    "expr.sin",
    "expr.cos",
    "expr.tan",
    "expr.cot",
    "expr.arcsin",
    "expr.arccos",
    "expr.arctan",
    "expr.arctan2",
    "expr.sinh",
    "expr.cosh",
    "expr.tanh",
    "expr.arcsinh",
    "expr.arccosh",
    "expr.arctanh",
    "expr.degrees",
    "expr.radians",
    "expr.is_null",
    "expr.is_not_null",
    "expr.fill_null",
    "expr.shift",
    "expr.over",
    "expr.sum",
    "expr.mean",
    "expr.min",
    "expr.max",
    "expr.count",
    "expr.len",
    "expr.null_count",
    "expr.rolling_mean",
    "expr.rolling_sum",
    "expr.rolling_min",
    "expr.rolling_max",
    "dt.year",
    "dt.iso_year",
    "dt.quarter",
    "dt.month",
    "dt.week",
    "dt.weekday",
    "dt.ordinal_day",
    "dt.is_leap_year",
    "dt.days_in_month",
    "dt.date",
    "dt.month_start",
    "dt.month_end",
    "dt.offset_by",
    "str.to_uppercase",
    "str.to_lowercase",
    "str.contains",
}

INTEGER_TYPES = {"i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize"}
FLOAT_TYPES = {"f32", "f64"}
STRING_TYPES = {"&str", "String", "PlSmallStr", "&'static str"}


@dataclasses.dataclass
class Arg:
    name: str
    kind: str  # expr | exprs | string | int | float | bool | optional
    inner_kind: str | None = None  # for optional
    rust_type: str = ""


@dataclasses.dataclass
class Method:
    source_file: str
    namespace: str
    accessor: str | None
    rust_name: str
    doc: str
    args: list[Arg]
    status: str  # bound | deferred
    reason: str = ""
    fallible: bool = False

    @property
    def id(self) -> str:
        if self.namespace == "root_fn":
            return f"root.{self.rust_name}"
        prefix = "expr" if self.namespace == "expr" else self.namespace
        return f"{prefix}.{self.rust_name}"


# -----------------------------------------------------------------------
# Rust signature scanning (regex + balanced-delimiter split; not a full
# parser, but sufficient for polars-plan's consistently-formatted source)
# -----------------------------------------------------------------------

FN_START_RE = re.compile(
    r"^\s*pub fn\s+([A-Za-z0-9_]+)\s*(<[^>]*>)?\s*\(", re.MULTILINE
)


def find_polars_plan_src(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit)
    candidates = glob.glob(
        os.path.expanduser("~/.cargo/registry/src/*/polars-plan-0.55.2/src")
    )
    if not candidates:
        raise SystemExit(
            "Could not find vendored polars-plan-0.55.2 source under "
            "~/.cargo/registry/src/*/polars-plan-0.55.2/src. Pass --polars-src."
        )
    return Path(candidates[0])


def extract_balanced(
    text: str, open_pos: int, open_ch: str, close_ch: str
) -> tuple[str, int]:
    """Given text[open_pos] == open_ch, return (inner_text, index_after_close)."""
    depth = 0
    i = open_pos
    while i < len(text):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return text[open_pos + 1 : i], i + 1
        i += 1
    raise ValueError("unbalanced delimiters")


def split_top_level(text: str, sep: str = ",") -> list[str]:
    parts: list[str] = []
    depth = 0
    current = []
    for ch in text:
        if ch in "<([{":
            depth += 1
        elif ch in ">)]}":
            depth -= 1
        if ch == sep and depth == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(ch)
    if "".join(current).strip():
        parts.append("".join(current))
    return [p.strip() for p in parts if p.strip()]


def leading_doc_comment(lines: list[str], fn_line_idx: int) -> str:
    doc_lines: list[str] = []
    i = fn_line_idx - 1
    while i >= 0:
        stripped = lines[i].strip()
        if stripped.startswith("///"):
            doc_lines.insert(0, stripped[3:].strip())
            i -= 1
            continue
        if stripped.startswith("#[") or stripped == "":
            i -= 1
            continue
        break
    return " ".join(doc_lines).strip()


def parse_generic_bounds(generics: str | None) -> dict[str, str]:
    """Map a generic type parameter letter to a normalized bound description."""
    bounds: dict[str, str] = {}
    if not generics:
        return bounds
    inner = generics.strip()
    if inner.startswith("<"):
        inner = inner[1:-1]
    for clause in split_top_level(inner):
        if ":" not in clause:
            continue
        name, bound = clause.split(":", 1)
        name = name.strip()
        bound = bound.strip()
        bounds[name] = bound
    return bounds


def classify_type(rust_type: str, bounds: dict[str, str]) -> tuple[str, str] | None:
    """Return (kind, inner_kind_or_empty) or None if not classifiable."""
    t = rust_type.strip()
    # Strip a leading & or &'a
    t = re.sub(r"^&('[a-zA-Z_]+\s+)?", "", t).strip()

    if t in bounds:
        bound = bounds[t]
        if "Into<Expr>" in bound or "Into < Expr >" in bound:
            return ("expr", "")
        if "AsRef<[Expr]>" in bound or re.search(
            r"AsRef\s*<\s*\[\s*Expr\s*\]\s*>", bound
        ):
            return ("exprs", "")
        if "AsRef<[IE]>" in bound:
            return ("exprs", "")
        if "Into<PlSmallStr>" in bound or "Into<String>" in bound:
            return ("string", "")
        if "Into<Expr>" not in bound:
            return None
        return None

    if t == "Expr" or t == "Self":
        return ("expr", "")
    if t.startswith("Vec<Expr>") or t.startswith("Vec < Expr >"):
        return ("exprs", "")
    if t.startswith("&[Expr]"):
        return ("exprs", "")
    if t == "bool":
        return ("bool", "")
    if t in INTEGER_TYPES:
        return ("int", "")
    if t in FLOAT_TYPES:
        return ("float", "")
    if t in STRING_TYPES:
        return ("string", "")
    if t.startswith("Option<") or t.startswith("Option <"):
        inner = extract_angle_inner(t)
        inner_kind = classify_type(inner, bounds)
        if inner_kind is None:
            return None
        return ("optional", inner_kind[0])
    return None


def extract_angle_inner(t: str) -> str:
    start = t.index("<")
    inner, _ = extract_balanced(t, start, "<", ">")
    return inner.strip()


def parse_args(arg_list: str, bounds: dict[str, str]) -> list[Arg] | None:
    args: list[Arg] = []
    for raw in split_top_level(arg_list):
        raw = raw.strip()
        if raw in ("self", "&self", "&mut self"):
            continue
        if ":" not in raw:
            return None
        name, rust_type = raw.split(":", 1)
        name = name.strip()
        rust_type = rust_type.strip()
        if any(
            tok in rust_type for tok in ("Fn(", "FnMut(", "FnOnce(", "dyn ", "Box<dyn")
        ):
            return None
        classified = classify_type(rust_type, bounds)
        if classified is None:
            return None
        kind, inner = classified
        args.append(
            Arg(name=name, kind=kind, inner_kind=inner or None, rust_type=rust_type)
        )
    return args


def method_return_ok(return_type: str) -> bool:
    rt = return_type.strip()
    if rt in ("Self", "Expr"):
        return True
    m = re.match(r"PolarsResult\s*<\s*(Self|Expr)\s*>$", rt)
    return bool(m)


def scan_file(path: Path, namespace: str, accessor: str | None) -> list[Method]:
    text = path.read_text()
    lines = text.splitlines()
    methods: list[Method] = []
    for m in FN_START_RE.finditer(text):
        name = m.group(1)
        generics = m.group(2)
        # locate opening paren for the arg list (start of match end - 1 is '(')
        open_paren = m.end() - 1
        arg_text, after_args = extract_balanced(text, open_paren, "(", ")")
        rest = text[after_args : after_args + 400]
        rest_stripped = rest.lstrip()
        if rest_stripped.startswith("->"):
            arrow_rest = rest_stripped[2:]
            brace_idx = arrow_rest.find("{")
            where_idx = arrow_rest.find(" where")
            end_idx = (
                brace_idx
                if where_idx == -1
                else min(brace_idx, where_idx if where_idx != -1 else brace_idx)
            )
            return_type = arrow_rest[
                : (brace_idx if brace_idx != -1 else len(arrow_rest))
            ].strip()
            if where_idx != -1 and where_idx < brace_idx:
                return_type = arrow_rest[:where_idx].strip()
        else:
            return_type = "()"

        fn_line_idx = text[: m.start()].count("\n")
        doc = leading_doc_comment(lines, fn_line_idx)

        if name in EXCLUDED_NAMES:
            continue
        if name.startswith(EXCLUDED_NAME_PREFIXES):
            continue
        if any(sub in name.lower() for sub in EXCLUDED_NAME_SUBSTRINGS):
            continue
        if not name[0].isalpha():
            continue

        first_arg = split_top_level(arg_text)[0].strip() if arg_text.strip() else ""
        if first_arg not in ("self", "&self", "&mut self"):
            # Associated function (constructor) rather than an instance method — not
            # callable as `input.name(...)`.
            methods.append(
                Method(
                    source_file=path.name,
                    namespace=namespace,
                    accessor=accessor,
                    rust_name=name,
                    doc=doc,
                    args=[],
                    status="deferred",
                    reason="associated function without a self receiver (constructor, not an instance method)",
                )
            )
            continue

        if not method_return_ok(return_type):
            methods.append(
                Method(
                    source_file=path.name,
                    namespace=namespace,
                    accessor=accessor,
                    rust_name=name,
                    doc=doc,
                    args=[],
                    status="deferred",
                    reason=f"return type `{return_type}` is not Expr/Self",
                )
            )
            continue

        bounds = parse_generic_bounds(generics)
        args = parse_args(arg_text, bounds)
        if args is None:
            methods.append(
                Method(
                    source_file=path.name,
                    namespace=namespace,
                    accessor=accessor,
                    rust_name=name,
                    doc=doc,
                    args=[],
                    status="deferred",
                    reason="argument signature does not fit the generic expr/literal/list shapes",
                )
            )
            continue

        methods.append(
            Method(
                source_file=path.name,
                namespace=namespace,
                accessor=accessor,
                rust_name=name,
                doc=doc,
                args=args,
                status="bound",
            )
        )
    return methods


def scan_root_functions(path: Path, allowlist: list[str] | None) -> list[Method]:
    text = path.read_text()
    lines = text.splitlines()
    methods: list[Method] = []
    for m in FN_START_RE.finditer(text):
        name = m.group(1)
        if allowlist is not None and name not in allowlist:
            continue
        generics = m.group(2)
        open_paren = m.end() - 1
        arg_text, after_args = extract_balanced(text, open_paren, "(", ")")
        rest = text[after_args : after_args + 400].lstrip()
        if rest.startswith("->"):
            arrow_rest = rest[2:]
            brace_idx = arrow_rest.find("{")
            return_type = arrow_rest[
                : (brace_idx if brace_idx != -1 else len(arrow_rest))
            ].strip()
        else:
            return_type = "()"
        fn_line_idx = text[: m.start()].count("\n")
        doc = leading_doc_comment(lines, fn_line_idx)

        if not method_return_ok(return_type):
            continue
        bounds = parse_generic_bounds(generics)
        args = parse_args(arg_text, bounds)
        if args is None:
            methods.append(
                Method(
                    source_file=path.name,
                    namespace="root_fn",
                    accessor=None,
                    rust_name=name,
                    doc=doc,
                    args=[],
                    status="deferred",
                    reason="argument signature does not fit the generic expr/literal/list shapes",
                )
            )
            continue
        methods.append(
            Method(
                source_file=path.name,
                namespace="root_fn",
                accessor=None,
                rust_name=name,
                doc=doc,
                args=args,
                status="bound",
            )
        )
    return methods


def build_spec(polars_src: Path) -> list[Method]:
    dsl = polars_src / "dsl"
    methods: list[Method] = []
    for rel, namespace, accessor in NAMESPACE_SOURCES:
        methods.extend(scan_file(dsl / rel, namespace, accessor))
    for rel, allowlist in ROOT_FUNCTION_SOURCES:
        methods.extend(scan_root_functions(dsl / rel, allowlist))

    # De-duplicate by id, keeping the first (files are visited in a stable order and
    # id collisions only happen for methods re-exported through the prelude).
    seen: dict[str, Method] = {}
    already_handwritten = 0
    for method in methods:
        if method.id in EXISTING_HANDWRITTEN_IDS:
            already_handwritten += 1
            continue
        if method.id not in seen:
            seen[method.id] = method
    print(f"skipped (already hand-written): {already_handwritten}", file=sys.stderr)
    return list(seen.values())


# -----------------------------------------------------------------------
# Rust / docs / TS rendering
# -----------------------------------------------------------------------

NAMESPACE_ACCESSOR = {
    "str": "str",
    "dt": "dt",
    "list": "list",
    "arr": "arr",
    "struct": "struct_",
    "cat": "cat",
}

NAMESPACE_CATEGORY = {
    "expr": "Generated expression methods",
    "str": "Generated string namespace",
    "dt": "Generated date namespace",
    "list": "Generated list namespace",
    "arr": "Generated array namespace",
    "struct": "Generated struct namespace",
    "cat": "Generated categorical namespace",
    "root_fn": "Generated root functions",
}


def path_tokens(m: Method) -> list[str]:
    if m.namespace in ("expr",):
        return [m.rust_name]
    if m.namespace == "root_fn":
        return []
    return [m.namespace, m.rust_name]


def catalog_signature(m: Method) -> str:
    parts = []
    for a in m.args:
        label = a.name
        if a.kind == "optional":
            label += "?"
        parts.append(label)
    joined = ", ".join(parts)
    if m.namespace == "root_fn":
        return f"{m.rust_name}({joined})"
    prefix = "." if m.namespace != "root_fn" else ""
    dotted = ".".join(path_tokens(m)) if m.namespace != "expr" else m.rust_name
    return f"{prefix}{dotted}({joined})"


def catalog_id_str(m: Method) -> str:
    return m.id


def rust_string_literal(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def render_extraction(m: Method) -> tuple[list[str], list[str]]:
    """Return (setup_lines, call_arg_exprs)."""
    lines: list[str] = []
    call_args: list[str] = []
    path_label = ".".join(path_tokens(m)) if m.namespace != "root_fn" else m.rust_name
    for i, a in enumerate(m.args):
        var = f"a{i}"
        missing_msg = rust_string_literal(f".{path_label} expects {a.name}")
        label = rust_string_literal(a.name)
        if a.kind == "expr":
            lines.append(
                f'    let {var} = resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?.to_polars(document)?;"
            )
        elif a.kind == "exprs":
            lines.append(
                f'    let raw_{var} = resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?;"
            )
            lines.append(
                f"    let {var} = flatten_polars_arguments(std::slice::from_ref(raw_{var}), document)?;"
            )
        elif a.kind == "string":
            lines.append(
                f'    let {var} = literal_string(resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?, {label})?;"
            )
        elif a.kind == "int":
            cast = "" if a.rust_type == "i64" else f" as {a.rust_type}"
            lines.append(
                f'    let {var} = literal_int(resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?, {label})?{cast};"
            )
        elif a.kind == "float":
            cast = "" if a.rust_type == "f64" else f" as {a.rust_type}"
            lines.append(
                f'    let {var} = literal_float(resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?, {label})?{cast};"
            )
        elif a.kind == "bool":
            lines.append(
                f'    let {var} = literal_bool(resolve_arg(arguments, keyword_arguments, {i}, "{a.name}")'
                f".ok_or_else(|| {missing_msg}.to_string())?, {label})?;"
            )
        elif a.kind == "optional":
            inner = a.inner_kind
            lines.append(
                f'    let {var} = match resolve_arg(arguments, keyword_arguments, {i}, "{a.name}") {{'
            )
            if inner == "expr":
                lines.append(f"        Some(raw) => Some(raw.to_polars(document)?),")
            elif inner == "string":
                lines.append(
                    f"        Some(raw) => Some(literal_string(raw, {label})?),"
                )
            elif inner == "int":
                inner_type = a.rust_type.removeprefix("Option<").removesuffix(">")
                cast = "" if inner_type == "i64" else f" as {inner_type}"
                lines.append(
                    f"        Some(raw) => Some(literal_int(raw, {label})?{cast}),"
                )
            elif inner == "float":
                inner_type = a.rust_type.removeprefix("Option<").removesuffix(">")
                cast = "" if inner_type == "f64" else f" as {inner_type}"
                lines.append(
                    f"        Some(raw) => Some(literal_float(raw, {label})?{cast}),"
                )
            elif inner == "bool":
                lines.append(f"        Some(raw) => Some(literal_bool(raw, {label})?),")
            else:
                raise ValueError(f"unhandled optional inner kind {inner} in {m.id}")
            lines.append("        None => None,")
            lines.append("    };")
        else:
            raise ValueError(f"unhandled arg kind {a.kind} in {m.id}")
        call_args.append(var)
    return lines, call_args


def render_method_arm(m: Method) -> str:
    tokens = path_tokens(m)
    pattern = "[" + ", ".join(rust_string_literal(t) for t in tokens) + "]"
    lines, call_args = render_extraction(m)
    total = len(m.args)
    path_label = ".".join(tokens)
    guard = render_guard(m.args, total, f".{path_label}")
    accessor = NAMESPACE_ACCESSOR.get(m.namespace)
    receiver = f"input.{accessor}()" if accessor else "input"
    call = f"{receiver}.{m.rust_name}({', '.join(call_args)})"
    tail = (
        f"    {call}.map_err(|error| error.to_string())"
        if m.fallible
        else f"    Ok({call})"
    )
    body = "\n".join(guard + lines + [tail])
    return f"        {pattern} => (|| -> Result<pl::Expr, String> {{\n{body}\n        }})(),"


def render_guard(margs: list[Arg], total: int, label: str) -> list[str]:
    too_many_check = (
        "!arguments.is_empty()" if total == 0 else f"arguments.len() > {total}"
    )
    guard = [
        f"    if {too_many_check} {{",
        f'        return Err("{label} expects at most {total} argument(s)".to_string());',
        "    }",
    ]
    if margs:
        known_names = ", ".join(rust_string_literal(a.name) for a in margs)
        guard += [
            f"    let allowed_keywords: &[&str] = &[{known_names}];",
            "    for (keyword, _) in keyword_arguments {",
            "        if !allowed_keywords.contains(&keyword.as_str()) {",
            f"            return Err(format!(\"{label} does not accept keyword argument '{{keyword}}'\"));",
            "        }",
            "    }",
        ]
    else:
        guard += [
            "    if !keyword_arguments.is_empty() {",
            f'        return Err("{label} does not accept keyword arguments".to_string());',
            "    }",
        ]
    return guard


def render_root_fn_arm(m: Method) -> str:
    lines, call_args = render_extraction(m)
    total = len(m.args)
    guard = render_guard(m.args, total, f"{m.rust_name}(...)")
    call = f"pl::{m.rust_name}({', '.join(call_args)})"
    tail = (
        f"    {call}.map_err(|error| error.to_string())"
        if m.fallible
        else f"    Ok({call})"
    )
    body = "\n".join(guard + lines + [tail])
    return f"        {rust_string_literal(m.rust_name)} => (|| -> Result<pl::Expr, String> {{\n{body}\n        }})(),"


RUST_HEADER = """//! GENERATED FILE — do not edit by hand.
//!
//! Generated by `tools/generate_expr_bindings.py` from the vendored
//! `polars-plan-0.55.2` source (see that script for the full pipeline and the
//! exclusion/deferral rules). Regenerate with:
//!
//! ```text
//! python3 tools/generate_expr_bindings.py
//! ```
//! This module widens the formula compiler's Polars `Expr` surface beyond
//! the hand-written methods in `lib.rs`. It is consulted only after the
//! hand-written `compile_polars_method` / `compile_polars_root_call` arms
//! fail to match a name, so hand-written behavior is unchanged.
#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use crate::{Document, Expr, flatten_polars_arguments, keyword_argument};
use polars::prelude as pl;

fn resolve_arg<'a>(
    arguments: &'a [Expr],
    keyword_arguments: &'a [(String, Expr)],
    index: usize,
    name: &str,
) -> Option<&'a Expr> {
    arguments
        .get(index)
        .or_else(|| keyword_argument(keyword_arguments, name))
}

fn literal_string(expr: &Expr, label: &str) -> Result<String, String> {
    match expr {
        Expr::String { value } => Ok(value.clone()),
        _ => Err(format!("{label} must be a string literal")),
    }
}

fn literal_bool(expr: &Expr, label: &str) -> Result<bool, String> {
    match expr {
        Expr::Boolean { value } => Ok(*value),
        _ => Err(format!("{label} must be True or False")),
    }
}

fn literal_int(expr: &Expr, label: &str) -> Result<i64, String> {
    match expr {
        Expr::Integer { value } => Ok(*value),
        Expr::Number { value } if value.fract() == 0.0 => Ok(*value as i64),
        _ => Err(format!("{label} must be an integer literal")),
    }
}

fn literal_float(expr: &Expr, label: &str) -> Result<f64, String> {
    match expr {
        Expr::Integer { value } => Ok(*value as f64),
        Expr::Number { value } => Ok(*value),
        _ => Err(format!("{label} must be a numeric literal")),
    }
}

/// Dispatch a namespaced or root instance method (`.method(...)` /
/// `.namespace.method(...)`) that is not handled by the hand-written
/// `compile_polars_method`. Returns `None` when `path` is not one of the
/// generated methods, so the caller can fall through to its own error.
pub(crate) fn compile_generated_method(
    input: pl::Expr,
    path: &[String],
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Option<Result<pl::Expr, String>> {
    let path_slice: Vec<&str> = path.iter().map(String::as_str).collect();
    Some(match path_slice.as_slice() {
__METHOD_ARMS__
        _ => return None,
    })
}

/// Dispatch a root (non-method) Polars function call, such as `concat_str(...)`.
/// Returns `None` when `name` is not one of the generated root functions.
pub(crate) fn compile_generated_root_call(
    name: &str,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Option<Result<pl::Expr, String>> {
    Some(match name {
__ROOT_FN_ARMS__
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedFunctionDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub minimum_arguments: usize,
    pub maximum_arguments: usize,
    pub return_type: &'static str,
}

pub(crate) const GENERATED_FORMULA_FUNCTIONS: &[GeneratedFunctionDefinition] = &[
__CATALOG_ENTRIES__
];
"""


def rust_return_type(m: Method) -> str:
    for a in m.args:
        if a.kind == "expr" and a.name in ("condition",):
            pass
    if m.rust_name in (
        "is_null",
        "is_not_null",
        "is_nan",
        "is_not_nan",
        "is_finite",
        "is_infinite",
        "is_unique",
        "is_duplicated",
        "is_first_distinct",
        "is_last_distinct",
        "is_sorted",
        "starts_with",
        "ends_with",
        "contains_literal",
        "contains_any",
    ):
        return "boolean"
    if m.namespace == "str" or m.rust_name in ("to_titlecase",):
        return "string"
    if m.namespace == "dt":
        return "date"
    return "dynamic"


def render_rust(methods: list[Method]) -> str:
    bound = [m for m in methods if m.status == "bound"]
    method_arms = []
    root_arms = []
    catalog_entries = []
    for m in bound:
        if m.namespace == "root_fn":
            root_arms.append(render_root_fn_arm(m))
        else:
            method_arms.append(render_method_arm(m))
        required = sum(1 for a in m.args if a.kind != "optional")
        total = len(m.args)
        doc = m.doc.replace('"', "'") or f"{m.rust_name} (see Polars docs)."
        catalog_entries.append(
            "    GeneratedFunctionDefinition { id: %s, name: %s, category: %s, "
            "signature: %s, description: %s, minimum_arguments: %d, maximum_arguments: %d, "
            "return_type: %s },"
            % (
                rust_string_literal(m.id),
                rust_string_literal(
                    catalog_signature(m).split("(")[0]
                    if m.namespace != "root_fn"
                    else m.rust_name
                ),
                rust_string_literal(NAMESPACE_CATEGORY[m.namespace]),
                rust_string_literal(catalog_signature(m)),
                rust_string_literal(doc[:220]),
                required,
                total,
                rust_string_literal(rust_return_type(m)),
            )
        )
    return (
        RUST_HEADER.replace("__METHOD_ARMS__", "\n".join(method_arms))
        .replace("__ROOT_FN_ARMS__", "\n".join(root_arms))
        .replace("__CATALOG_ENTRIES__", "\n".join(catalog_entries))
    )


def render_docs(methods: list[Method]) -> str:
    bound = [m for m in methods if m.status == "bound"]
    deferred = [m for m in methods if m.status == "deferred"]
    by_ns: dict[str, list[Method]] = {}
    for m in bound:
        by_ns.setdefault(m.namespace, []).append(m)
    lines = [
        "<!-- GENERATED FILE. Do not edit by hand. -->",
        "<!-- Regenerate with: python3 tools/generate_expr_bindings.py -->",
        "",
        "# Generated Polars expression surface",
        "",
        f"Generated from polars-plan 0.55.2. {len(bound)} additional methods/functions are bound "
        f"beyond the hand-written core surface; {len(deferred)} were enumerated but deferred "
        "(closures/UDFs, options-struct/enum arguments, or other shapes the generator cannot "
        "bind with certainty — see `tools/expr_bindings_spec.json` for the full list with reasons).",
        "",
    ]
    order = ["root_fn", "expr", "str", "dt", "list", "arr", "struct", "cat"]
    for ns in order:
        items = sorted(by_ns.get(ns, []), key=lambda m: m.rust_name)
        if not items:
            continue
        lines.append(f"## {NAMESPACE_CATEGORY[ns]}")
        lines.append("")
        lines.append("| Signature | Description |")
        lines.append("| --- | --- |")
        for m in items:
            sig = catalog_signature(m).replace("|", "\\|")
            desc = (m.doc or "See Polars documentation.").replace("|", "\\|")
            lines.append(f"| `{sig}` | {desc} |")
        lines.append("")
    return "\n".join(lines) + "\n"


def render_ts(methods: list[Method]) -> str:
    bound = sorted(
        [m for m in methods if m.status == "bound"],
        key=lambda m: (m.namespace, m.rust_name),
    )

    def ts_str(s: str) -> str:
        return json.dumps(s)

    rows = []
    for m in bound:
        required = sum(1 for a in m.args if a.kind != "optional")
        total = len(m.args)
        rows.append(
            "  [%s, %s, [], %s, %s, %s, %d, %d],"
            % (
                ts_str(m.id),
                ts_str(
                    m.rust_name
                    if m.namespace == "root_fn"
                    else (
                        "." + m.rust_name
                        if m.namespace == "expr"
                        else "." + ".".join(path_tokens(m))
                    )
                ),
                ts_str(NAMESPACE_CATEGORY[m.namespace]),
                ts_str(catalog_signature(m)),
                ts_str((m.doc or "See Polars documentation.")[:220]),
                required,
                total,
            )
        )
    header = """// GENERATED FILE — do not edit by hand.
// Regenerate with: python3 tools/generate_expr_bindings.py
//
// Mirrors crates/framework-core/src/generated_expr_bindings.rs's catalog
// frame for the browser-preview formula autocomplete.
import type { FormulaFunction } from "./types";

const generatedDefinitions: Array<[
  id: string,
  name: string,
  aliases: string[],
  category: string,
  signature: string,
  description: string,
  minimumArguments: number,
  maximumArguments: number,
]> = [
"""
    footer = """];

export const generatedFormulaFunctions: FormulaFunction[] = generatedDefinitions.map((definition) => {
  const [id, name, aliases, category, signature, description, minimumArguments, maximumArguments] = definition;
  return {
    id,
    name,
    aliases,
    category,
    signature,
    description,
    minimumArguments,
    maximumArguments,
    returnType: generatedReturnType(id),
    nullBehavior: "native Polars behavior",
    // Browser preview has no Rust document behind it. Its fallback help is
    // the visible signature; desktop entries carry canonical argument prose.
    arguments: [],
  };
});

function generatedReturnType(id: string): string {
  if (id.startsWith("str.") || id === "expr.to_titlecase") return "string";
  if (id.startsWith("dt.")) return "date";
  if (
    id.includes("is_null") ||
    id.includes("is_not_null") ||
    id.includes("is_nan") ||
    id.includes("is_finite") ||
    id.includes("is_infinite") ||
    id.includes("is_unique") ||
    id.includes("is_duplicated") ||
    id.includes("is_sorted") ||
    id.includes("starts_with") ||
    id.includes("ends_with") ||
    id.includes("contains")
  ) {
    return "boolean";
  }
  return "dynamic";
}
"""
    return header + "\n".join(rows) + "\n" + footer


# -----------------------------------------------------------------------
# Smoke tests
# -----------------------------------------------------------------------

# Methods where a single generic heuristic argument can't produce a
# semantically valid call (e.g. a duration-string vs. a plain numeric
# argument both typed `Expr`, or a value whose valid range depends on the
# fixture in a way the generator can't infer). These stay bound in the
# compiler — only the auto-generated execution smoke test is skipped, with
# the reason recorded here.
SMOKE_TEST_SKIP: dict[str, str] = {
    "expr.repeat_by": "second argument must be an integer-typed list-length expression per row",
    "expr.reshape": "requires a fixed-size shape argument tied to the fixture length",
    "expr.extend_constant": "value must share the column's exact dtype",
    "expr.append": "requires a second Series with a compatible chunk layout",
    "expr.replace": "requires parallel old/new mapping lists of equal length",
    "expr.replace_strict": "requires parallel old/new mapping lists of equal length",
    "expr.map_elements": "excluded surface, listed defensively",
    "str.pad_start": "requires exact fill-character length semantics",
    "str.pad_end": "requires exact fill-character length semantics",
    "list.set_union": "second argument must be a same-dtype list expression",
    "list.set_intersection": "second argument must be a same-dtype list expression",
    "list.set_difference": "second argument must be a same-dtype list expression",
    "list.set_symmetric_difference": "second argument must be a same-dtype list expression",
    "list.contains": "second argument must match the inner list dtype",
    "list.count_matches": "second argument must match the inner list dtype",
    "dt.base_utc_offset": "requires a time-zone-aware Datetime; FrameWork's Date fixture can't provide one",
    "dt.dst_offset": "requires a time-zone-aware Datetime; FrameWork's Date fixture can't provide one",
    "dt.time": "requires a Datetime/Time value; FrameWork has no Datetime column type",
    "dt.datetime": "requires a Datetime value; FrameWork has no Datetime column type",
    "dt.hour": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.minute": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.second": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.millisecond": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.microsecond": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.nanosecond": "sub-day granularity requires Datetime; FrameWork has no Datetime column type",
    "dt.total_days": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_hours": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_minutes": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_seconds": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_milliseconds": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_microseconds": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "dt.total_nanoseconds": "operates on a Duration expression (e.g. a date difference), not a bare Date column",
    "str.base64_decode": "returns a Binary result; computed columns cannot yet display that dtype",
    "str.hex_decode": "returns a Binary result; computed columns cannot yet display that dtype",
    "str.extract_all": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.split": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.split_inclusive": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.splitn": "returns a Struct result; computed columns cannot yet display nested dtypes",
    "str.extract_many": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.find_many": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.split_exact": "returns a Struct result; computed columns cannot yet display nested dtypes",
    "str.split_exact_inclusive": "returns a Struct result; computed columns cannot yet display nested dtypes",
    "str.split_regex": "returns a List result; computed columns cannot yet display nested dtypes",
    "str.split_regex_inclusive": "returns a List result; computed columns cannot yet display nested dtypes",
    "expr.agg_groups": "Polars panics (internal error, not a FrameWork bug) when used outside a real group-by context",
    "expr.cumulative_eval": "the evaluation argument must itself reference a per-window placeholder; a generic literal isn't representative and panics in Polars",
    # These change row cardinality (drop/dedupe/sample/slice rows), so they cannot
    # appear as a plain per-row computed column in FrameWork's frame model outside a
    # real group-by/window — Polars correctly rejects the length mismatch at
    # execution. The binding itself is still valid Expr construction.
    "expr.gather_every": "changes row cardinality; not usable as a plain computed column",
    "expr.drop_nulls": "changes row cardinality; not usable as a plain computed column",
    "expr.drop_nans": "changes row cardinality; not usable as a plain computed column",
    "expr.unique": "changes row cardinality; not usable as a plain computed column",
    "expr.unique_stable": "changes row cardinality; not usable as a plain computed column",
    "expr.arg_unique": "changes row cardinality; not usable as a plain computed column",
    "expr.sample_n": "changes row cardinality; not usable as a plain computed column",
    "expr.sample_frac": "changes row cardinality; not usable as a plain computed column",
    "expr.head": "changes row cardinality unless length equals the fixture size",
    "expr.tail": "changes row cardinality unless length equals the fixture size",
    "expr.slice": "changes row cardinality unless it spans the whole fixture",
    "expr.rle": "returns a run-length-encoded struct of different cardinality",
    "expr.explode": "changes row cardinality",
    "expr.item": "requires exactly one value in the group and is not a per-row op",
    "expr.hist": "bins and bin_count are mutually exclusive; a generic heuristic can't satisfy an either/or constraint",
    "expr.implode": "returns a List result; computed columns cannot yet display nested dtypes",
}

# Namespaces whose real-execution smoke coverage is structurally limited by
# FrameWork's current column model:
#   - cat: Categorical columns are still backed by a Utf8 Series (see
#     docs/architecture.md's "Polars boundary" note), so `.cat.*` methods
#     hit a Polars dtype error against any fixture column FrameWork can
#     construct today, even though the binding itself is correct.
#   - struct: FrameWork has no Struct column type and the formula surface
#     has no struct-constructing function, so there is no struct-typed
#     expression to call `.struct.*` on.
#   - list / arr: most `.list.*` / `.arr.*` methods return a List/Array
#     result, and computed columns cannot yet display nested dtypes
#     ("Polars output type list[...] is not yet displayable as a frame
#     column" — architecture.md: "Nested list/array/struct dtypes ... can be
#     added directly to the compiler as FrameWork gains display and editing
#     support for those result types"). The handful of list/arr methods that
#     reduce to a scalar (sum/mean/len/...) could be tested individually,
#     but the generic per-namespace smoke test can't tell which without
#     re-deriving Polars' own return-type rules, so the whole namespace is
#     left to manual/targeted tests instead.
# All four namespaces remain fully bound in the compiler; only the
# generated *execution* smoke test is skipped.
SMOKE_TEST_NAMESPACE_SKIP = {"cat", "struct", "list", "arr"}


def sample_call_text(namespace: str, arg: Arg) -> str:
    name = arg.name.lower()
    effective = arg.inner_kind if arg.kind == "optional" else arg.kind
    if effective == "bool":
        if name == "strict" and namespace == "str":
            return "False"
        return "True"
    if effective == "int":
        if name == "ddof":
            return "1"
        if name == "seed":
            return "7"
        if name in (
            "width",
            "n",
            "length",
            "k",
            "index",
            "group_index",
            "digits",
            "decimals",
            "scale",
        ):
            return "1"
        return "1"
    if effective == "float":
        if "frac" in name:
            return "0.5"
        return "1.5"
    if effective == "string":
        if namespace == "dt" and name in ("every", "by"):
            return '"1mo"'
        if name in ("separator", "sep"):
            return '"-"'
        return '"a"'
    if effective == "expr":
        if namespace == "str":
            if name in ("n", "length", "index", "width", "group_index", "n_field"):
                return "1"
            if name in ("separator", "sep"):
                return '"-"'
            return '"a"'
        if namespace == "dt":
            if name in ("every", "offset"):
                return '"1mo"'
            if name == "ambiguous":
                return '"raise"'
            return "1"
        if namespace == "cat":
            return '"a"'
        if namespace in ("list", "arr"):
            if name in ("index", "offset"):
                return "0"
            if name in ("n", "length", "k"):
                return "1"
            if name in ("separator", "sep"):
                return '"-"'
            return "`Num`"
        # root instance methods: receiver is the `Num` numeric column
        if name == "by":
            return "`Num`"
        if name == "k":
            # top_k/bottom_k return exactly k values (not broadcast), so k must equal
            # the fixture's row count for the result to fit the frame.
            return "3"
        return "2"
    if effective == "exprs":
        return "[`Num`, `Num`]"
    raise ValueError(f"no sample for kind {effective}")


# Per-method literal overrides, keyed by method id, for arguments whose valid
# values can't be derived from the argument's name/kind alone (e.g. a JSONPath
# expression must actually parse as one). Maps arg index -> literal text.
SMOKE_TEST_ARG_OVERRIDES: dict[str, dict[int, str]] = {
    "str.json_path_match": {0: '"$.a"'},
}


def render_smoke_call(m: Method) -> str:
    overrides = SMOKE_TEST_ARG_OVERRIDES.get(m.id, {})
    parts = [
        overrides.get(i, sample_call_text(m.namespace, a)) for i, a in enumerate(m.args)
    ]
    joined = ", ".join(parts)
    if m.namespace == "root_fn":
        return f"{m.rust_name}({joined})"
    if m.namespace == "expr":
        receiver = "`Num`"
        if m.rust_name.startswith("bitwise_") or m.rust_name in ("not",):
            receiver = "`Flag`"
        return f"{receiver}.{m.rust_name}({joined})"
    receiver = (
        "`Words`"
        if m.namespace == "str"
        else "`When`"
        if m.namespace == "dt"
        else "`Num`"
    )
    if m.namespace in ("list", "arr"):
        base = "`Num`.implode(True)"
        if m.namespace == "arr":
            base += ".list().to_array(3)"
        return f"{base}.{m.namespace}.{m.rust_name}({joined})"
    return f"{receiver}.{m.namespace}.{m.rust_name}({joined})"


RUST_TEST_HEADER = """//! GENERATED FILE — do not edit by hand.
//! Regenerate with: python3 tools/generate_expr_bindings.py
//!
//! Execution smoke tests for the generated Polars expression bindings in
//! `generated_expr_bindings.rs`. Each formula below is added as a computed
//! column against a small fixed fixture frame and must parse, compile, and
//! execute (collect) without error. This does not assert the *value* each
//! method produces — only that the generated dispatch code is wired
//! correctly end to end. See `tools/generate_expr_bindings.py`'s
//! `SMOKE_TEST_SKIP` / `SMOKE_TEST_NAMESPACE_SKIP` for methods intentionally
//! left out, and why.
#![cfg(test)]
#![allow(clippy::too_many_lines)]

use crate::*;

    fn fixture_store() -> Store {
        let mut store = Store::new(Document {
            id: id(),
            name: "Generated bindings fixture".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        });
        store
            .apply(Operation::AddFrame {
                name: "Fixture".into(),
                grid: vec![
                    vec!["Num".into(), "Words".into(), "Flag".into(), "When".into()],
                    vec!["4".into(), "Hello World".into(), "True".into(), "2024-01-15".into()],
                    vec!["9".into(), "polars rocks".into(), "False".into(), "2024-03-20".into()],
                    vec!["16".into(), "  trim me  ".into(), "True".into(), "2024-06-01".into()],
                ],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        store
    }

    fn frame_id(store: &Store) -> Id {
        store
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn assert_formula_executes(store: &mut Store, frame_id: &Id, name: &str, formula: &str) {
        let result = store.apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: name.into(),
            formula: formula.into(),
            after_column_id: None,
        });
        assert!(
            result.is_ok(),
            "formula `{formula}` (as column `{name}`) failed: {:?}",
            result.err()
        );
    }

    fn assert_ordered_formula_executes(name: &str, formula: &str) {
        let mut store = fixture_store();
        let frame_id = frame_id(&store);
        let num_id = store.document.frame(&frame_id).unwrap().columns[0].id.clone();
        store
            .apply(Operation::SetFramePipeline {
                frame_id: frame_id.clone(),
                steps: vec![FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: num_id,
                        descending: false,
                    }],
                }],
            })
            .unwrap();
        assert_formula_executes(&mut store, &frame_id, name, formula);
    }

__TEST_FNS__
"""


def render_smoke_tests(methods: list[Method]) -> str:
    bound = [
        m
        for m in methods
        if m.status == "bound"
        and m.namespace not in SMOKE_TEST_NAMESPACE_SKIP
        and m.id not in SMOKE_TEST_SKIP
    ]
    by_ns: dict[str, list[Method]] = {}
    for m in bound:
        by_ns.setdefault(m.namespace, []).append(m)

    fns = []
    for ns, items in sorted(by_ns.items()):
        items = sorted(items, key=lambda m: m.rust_name)
        lines = [
            f"    #[test]",
            f"    fn generated_{ns}_bindings_execute_without_error() {{",
            "        let mut store = fixture_store();",
            "        let frame_id = frame_id(&store);",
        ]
        for i, m in enumerate(items):
            formula = render_smoke_call(m)
            col_name = f"gen_{ns}_{i}_{m.rust_name}"
            if m.rust_name.startswith("cum_"):
                lines.append(
                    f"        assert_ordered_formula_executes({rust_string_literal(col_name)}, {rust_string_literal(formula)});"
                )
            else:
                lines.append(
                    f"        assert_formula_executes(&mut store, &frame_id, {rust_string_literal(col_name)}, {rust_string_literal(formula)});"
                )
        lines.append("    }")
        fns.append("\n".join(lines))
    return RUST_TEST_HEADER.replace("__TEST_FNS__", "\n\n".join(fns))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--polars-src", default=None)
    parser.add_argument(
        "--spec-out",
        default=str(REPO_ROOT / "tools" / "expr_bindings_spec.json"),
    )
    parser.add_argument(
        "--no-render", action="store_true", help="Only write the JSON spec"
    )
    args = parser.parse_args()

    polars_src = find_polars_plan_src(args.polars_src)
    methods = build_spec(polars_src)

    # Determine fallibility (PolarsResult<...> return) by re-scanning; build_spec doesn't
    # currently retain it, so recompute here for simplicity by checking source text.
    annotate_fallible(methods, polars_src)

    bound = [m for m in methods if m.status == "bound"]
    deferred = [m for m in methods if m.status == "deferred"]

    spec = {
        "polars_version": "0.55.2",
        "generator": "tools/generate_expr_bindings.py",
        "skipped_files_note": _SKIPPED_FILES_NOTE,
        "counts": {
            "enumerated": len(methods),
            "bound": len(bound),
            "deferred": len(deferred),
        },
        "methods": [dict(id=m.id, **dataclasses.asdict(m)) for m in methods],
    }
    Path(args.spec_out).write_text(json.dumps(spec, indent=2, sort_keys=True))
    print(
        f"enumerated={len(methods)} bound={len(bound)} deferred={len(deferred)}",
        file=sys.stderr,
    )

    if args.no_render:
        return

    rust_out = (
        REPO_ROOT / "crates" / "framework-core" / "src" / "generated_expr_bindings.rs"
    )
    rust_out.write_text(render_rust(methods))

    docs_out = REPO_ROOT / "docs" / "formula-function-catalog.generated.md"
    docs_out.write_text(render_docs(methods))

    ts_out = REPO_ROOT / "src" / "lib" / "formulaFunctionCatalog.generated.ts"
    ts_out.write_text(render_ts(methods))

    tests_out = (
        REPO_ROOT
        / "crates"
        / "framework-core"
        / "src"
        / "generated_expr_bindings_tests.rs"
    )
    tests_out.write_text(render_smoke_tests(methods))


def annotate_fallible(methods: list[Method], polars_src: Path) -> None:
    dsl = polars_src / "dsl"
    file_texts: dict[str, str] = {}
    for m in methods:
        if m.status != "bound":
            m.fallible = False
            continue
        key = m.source_file
        if key not in file_texts:
            # search across all namespace source paths sharing this filename
            found = None
            for rel, _, _ in NAMESPACE_SOURCES:
                if Path(rel).name == key:
                    found = dsl / rel
                    break
            if found is None:
                for rel, _ in ROOT_FUNCTION_SOURCES:
                    if Path(rel).name == key:
                        found = dsl / rel
                        break
            file_texts[key] = found.read_text() if found else ""
        text = file_texts[key]
        m2 = re.search(rf"pub fn\s+{re.escape(m.rust_name)}\s*(<[^>]*>)?\s*\(", text)
        fallible = False
        if m2:
            open_paren = m2.end() - 1
            _, after_args = extract_balanced(text, open_paren, "(", ")")
            rest = text[after_args : after_args + 200].lstrip()
            if rest.startswith("->") and "PolarsResult" in rest[:60]:
                fallible = True
        m.fallible = fallible


if __name__ == "__main__":
    main()
