//! Type-aware formula autocomplete.
//!
//! The type checker here *is* Polars: instead of re-implementing dtype inference,
//! we compile the sub-expression left of the cursor with the same parser/compiler
//! the rest of the engine uses, then ask a lazy Polars plan what dtype it produces
//! (`LazyFrame::collect_schema`, which resolves schema without materializing rows).
//! If the receiver doesn't parse or doesn't compile we degrade to untyped
//! suggestions — this module never returns an error to its caller.

use crate::formula::ast::reference_matches;
use crate::formula::lexer::{FormulaReference, ReferenceName, Token, tokenize};
use crate::{
    Column, DataObject, Document, Expr, FormulaFunction, FrameObject, Layer, SeriesObject,
    ValueObject,
};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use polars::prelude as pl;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

const PROBE_ALIAS: &str = "__framework_completion_probe";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SuggestionKind {
    Frame,
    Column,
    Value,
    RootFunction,
    Namespace,
    Method,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
// The frontend calls this `CompletionSuggestion`; the alias is applied where
// the bindings are re-exported, so the name here stays the one the completer
// reads best from.
#[ts(export)]
pub struct Suggestion {
    pub id: String,
    pub label: String,
    pub insert_text: String,
    pub kind: SuggestionKind,
    pub detail: String,
    /// A fuzzy-match score, which ts-rs would otherwise render as `bigint`
    /// because the Rust side is 64-bit. It is a ranking number in the low
    /// hundreds, and JSON carries it as an ordinary JavaScript number.
    #[ts(as = "i32")]
    pub score: i64,
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<u32>>")]
    pub match_indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CompletionResult {
    /// Character offset into the formula text where `insert_text` should replace
    /// through `cursor_pos` — i.e. the start of whatever the user has typed so
    /// far for this token (empty when nothing has been typed yet).
    pub replace_start: usize,
    pub receiver_dtype: Option<String>,
    pub namespace: Option<String>,
    pub suggestions: Vec<Suggestion>,
    pub note: Option<String>,
    /// Catalog entry for the innermost call containing the cursor, when the
    /// partial source names one. The UI already owns the catalog's prose; it
    /// only needs the typed cursor context and current argument from core.
    pub active_function_id: Option<String>,
    pub active_argument: Option<usize>,
}

fn empty_result(cursor: usize) -> CompletionResult {
    CompletionResult {
        replace_start: cursor,
        receiver_dtype: None,
        namespace: None,
        suggestions: Vec::new(),
        note: None,
        active_function_id: None,
        active_argument: None,
    }
}

/// Entry point. `cursor_pos` is a character (not byte) offset into `formula_text`.
pub fn complete_formula(
    document: &Document,
    frame_id: &str,
    formula_text: &str,
    cursor_pos: usize,
) -> CompletionResult {
    let Ok(frame) = document.frame(frame_id) else {
        return empty_result(cursor_pos.min(formula_text.chars().count()));
    };
    complete_formula_in_scope(document, frame, frame_id, formula_text, cursor_pos)
}

/// Completion against an explicit scope rather than a frame's own columns.
///
/// A formula inside a chain sees what the steps *before it* leave behind,
/// which is neither the frame's data schema nor its final output — after a
/// summarize, the source columns are gone and the aggregates are what
/// exist. Passing the scope in is what lets the editor ask about a position
/// in a chain that has not been saved.
pub fn complete_formula_in_scope(
    document: &Document,
    frame: &FrameObject,
    frame_id: &str,
    formula_text: &str,
    cursor_pos: usize,
) -> CompletionResult {
    let chars: Vec<char> = formula_text.chars().collect();
    let cursor = cursor_pos.min(chars.len());

    let context = scan_cursor_context(&chars, cursor);
    let partial_len = match &context {
        CursorContext::Backtick { partial }
        | CursorContext::Root { partial }
        | CursorContext::AfterDot { partial, .. } => partial.chars().count(),
    };
    let mut result = match context {
        CursorContext::Backtick { partial } => complete_backtick(document, frame, &partial),
        CursorContext::Root { partial } => complete_root(document, frame, &partial),
        CursorContext::AfterDot {
            receiver_text,
            path,
            partial,
        } => complete_after_dot(document, frame, frame_id, &receiver_text, &path, &partial),
    };
    result.replace_start = cursor - partial_len;
    if let Some((spelling, argument)) = active_call_at_cursor(&chars, cursor) {
        result.active_function_id = function_id_for_spelling(&spelling);
        if result.active_function_id.is_some() {
            result.active_argument = Some(argument);
        }
    }
    result
}

/// The innermost call whose closing parenthesis is still to the right of the
/// cursor. This deliberately scans incomplete text rather than asking the
/// parser: parameter help is most useful between the opening parenthesis and
/// the moment the expression becomes valid.
fn active_call_at_cursor(chars: &[char], cursor: usize) -> Option<(String, usize)> {
    #[derive(Debug)]
    enum Open {
        Call { spelling: String, argument: usize },
        Group,
        Bracket,
    }

    let mut open = Vec::new();
    let mut index = 0;
    while index < cursor {
        match chars[index] {
            '"' => {
                index += 1;
                while index < cursor {
                    if chars[index] == '\\' {
                        index = (index + 2).min(cursor);
                    } else if chars[index] == '"' {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            '`' => {
                index += 1;
                while index < cursor {
                    if chars[index] == '`' && chars.get(index + 1) == Some(&'`') {
                        index += 2;
                    } else if chars[index] == '`' {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            '(' => {
                let spelling = callable_before(chars, index);
                open.push(if spelling.is_empty() {
                    Open::Group
                } else {
                    Open::Call {
                        spelling,
                        argument: 0,
                    }
                });
            }
            '[' => open.push(Open::Bracket),
            ')' | ']' => {
                open.pop();
            }
            ',' => {
                if let Some(Open::Call { argument, .. }) = open.last_mut() {
                    *argument += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    open.into_iter().rev().find_map(|item| match item {
        Open::Call { spelling, argument } => Some((spelling, argument)),
        Open::Group | Open::Bracket => None,
    })
}

fn callable_before(chars: &[char], opening: usize) -> String {
    let mut end = opening;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == '.')
    {
        start -= 1;
    }
    let spelling: String = chars[start..end].iter().collect();
    if spelling.eq_ignore_ascii_case("frame.len") {
        spelling
    } else if let Some(dot) = spelling.find('.') {
        spelling[dot..].to_string()
    } else {
        spelling
    }
}

fn function_id_for_spelling(spelling: &str) -> Option<String> {
    let method = spelling.starts_with('.');
    let normalized = spelling.trim_start_matches('.').to_lowercase();
    crate::formula_function_catalog()
        .into_iter()
        .find(|function| {
            function.name.starts_with('.') == method
                && (function.name.trim_start_matches('.').to_lowercase() == normalized
                    || function
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase() == normalized))
        })
        .map(|function| function.id)
}

// ---------------------------------------------------------------------------
// Cursor-context scanning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum CursorContext {
    /// Cursor sits inside an unclosed `` ` `` — suggest column/value names.
    Backtick { partial: String },
    /// Start of formula, after an operator, `(`, `[`, or `,` — columns + root functions.
    Root { partial: String },
    /// Cursor follows `receiver.path0.path1...partial`. `path` holds namespace
    /// segments already typed (e.g. `["dt"]` for `` `Col`.dt.y ``); `receiver_text`
    /// is handed verbatim to the real parser.
    AfterDot {
        receiver_text: String,
        path: Vec<String>,
        partial: String,
    },
}

fn scan_cursor_context(chars: &[char], cursor: usize) -> CursorContext {
    let before = &chars[..cursor];
    if let Some(partial) = unmatched_backtick_partial(before) {
        return CursorContext::Backtick { partial };
    }

    let mut idx = cursor;
    let partial = consume_ident_backward(chars, &mut idx);
    skip_ws_backward(chars, &mut idx);

    if idx > 0 && chars[idx - 1] == '.' {
        idx -= 1;
        let mut path: Vec<String> = Vec::new();
        loop {
            skip_ws_backward(chars, &mut idx);
            if idx == 0 {
                break;
            }
            let boundary_char = chars[idx - 1];
            if boundary_char == ')' || boundary_char == '`' || boundary_char == ']' {
                // A completed call, a backtick reference, or a list literal ends the
                // in-progress namespace chain; everything up to here is the receiver.
                break;
            }
            if boundary_char.is_alphanumeric() || boundary_char == '_' {
                let before_segment = idx;
                let segment = consume_ident_backward(chars, &mut idx);
                skip_ws_backward(chars, &mut idx);
                if idx > 0 && chars[idx - 1] == '.' {
                    path.push(segment);
                    idx -= 1;
                    continue;
                }
                // Not preceded by another dot: this word is the receiver itself
                // (e.g. a bare `true`/`null` literal), not a namespace segment.
                idx = before_segment;
                break;
            }
            // Operator, `(`, `,`, `[`, or start of formula: no receiver to type-check.
            break;
        }
        path.reverse();
        let receiver_text: String = chars[..idx].iter().collect();
        return CursorContext::AfterDot {
            receiver_text,
            path,
            partial,
        };
    }

    CursorContext::Root { partial }
}

fn consume_ident_backward(chars: &[char], idx: &mut usize) -> String {
    let end = *idx;
    while *idx > 0 && (chars[*idx - 1].is_alphanumeric() || chars[*idx - 1] == '_') {
        *idx -= 1;
    }
    chars[*idx..end].iter().collect()
}

fn skip_ws_backward(chars: &[char], idx: &mut usize) {
    while *idx > 0 && chars[*idx - 1].is_whitespace() {
        *idx -= 1;
    }
}

/// If `before` ends inside an unclosed `` ` `` pair, returns the text typed since
/// the last opening backtick. Doubled backticks (the formula language's escape for
/// a literal backtick inside a name) are treated as a close+reopen, which is a
/// deliberate simplification — see module tests for the accepted behavior.
fn unmatched_backtick_partial(before: &[char]) -> Option<String> {
    let mut inside = false;
    let mut start = 0usize;
    for (index, &character) in before.iter().enumerate() {
        if character == '`' {
            if inside {
                inside = false;
            } else {
                inside = true;
                start = index + 1;
            }
        }
    }
    inside.then(|| before[start..].iter().collect())
}

// ---------------------------------------------------------------------------
// Dtype resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DtypeFamily {
    Numeric,
    String,
    Date,
    Boolean,
    List,
    Array,
    Struct,
    Categorical,
    Other,
}

fn dtype_family(dtype: &pl::DataType) -> DtypeFamily {
    use pl::DataType::*;
    match dtype {
        Int8
        | Int16
        | Int32
        | Int64
        | Int128
        | UInt8
        | UInt16
        | UInt32
        | UInt64
        | UInt128
        | Float16
        | Float32
        | Float64
        | Decimal(_, _) => DtypeFamily::Numeric,
        String => DtypeFamily::String,
        Date | Datetime(_, _) | Duration(_) => DtypeFamily::Date,
        Boolean => DtypeFamily::Boolean,
        List(_) => DtypeFamily::List,
        // A fixed-width array and a struct each have their own namespace, and
        // a column of one is nothing like a column of the other.
        Array(..) => DtypeFamily::Array,
        Struct(..) => DtypeFamily::Struct,
        Categorical(..) | Enum(..) => DtypeFamily::Categorical,
        _ => DtypeFamily::Other,
    }
}

fn describe_dtype(dtype: &pl::DataType) -> String {
    // Polars prints an enum as `Enum([...])`, which tells you nothing about
    // the one thing an enum is: the list.
    let categories = crate::engine::declared_categories(dtype);
    if categories.is_empty() {
        format!("{dtype:?}")
    } else {
        format!("one of {}", categories.join(", "))
    }
}

fn family_label(family: DtypeFamily) -> &'static str {
    match family {
        DtypeFamily::Numeric => "a number",
        DtypeFamily::String => "a string",
        DtypeFamily::Date => "a date",
        DtypeFamily::Boolean => "a boolean",
        DtypeFamily::List => "a list",
        DtypeFamily::Array => "an array",
        DtypeFamily::Struct => "a struct",
        DtypeFamily::Categorical => "categorical",
        DtypeFamily::Other => "an unrecognized type",
    }
}

fn expected_namespace_for(family: DtypeFamily) -> Option<&'static str> {
    match family {
        DtypeFamily::Date => Some("dt"),
        DtypeFamily::String => Some("str"),
        DtypeFamily::List => Some("list"),
        DtypeFamily::Array => Some("arr"),
        DtypeFamily::Struct => Some("struct"),
        DtypeFamily::Categorical => Some("cat"),
        _ => None,
    }
}

/// Best-effort: parse `receiver_text` with the real formula parser, compile it to a
/// Polars expression against `frame_id`'s schema, and resolve its output dtype via
/// a lazy schema probe (no rows are materialized). Returns `None` on any failure —
/// callers treat that as "receiver type unknown" and fall back to untyped filtering.
fn resolve_receiver_dtype(
    document: &Document,
    frame_id: &str,
    receiver_text: &str,
) -> Option<pl::DataType> {
    if receiver_text.trim().is_empty() {
        return None;
    }
    let expression = document
        .parse_formula_for_frame(frame_id, receiver_text)
        .ok()?;
    let polars_expression = expression.to_polars(document).ok()?;
    let plan = document
        .materialize_frame_lazy(frame_id, Layer::Data, &mut HashSet::new())
        .ok()?;
    let schema = plan
        .select([polars_expression.alias(PROBE_ALIAS)])
        .collect_schema()
        .ok()?;
    schema.get(PROBE_ALIAS).cloned()
}

fn receiver_description(document: &Document, frame: &FrameObject, receiver_text: &str) -> String {
    if let Ok(Expr::Column { column_id }) =
        document.parse_formula_for_frame(&frame.id, receiver_text)
        && let Some(column) = frame.columns.iter().find(|column| column.id == column_id)
    {
        return format!("`{}`", column.name);
    }
    "This expression".to_string()
}

// ---------------------------------------------------------------------------
// Catalog filtering
// ---------------------------------------------------------------------------

/// The namespace an entry belongs to, taken from its `id` prefix (`"root.sum" ->
/// "root"`, `"dt.year" -> "dt"`). This is metadata already on every catalog entry,
/// hand-curated or generated, so filtering never hardcodes a method list.
fn entry_namespace(entry: &FormulaFunction) -> &str {
    entry.id.split('.').next().unwrap_or("")
}

/// Coarse category -> dtype-family frame. Categories not listed here (Nulls,
/// Window, Aggregation, and anything a future generated catalog introduces) are
/// treated as universal, matching the "never hardcode individual methods" design.
fn numeric_only_category(category: &str) -> bool {
    matches!(category, "Numeric methods" | "Trigonometry" | "Rolling")
}

/// Generated receiver methods come from Polars' Rust signatures, which say
/// that they take an expression but not which dtype that expression must
/// produce. Showing all of them on every known dtype turns autocomplete into
/// a list of runtime errors. Keep the compact, genuinely dtype-agnostic set
/// here; type-specific generated methods graduate into their namespace or a
/// curated family as we can state their contract honestly.
fn generated_expr_is_universal(id: &str) -> bool {
    matches!(
        id,
        "expr.drop_nulls"
            | "expr.first"
            | "expr.first_non_null"
            | "expr.has_nulls"
            | "expr.head"
            | "expr.is_duplicated"
            | "expr.is_empty"
            | "expr.is_first_distinct"
            | "expr.is_last_distinct"
            | "expr.is_unique"
            | "expr.last"
            | "expr.n_unique"
            | "expr.reverse"
            | "expr.sort"
            | "expr.tail"
            | "expr.unique"
            | "expr.unique_counts"
            | "expr.value_counts"
    )
}

fn expr_entry_matches_family(entry: &FormulaFunction, family: Option<DtypeFamily>) -> bool {
    let Some(family) = family else {
        return true;
    };
    if entry.category == "Generated expression methods" {
        return generated_expr_is_universal(&entry.id)
            || matches!(family, DtypeFamily::Numeric)
                && !matches!(entry.id.as_str(), "expr.all" | "expr.any")
            || family == DtypeFamily::Boolean
                && matches!(entry.id.as_str(), "expr.all" | "expr.any");
    }
    if numeric_only_category(&entry.category)
        || matches!(entry.id.as_str(), "expr.show" | "expr.sum" | "expr.mean")
    {
        return family == DtypeFamily::Numeric;
    }
    true
}

/// Whether `entry` should be offered directly after a receiver of dtype `family`
/// (family `None` means "unknown" — permissive, show everything).
fn entry_matches_family(entry: &FormulaFunction, family: Option<DtypeFamily>) -> bool {
    match entry_namespace(entry) {
        "expr" => expr_entry_matches_family(entry, family),
        "dt" => matches!(family, None | Some(DtypeFamily::Date)),
        // Categorical labels are text for the one conversion whose result is
        // a date. The other string methods are intentionally still hidden:
        // their categorical dispatch has different physical semantics.
        "str" => {
            matches!(family, None | Some(DtypeFamily::String))
                || entry.id == "str.to_date" && family == Some(DtypeFamily::Categorical)
        }
        "list" => matches!(family, None | Some(DtypeFamily::List)),
        "arr" => matches!(family, None | Some(DtypeFamily::Array)),
        "struct" => matches!(family, None | Some(DtypeFamily::Struct)),
        "cat" => matches!(family, None | Some(DtypeFamily::Categorical)),
        _ => true,
    }
}

/// Whether an *explicitly typed* namespace (the user wrote `.dt.` or `.str.`)
/// matches the receiver's family. Unknown namespaces (future generated catalog)
/// are permissive rather than blocked.
fn namespace_matches_family(namespace: &str, family: DtypeFamily) -> bool {
    match namespace {
        "dt" => family == DtypeFamily::Date,
        "str" => matches!(family, DtypeFamily::String | DtypeFamily::Categorical),
        "list" => family == DtypeFamily::List,
        "arr" => family == DtypeFamily::Array,
        "struct" => family == DtypeFamily::Struct,
        "cat" => family == DtypeFamily::Categorical,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Suggestion construction
// ---------------------------------------------------------------------------

fn column_suggestion(column: &Column, quote: bool) -> Suggestion {
    let escaped = column.name.replace('`', "``");
    Suggestion {
        id: format!("column.{}", column.id),
        label: column.name.clone(),
        insert_text: if quote {
            format!("`{escaped}`")
        } else {
            escaped
        },
        kind: SuggestionKind::Column,
        detail: format!("{:?} column", column.data_type),
        score: 0,
        match_indices: Vec::new(),
    }
}

fn frame_suggestion(frame: &FrameObject, rows: Option<usize>) -> Suggestion {
    Suggestion {
        id: format!("frame.{}", frame.id),
        label: frame.name.clone(),
        insert_text: format!("{}.", crate::formula::ast::formula_name(&frame.name)),
        kind: SuggestionKind::Frame,
        detail: match rows {
            Some(rows) => format!("{rows} rows · {} columns", frame.columns.len()),
            None => format!("{} columns", frame.columns.len()),
        },
        score: 0,
        match_indices: Vec::new(),
    }
}

/// A column of another frame, offered whole: picking it writes both halves
/// of `` `Frame`.`Column` ``, because the qualified form is the only one
/// that resolves and nobody should have to know that in advance.
///
/// The row count is in the detail line rather than left to be discovered.
/// It is the thing that decides whether the reference is a value or a list,
/// and knowing it before typing is the difference between writing the
/// formula and writing it twice.
fn foreign_column_suggestion(frame: &FrameObject, column: &Column, rows: usize) -> Suggestion {
    let quoted = crate::formula::ast::formula_name;
    Suggestion {
        id: format!("foreignColumn.{}", column.id),
        label: format!("{}.{}", frame.name, column.name),
        insert_text: format!("{}.{}", quoted(&frame.name), quoted(&column.name)),
        kind: SuggestionKind::Column,
        detail: if rows == 1 {
            format!("{:?} value from {}", column.data_type, frame.name)
        } else {
            format!(
                "{:?}, a list of {rows} from {}",
                column.data_type, frame.name
            )
        },
        score: 0,
        match_indices: Vec::new(),
    }
}

/// The name a canvas object answers to in a formula: every container it
/// sits in, outermost first, then its own name — each segment backticked.
/// The same path [`Expr::render`] writes back, because a completion that
/// inserts anything else is teaching a syntax the document will not echo.
fn qualified_object_token(document: &Document, object_id: &str, name: &str) -> (String, String) {
    let mut path = vec![name.to_string()];
    let mut current = object_id;
    while let Some(container) = document.container_of(current) {
        path.push(container.name.clone());
        current = &container.id;
    }
    path.reverse();
    let label = path.join(".");
    let token = path
        .iter()
        .map(|segment| crate::formula::ast::formula_name(segment))
        .collect::<Vec<_>>()
        .join(".");
    (label, token)
}

fn value_suggestion(document: &Document, value: &ValueObject) -> Suggestion {
    let (label, insert_text) = qualified_object_token(document, &value.id, &value.name);
    Suggestion {
        id: format!("value.{}", value.id),
        label,
        insert_text,
        kind: SuggestionKind::Value,
        detail: format!("{:?} value", value.data_type),
        score: 0,
        match_indices: Vec::new(),
    }
}

fn result_suggestion(document: &Document, result: &crate::ResultObject) -> Suggestion {
    let (label, insert_text) = qualified_object_token(document, &result.id, &result.name);
    Suggestion {
        id: format!("result.{}", result.id),
        label,
        insert_text,
        kind: SuggestionKind::Value,
        detail: "computed result".into(),
        score: 0,
        match_indices: Vec::new(),
    }
}

/// A canvas list. The count is in the detail because it is the difference
/// between this and a value, and the reason it can only go where a list can.
fn series_suggestion(document: &Document, series: &SeriesObject) -> Suggestion {
    let (label, insert_text) = qualified_object_token(document, &series.id, &series.name);
    Suggestion {
        id: format!("series.{}", series.id),
        label,
        insert_text,
        kind: SuggestionKind::Value,
        detail: format!("{:?} list of {}", series.data_type, series.values.len()),
        score: 0,
        match_indices: Vec::new(),
    }
}

/// One line of a block, offered whole: picking it writes both halves of
/// `` `Block`.`line` ``, which is the only spelling that resolves from
/// outside the block.
fn block_line_suggestion(block: &crate::BlockObject, line: &crate::BlockLine) -> Suggestion {
    let quoted = crate::formula::ast::formula_name;
    Suggestion {
        id: format!("blockLine.{}", line.id),
        label: format!("{}.{}", block.name, line.name),
        insert_text: format!("{}.{}", quoted(&block.name), quoted(&line.name)),
        kind: SuggestionKind::Value,
        detail: format!("line of {}", block.name),
        score: 0,
        match_indices: Vec::new(),
    }
}

fn root_function_suggestion(entry: &FormulaFunction) -> Suggestion {
    Suggestion {
        id: entry.id.clone(),
        label: entry.name.clone(),
        insert_text: format!("{}(", entry.name),
        kind: SuggestionKind::RootFunction,
        detail: entry.description.clone(),
        score: 0,
        match_indices: Vec::new(),
    }
}

/// `already_typed_segments` is how many leading path segments (e.g. `["dt"]`) the
/// user already typed, so we only insert the remainder.
fn method_suggestion(entry: &FormulaFunction, already_typed_segments: usize) -> Suggestion {
    let segments: Vec<&str> = entry.name.trim_start_matches('.').split('.').collect();
    let start = already_typed_segments.min(segments.len());
    let insert_text = format!("{}(", segments[start..].join("."));
    Suggestion {
        id: entry.id.clone(),
        label: entry.name.clone(),
        insert_text,
        kind: SuggestionKind::Method,
        detail: entry.description.clone(),
        score: 0,
        match_indices: Vec::new(),
    }
}

fn namespace_suggestion(namespace: &str, family: DtypeFamily) -> Suggestion {
    Suggestion {
        id: format!("namespace.{namespace}"),
        label: format!(".{namespace}"),
        insert_text: format!("{namespace}."),
        kind: SuggestionKind::Namespace,
        detail: format!("{} methods", family_label(family)),
        score: 0,
        match_indices: Vec::new(),
    }
}

/// How many segments a label carries: `.sum` is 1, `.arr.sum` is 2.
///
/// A method on the receiver itself beats the same name reached through a
/// namespace. Dtype filtering settles most of those collisions before ranking
/// ever sees them, but not when the receiver's type could not be resolved:
/// there every namespace is on offer at once, and `.sum` is the better guess
/// than `.arr.sum` for someone who typed "sum".
fn label_depth(label: &str) -> usize {
    label.trim_start_matches('.').split('.').count()
}

/// How directly a label answers what was typed, before fuzziness is weighed:
/// a name typed in full beats one that merely starts with it, which beats a
/// match scattered through the middle.
///
/// Fuzzy score alone does not order these the way a person expects. Skim scores
/// `.cum_sum` above `.sum` for "sum" — both match the three letters after a
/// word boundary and the longer label collects more bonuses — so Tab on a
/// finished word inserted something the user had not asked for.
///
/// Compared on the last segment, so `.arr.sum` counts as typed in full too;
/// `label_depth` is what then puts the receiver's own `.sum` above it.
fn match_tier(label: &str, partial: &str) -> u8 {
    let segment = label.rsplit('.').next().unwrap_or(label).to_lowercase();
    let partial = partial.to_lowercase();
    if segment == partial {
        0
    } else if segment.starts_with(&partial) {
        1
    } else {
        2
    }
}

/// Fuzzy-rank (SkimMatcherV2) against `partial`, or alphabetize when nothing has
/// been typed yet. Entries that don't match `partial` at all are dropped.
fn rank(items: Vec<Suggestion>, partial: &str) -> Vec<Suggestion> {
    if partial.is_empty() {
        let mut items = items;
        items.sort_by(|a, b| {
            label_depth(&a.label)
                .cmp(&label_depth(&b.label))
                .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
        });
        return items;
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<Suggestion> = items
        .into_iter()
        .filter_map(|mut item| {
            let (score, indices) = matcher.fuzzy_indices(&item.label, partial)?;
            item.score = score;
            item.match_indices = indices.into_iter().map(|index| index as u32).collect();
            Some(item)
        })
        .collect();
    scored.sort_by(|a, b| {
        match_tier(&a.label, partial)
            .cmp(&match_tier(&b.label, partial))
            .then_with(|| b.score.cmp(&a.score))
            .then_with(|| label_depth(&a.label).cmp(&label_depth(&b.label)))
            .then_with(|| a.label.cmp(&b.label))
    });
    scored
}

// ---------------------------------------------------------------------------
// Context handlers
// ---------------------------------------------------------------------------

/// Everything on the canvas a formula here may name, as fully backticked
/// tokens: values and lists (qualified by the containers they sit in), and
/// every column of every frame holding a snapshot — which is exactly the
/// set a formula here is allowed to name. Offering them is most of what
/// makes the feature findable: the syntax is discoverable by looking at
/// the list rather than by being told about it.
fn canvas_suggestions(document: &Document, frame: &FrameObject) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = document
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Value(value) => Some(value_suggestion(document, value)),
            DataObject::Result(result) => Some(result_suggestion(document, result)),
            DataObject::Series(series) => Some(series_suggestion(document, series)),
            _ => None,
        })
        .collect();
    for object in &document.objects {
        if let DataObject::Block(block) = object {
            suggestions.extend(
                block
                    .lines
                    .iter()
                    // Blank and comment lines answer to no name, so there is
                    // nothing to offer and nothing that would resolve.
                    .filter(|line| !line.name.is_empty())
                    .map(|line| block_line_suggestion(block, line)),
            );
        }
    }
    for object in &document.objects {
        let DataObject::Frame(other) = object else {
            continue;
        };
        let Some(materialization) = &other.materialization else {
            continue;
        };
        if other.id == frame.id {
            continue;
        }
        suggestions.push(frame_suggestion(
            other,
            Some(materialization.artifact.row_count),
        ));
        suggestions.extend(other.columns.iter().map(|column| {
            foreign_column_suggestion(other, column, materialization.artifact.row_count)
        }));
    }
    suggestions
}

fn complete_root(document: &Document, frame: &FrameObject, partial: &str) -> CompletionResult {
    let mut suggestions: Vec<Suggestion> = frame
        .columns
        .iter()
        .map(|column| column_suggestion(column, true))
        .collect();
    suggestions.push(frame_suggestion(frame, None));
    suggestions.extend(canvas_suggestions(document, frame));
    suggestions.extend(
        crate::formula_function_catalog()
            .iter()
            .filter(|entry| entry_namespace(entry) == "root")
            .map(root_function_suggestion),
    );
    CompletionResult {
        replace_start: 0,
        receiver_dtype: None,
        namespace: None,
        suggestions: rank(suggestions, partial),
        note: None,
        active_function_id: None,
        active_argument: None,
    }
}

fn complete_backtick(document: &Document, frame: &FrameObject, partial: &str) -> CompletionResult {
    let mut suggestions: Vec<Suggestion> = frame
        .columns
        .iter()
        .map(|column| column_suggestion(column, true))
        .collect();
    suggestions.push(frame_suggestion(frame, None));
    suggestions.extend(canvas_suggestions(document, frame));
    // Every token above is fully backticked; the user already typed the
    // opening one, so it is dropped from what gets inserted.
    for suggestion in &mut suggestions {
        if let Some(rest) = suggestion.insert_text.strip_prefix('`') {
            suggestion.insert_text = rest.to_string();
        }
    }
    CompletionResult {
        replace_start: 0,
        receiver_dtype: None,
        namespace: None,
        suggestions: rank(suggestions, partial),
        note: None,
        active_function_id: None,
        active_argument: None,
    }
}

fn trailing_reference(receiver_text: &str) -> Option<ReferenceName> {
    let tokens = tokenize(receiver_text).ok()?;
    match tokens
        .iter()
        .rev()
        .find(|token| !matches!(token, Token::End))?
    {
        Token::Identifier(FormulaReference::Unqualified(name)) => Some(name.clone()),
        _ => None,
    }
}

/**
 * A frame name is a namespace only when it resolves to one readable frame.
 * Live foreign frames cannot be referenced by formulas, so advertising their
 * members here would make completion write an expression the parser rejects.
 */
fn frame_members_after_dot(
    document: &Document,
    current: &FrameObject,
    receiver_text: &str,
    partial: &str,
) -> Option<CompletionResult> {
    let reference = trailing_reference(receiver_text)?;
    let matches: Vec<&FrameObject> = document
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Frame(frame)
                if reference_matches(&frame.name, &reference)
                    && (frame.id == current.id || frame.materialization.is_some()) =>
            {
                Some(frame)
            }
            _ => None,
        })
        .collect();
    if matches.len() > 1 {
        return Some(CompletionResult {
            replace_start: 0,
            receiver_dtype: None,
            namespace: None,
            suggestions: Vec::new(),
            note: Some("More than one readable frame has that name.".into()),
            active_function_id: None,
            active_argument: None,
        });
    }
    let frame = matches.first()?;
    Some(CompletionResult {
        replace_start: 0,
        receiver_dtype: None,
        namespace: Some(frame.name.clone()),
        suggestions: rank(
            frame
                .columns
                .iter()
                .map(|column| column_suggestion(column, true))
                .collect(),
            partial,
        ),
        note: None,
        active_function_id: None,
        active_argument: None,
    })
}

fn complete_after_dot(
    document: &Document,
    frame: &FrameObject,
    frame_id: &str,
    receiver_text: &str,
    path: &[String],
    partial: &str,
) -> CompletionResult {
    if path.is_empty() && receiver_text.eq_ignore_ascii_case("frame") {
        let suggestions = crate::formula_function_catalog()
            .into_iter()
            .find(|entry| entry.id == "root.frame_len")
            .map(|entry| {
                let mut suggestion = root_function_suggestion(&entry);
                suggestion.insert_text = "len(".into();
                suggestion
            })
            .into_iter()
            .collect();
        return CompletionResult {
            replace_start: 0,
            receiver_dtype: None,
            namespace: Some("frame".into()),
            suggestions: rank(suggestions, partial),
            note: None,
            active_function_id: None,
            active_argument: None,
        };
    }
    if path.is_empty()
        && let Some(result) = frame_members_after_dot(document, frame, receiver_text, partial)
    {
        return result;
    }
    let receiver_dtype = resolve_receiver_dtype(document, frame_id, receiver_text);
    let family = receiver_dtype.as_ref().map(dtype_family);
    let dtype_label = receiver_dtype.as_ref().map(describe_dtype);
    let catalog = crate::formula_function_catalog();

    if let Some(namespace) = path.last() {
        let candidates: Vec<&FormulaFunction> = catalog
            .iter()
            .filter(|entry| entry_namespace(entry) == namespace.as_str())
            .filter(|entry| entry_matches_family(entry, family))
            .collect();

        if let Some(family) = family
            && !namespace_matches_family(namespace, family)
        {
            let receiver_desc = receiver_description(document, frame, receiver_text);
            let note = match expected_namespace_for(family) {
                Some(expected) => format!(
                    "{receiver_desc} is {} — try .{expected}.",
                    family_label(family)
                ),
                None => format!(
                    "{receiver_desc} is {} and has no .{namespace} namespace.",
                    family_label(family)
                ),
            };
            return CompletionResult {
                replace_start: 0,
                receiver_dtype: dtype_label,
                namespace: Some(namespace.clone()),
                suggestions: Vec::new(),
                note: Some(note),
                active_function_id: None,
                active_argument: None,
            };
        }

        let suggestions = candidates
            .into_iter()
            .map(|entry| method_suggestion(entry, path.len()))
            .collect();
        return CompletionResult {
            replace_start: 0,
            receiver_dtype: dtype_label,
            namespace: Some(namespace.clone()),
            suggestions: rank(suggestions, partial),
            note: None,
            active_function_id: None,
            active_argument: None,
        };
    }

    // The first dot stays small: receiver methods plus one entry into the
    // receiver's typed namespace. Expanding every `.str.*` or `.dt.*` method
    // here made a string look as if it had hundreds of direct methods and put
    // the common null/count operations below a wall of specialist functions.
    let mut suggestions: Vec<Suggestion> = catalog
        .iter()
        .filter(|entry| entry_namespace(entry) == "expr")
        .filter(|entry| entry_matches_family(entry, family))
        .map(|entry| method_suggestion(entry, 0))
        .collect();
    if let Some(family) = family
        && let Some(namespace) = expected_namespace_for(family)
    {
        suggestions.push(namespace_suggestion(namespace, family));
    }
    CompletionResult {
        replace_start: 0,
        receiver_dtype: dtype_label,
        namespace: None,
        suggestions: rank(suggestions, partial),
        note: None,
        active_function_id: None,
        active_argument: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContainerObject, DataType, Document, FrameObject, Store, ValueObject, id};

    fn demo_store() -> Store {
        Store::new(Document::demo())
    }

    fn first_frame_id(store: &Store) -> String {
        store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .expect("demo document has a frame")
    }

    fn date_column(frame: &FrameObject) -> Option<&Column> {
        frame
            .columns
            .iter()
            .find(|column| column.data_type == DataType::Date)
    }

    fn numeric_column(frame: &FrameObject) -> Option<&Column> {
        frame.columns.iter().find(|column| {
            matches!(
                column.data_type,
                DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
            )
        })
    }

    fn string_column(frame: &FrameObject) -> Option<&Column> {
        frame
            .columns
            .iter()
            .find(|column| column.data_type == DataType::String)
    }

    fn categorical_column(frame: &FrameObject) -> Option<&Column> {
        frame
            .columns
            .iter()
            .find(|column| column.data_type == DataType::Categorical)
    }

    fn frame_of(store: &Store, frame_id: &str) -> FrameObject {
        store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.clone()),
                _ => None,
            })
            .expect("frame is in the document")
    }

    fn typed_columns_store() -> (Store, String) {
        let mut store = Store::new(Document {
            id: id(),
            name: "Completion types".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        });
        store
            .apply(crate::Operation::AddFrame {
                name: "Types".into(),
                grid: vec![
                    vec![
                        "Text".into(),
                        "Category".into(),
                        "Count".into(),
                        "Flag".into(),
                    ],
                    vec!["alpha".into(), "A".into(), "1".into(), "true".into()],
                    vec!["beta".into(), "B".into(), "2".into(), "false".into()],
                ],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let frame_id = first_frame_id(&store);
        let frame = frame_of(&store, &frame_id);
        let category_id = frame
            .columns
            .iter()
            .find(|column| column.name == "Category")
            .unwrap()
            .id
            .clone();
        store
            .apply(crate::Operation::SetColumnCategories {
                frame_id: frame_id.clone(),
                column_id: category_id,
                categories: vec!["A".into(), "B".into()],
            })
            .unwrap();
        (store, frame_id)
    }

    fn method_entry(id: &str, name: &str, category: &str) -> FormulaFunction {
        FormulaFunction {
            id: id.into(),
            name: name.into(),
            aliases: Vec::new(),
            category: category.into(),
            signature: format!("{name}()"),
            description: String::new(),
            minimum_arguments: 0,
            maximum_arguments: 0,
            return_type: "dynamic".into(),
            null_behavior: "propagates null".into(),
            arguments: Vec::new(),
        }
    }

    // -- cursor scanner -----------------------------------------------------

    #[test]
    fn scan_root_at_start_of_formula() {
        let chars: Vec<char> = "".chars().collect();
        assert_eq!(
            scan_cursor_context(&chars, 0),
            CursorContext::Root {
                partial: String::new()
            }
        );
    }

    #[test]
    fn scan_root_after_operator() {
        let chars: Vec<char> = "1 + su".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::Root {
                partial: "su".into()
            }
        );
    }

    #[test]
    fn scan_root_after_open_paren() {
        let chars: Vec<char> = "sum_horizontal(".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::Root {
                partial: String::new()
            }
        );
    }

    #[test]
    fn scan_backtick_partial() {
        let chars: Vec<char> = "`Post".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::Backtick {
                partial: "Post".into()
            }
        );
    }

    #[test]
    fn scan_backtick_closed_then_dot_is_after_dot() {
        let chars: Vec<char> = "`Col`.".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::AfterDot {
                receiver_text: "`Col`".into(),
                path: Vec::new(),
                partial: String::new(),
            }
        );
    }

    #[test]
    fn scan_namespace_in_progress() {
        let chars: Vec<char> = "`Col`.dt.y".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::AfterDot {
                receiver_text: "`Col`".into(),
                path: vec!["dt".into()],
                partial: "y".into(),
            }
        );
    }

    #[test]
    fn scan_chained_call_boundary() {
        let chars: Vec<char> = "`Col`.dt.month().".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::AfterDot {
                receiver_text: "`Col`.dt.month()".into(),
                path: Vec::new(),
                partial: String::new(),
            }
        );
    }

    #[test]
    fn scan_nested_parens_degrades_to_root() {
        let chars: Vec<char> = "sum_horizontal(`A`, `B`".chars().collect();
        let context = scan_cursor_context(&chars, chars.len());
        assert_eq!(
            context,
            CursorContext::Root {
                partial: String::new()
            }
        );
    }

    // -- end-to-end completion ----------------------------------------------

    #[test]
    fn root_context_offers_columns_and_root_functions() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let result = store.complete_formula(&frame_id, "", 0);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.kind == SuggestionKind::Column)
        );
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.kind == SuggestionKind::RootFunction)
        );
    }

    #[test]
    fn root_context_offers_the_frame_as_a_namespace() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = frame_of(&store, &frame_id);
        let result = store.complete_formula(&frame_id, &frame.name, frame.name.chars().count());
        let suggestion = result
            .suggestions
            .iter()
            .find(|suggestion| suggestion.kind == SuggestionKind::Frame)
            .expect("the current frame is independently selectable");
        assert_eq!(suggestion.label, frame.name);
        assert_eq!(
            suggestion.insert_text,
            format!("{}.", crate::formula::ast::formula_name(&frame.name))
        );
    }

    #[test]
    fn current_frame_namespace_completes_its_row_count() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let formula = "frame.";
        let result = store.complete_formula(&frame_id, formula, formula.chars().count());
        assert_eq!(result.namespace.as_deref(), Some("frame"));
        assert_eq!(result.suggestions.len(), 1);
        assert_eq!(result.suggestions[0].label, "frame.len");
        assert_eq!(result.suggestions[0].insert_text, "len(");
    }

    #[test]
    fn frame_dot_completes_only_that_frames_columns() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = frame_of(&store, &frame_id);
        let column = frame.columns.first().expect("demo frame has a column");
        let partial: String = column.name.chars().take(2).collect();
        let formula = format!(
            "{}.{}",
            crate::formula::ast::formula_name(&frame.name),
            partial
        );
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());

        assert_eq!(result.namespace.as_deref(), Some(frame.name.as_str()));
        assert!(
            result
                .suggestions
                .iter()
                .all(|item| item.kind == SuggestionKind::Column)
        );
        assert!(
            result
                .suggestions
                .iter()
                .any(|item| item.label == column.name)
        );
        assert_eq!(
            result.replace_start,
            formula.chars().count() - partial.chars().count()
        );
    }

    #[test]
    fn completion_reports_the_call_and_argument_containing_the_cursor() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let formula = "coalesce(`Name`, `Amount`.round(2";

        let nested = store.complete_formula(&frame_id, formula, formula.chars().count());
        assert_eq!(nested.active_function_id.as_deref(), Some("expr.round"));
        assert_eq!(nested.active_argument, Some(0));

        let outer_cursor = formula.find(",").unwrap() + 1;
        let outer = store.complete_formula(&frame_id, formula, outer_cursor);
        assert_eq!(outer.active_function_id.as_deref(), Some("root.coalesce"));
        assert_eq!(outer.active_argument, Some(1));
    }

    #[test]
    fn parameter_scanning_ignores_commas_in_nested_lists_and_strings() {
        let formula: Vec<char> = "coalesce([1, 2], \"a,b\", ".chars().collect();
        assert_eq!(
            active_call_at_cursor(&formula, formula.len()),
            Some(("coalesce".into(), 2))
        );
    }

    #[test]
    fn date_receiver_offers_the_dt_namespace_after_bare_dot() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.clone()),
                _ => None,
            })
            .unwrap();
        let Some(column) = date_column(&frame) else {
            return; // demo document has no date column in this configuration
        };
        let formula = format!("`{}`.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert_eq!(result.receiver_dtype.as_deref(), Some("Date"));
        assert!(result.suggestions.iter().any(|s| s.label == ".dt"));
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.label.starts_with(".dt."))
        );
    }

    #[test]
    fn date_receiver_str_namespace_is_empty_with_note() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.clone()),
                _ => None,
            })
            .unwrap();
        let Some(column) = date_column(&frame) else {
            return;
        };
        let formula = format!("`{}`.str.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert!(result.suggestions.is_empty());
        assert!(result.note.is_some());
        assert!(result.note.unwrap().contains("try .dt."));
    }

    #[test]
    fn date_receiver_dt_namespace_lists_dt_methods() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.clone()),
                _ => None,
            })
            .unwrap();
        let Some(column) = date_column(&frame) else {
            return;
        };
        let formula = format!("`{}`.dt.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert!(!result.suggestions.is_empty());
        assert!(result.note.is_none());
        assert!(
            result
                .suggestions
                .iter()
                .all(|s| s.label.starts_with(".dt."))
        );
    }

    #[test]
    fn chained_dt_month_then_dot_offers_numeric_methods() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = store
            .view()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.clone()),
                _ => None,
            })
            .unwrap();
        let Some(column) = date_column(&frame) else {
            return;
        };
        let formula = format!("`{}`.dt.month().", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert_eq!(result.receiver_dtype.as_deref(), Some("Int32"));
        assert!(result.suggestions.iter().any(|s| s.label == ".abs"));
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.label.starts_with(".dt."))
        );
    }

    #[test]
    fn backtick_context_lists_columns_and_values() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let formula = "`".to_string();
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert!(!result.suggestions.is_empty());
        // Names close the backtick the user opened; frame namespaces also
        // leave the member dot ready for the next few typed characters.
        assert!(
            result
                .suggestions
                .iter()
                .all(|s| s.insert_text.ends_with('`') || s.insert_text.ends_with("`."))
        );
        assert_eq!(result.replace_start, 1);
    }

    #[test]
    fn unparseable_receiver_degrades_gracefully_never_panics() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let formula = "`Nonexistent Column Name`.".to_string();
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert!(result.receiver_dtype.is_none());
        // Degraded (untyped) suggestions still come back instead of an error.
        assert!(!result.suggestions.is_empty());
    }

    #[test]
    fn ranking_prefers_closer_prefix_matches() {
        let entries = [
            FormulaFunction {
                id: "root.sum_horizontal".into(),
                name: "sum_horizontal".into(),
                aliases: Vec::new(),
                category: "Horizontal".into(),
                signature: "sum_horizontal([expressions])".into(),
                description: String::new(),
                minimum_arguments: 1,
                maximum_arguments: 64,
                return_type: "number".into(),
                null_behavior: "propagates null".into(),
                arguments: Vec::new(),
            },
            FormulaFunction {
                id: "root.mean_horizontal".into(),
                name: "mean_horizontal".into(),
                aliases: Vec::new(),
                category: "Horizontal".into(),
                signature: "mean_horizontal([expressions])".into(),
                description: String::new(),
                minimum_arguments: 1,
                maximum_arguments: 64,
                return_type: "number".into(),
                null_behavior: "propagates null".into(),
                arguments: Vec::new(),
            },
        ];
        let suggestions: Vec<Suggestion> = entries.iter().map(root_function_suggestion).collect();
        let ranked = rank(suggestions, "sum");
        assert_eq!(ranked[0].label, "sum_horizontal");
    }

    #[test]
    fn plain_method_outranks_the_same_name_in_a_namespace() {
        let entries = [
            method_entry("arr.sum", ".arr.sum", "Generated array namespace"),
            method_entry("list.sum", ".list.sum", "Generated list namespace"),
            method_entry("expr.sum", ".sum", "Aggregation"),
        ];
        let suggestions: Vec<Suggestion> = entries
            .iter()
            .map(|entry| method_suggestion(entry, 0))
            .collect();
        let ranked = rank(suggestions, "sum");
        assert_eq!(ranked[0].label, ".sum");
    }

    #[test]
    fn numeric_receiver_hides_array_and_struct_namespaces() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = frame_of(&store, &frame_id);
        let Some(column) = numeric_column(&frame) else {
            return; // demo document has no numeric column in this configuration
        };
        let formula = format!("`{}`.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert!(result.receiver_dtype.is_some());
        assert!(result.suggestions.iter().any(|s| s.label == ".sum"));
        assert!(result.suggestions.iter().any(|s| s.label == ".filter"));
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.label.starts_with(".arr.") || s.label.starts_with(".struct."))
        );
    }

    #[test]
    fn string_receiver_stays_small_until_str_namespace_is_opened() {
        let (store, frame_id) = typed_columns_store();
        let frame = frame_of(&store, &frame_id);
        let column = string_column(&frame).unwrap();
        let formula = format!("`{}`.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());

        assert!(result.suggestions.iter().any(|s| s.label == ".str"));
        assert!(result.suggestions.iter().any(|s| s.label == ".is_null"));
        assert!(!result.suggestions.iter().any(|s| {
            s.label.starts_with(".str.")
                || matches!(s.label.as_str(), ".abs" | ".sum" | ".mean" | ".show")
                || s.label.starts_with(".bitwise_")
        }));

        let namespaced = format!("`{}`.str.", column.name);
        let result = store.complete_formula(&frame_id, &namespaced, namespaced.chars().count());
        assert!(!result.suggestions.is_empty());
        assert!(
            result
                .suggestions
                .iter()
                .all(|suggestion| suggestion.label.starts_with(".str."))
        );
        assert!(result.suggestions.iter().any(|s| s.label == ".str.to_date"));
    }

    #[test]
    fn categorical_receiver_uses_cat_not_string_or_numeric_methods() {
        let (store, frame_id) = typed_columns_store();
        let frame = frame_of(&store, &frame_id);
        let column = categorical_column(&frame).unwrap();
        let formula = format!("`{}`.", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());

        assert!(result.suggestions.iter().any(|s| s.label == ".cat"));
        assert!(!result.suggestions.iter().any(|s| {
            s.label == ".str"
                || s.label.starts_with(".str.")
                || matches!(s.label.as_str(), ".abs" | ".sum" | ".mean" | ".show")
        }));

        let dates = format!("`{}`.str.", column.name);
        let date_result = store.complete_formula(&frame_id, &dates, dates.chars().count());
        assert_eq!(
            date_result
                .suggestions
                .iter()
                .map(|suggestion| suggestion.label.as_str())
                .collect::<Vec<_>>(),
            vec![".str.to_date"]
        );
    }

    #[test]
    fn integer_and_boolean_receivers_do_not_share_each_others_methods() {
        let (store, frame_id) = typed_columns_store();
        let frame = frame_of(&store, &frame_id);
        let integer = frame
            .columns
            .iter()
            .find(|column| column.data_type == DataType::Integer)
            .unwrap();
        let boolean = frame
            .columns
            .iter()
            .find(|column| column.data_type == DataType::Boolean)
            .unwrap();

        let integer_formula = format!("`{}`.", integer.name);
        let integer_result =
            store.complete_formula(&frame_id, &integer_formula, integer_formula.chars().count());
        assert!(integer_result.suggestions.iter().any(|s| s.label == ".abs"));
        assert!(!integer_result.suggestions.iter().any(|s| s.label == ".str"));

        let boolean_formula = format!("`{}`.", boolean.name);
        let boolean_result =
            store.complete_formula(&frame_id, &boolean_formula, boolean_formula.chars().count());
        assert!(
            boolean_result
                .suggestions
                .iter()
                .any(|s| s.label == ".is_null")
        );
        assert!(!boolean_result.suggestions.iter().any(|s| {
            matches!(
                s.label.as_str(),
                ".abs" | ".sum" | ".mean" | ".show" | ".str"
            )
        }));
    }

    /// Summing a numeric column is `.sum()`, not the array namespace's
    /// `.arr.sum()` — which does not compile against a number and is the
    /// first thing Tab used to insert.
    #[test]
    fn sum_after_a_numeric_column_completes_to_the_aggregate() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let frame = frame_of(&store, &frame_id);
        let Some(column) = numeric_column(&frame) else {
            return;
        };
        let formula = format!("`{}`.sum", column.name);
        let result = store.complete_formula(&frame_id, &formula, formula.chars().count());
        assert_eq!(
            result.suggestions.first().map(|s| s.label.as_str()),
            Some(".sum")
        );
        assert_eq!(
            result.suggestions.first().map(|s| s.insert_text.as_str()),
            Some("sum(")
        );
    }

    /// The demo's one assumption is a line of a block, which is where a
    /// constant lives now. Picking it has to write both halves, because
    /// `` `Tax rate` `` on its own resolves to nothing outside that block.
    #[test]
    fn root_context_offers_values_as_fully_backticked_tokens() {
        let store = demo_store();
        let frame_id = first_frame_id(&store);
        let result = store.complete_formula(&frame_id, "Tax", 3);
        let value = result
            .suggestions
            .iter()
            .find(|s| s.kind == SuggestionKind::Value)
            .expect("root context offers the demo assumption");
        assert_eq!(value.label, "Assumptions.Tax rate");
        assert_eq!(value.insert_text, "`Assumptions`.`Tax rate`");
    }

    #[test]
    fn container_member_completes_to_its_qualified_path() {
        let mut document = Document::demo();
        let value_id = id();
        document.objects.push(DataObject::Value(ValueObject {
            id: value_id.clone(),
            name: "My times".into(),
            raw: "10".into(),
            data_type: DataType::Number,
        }));
        document
            .objects
            .push(DataObject::Container(ContainerObject {
                id: id(),
                name: "MyContainer".into(),
                member_ids: vec![value_id.clone()],
            }));
        let store = Store::new(document);
        let frame_id = first_frame_id(&store);

        // Root context: the whole path, every segment backticked.
        let result = store.complete_formula(&frame_id, "My", 2);
        let member = result
            .suggestions
            .iter()
            .find(|s| s.id == format!("value.{value_id}"))
            .expect("container member is offered from root context");
        assert_eq!(member.label, "MyContainer.My times");
        assert_eq!(member.insert_text, "`MyContainer`.`My times`");

        // Backtick context: same token, minus the backtick already typed.
        let result = store.complete_formula(&frame_id, "`My", 3);
        let member = result
            .suggestions
            .iter()
            .find(|s| s.id == format!("value.{value_id}"))
            .expect("container member is offered from backtick context");
        assert_eq!(member.insert_text, "MyContainer`.`My times`");
    }

    #[test]
    fn empty_partial_ranks_alphabetically() {
        let suggestions = vec![
            Suggestion {
                id: "b".into(),
                label: "banana".into(),
                insert_text: "banana(".into(),
                kind: SuggestionKind::RootFunction,
                detail: String::new(),
                score: 0,
                match_indices: Vec::new(),
            },
            Suggestion {
                id: "a".into(),
                label: "apple".into(),
                insert_text: "apple(".into(),
                kind: SuggestionKind::RootFunction,
                detail: String::new(),
                score: 0,
                match_indices: Vec::new(),
            },
        ];
        let ranked = rank(suggestions, "");
        assert_eq!(ranked[0].label, "apple");
        assert_eq!(ranked[1].label, "banana");
    }
}
