use crate::formula::ast::{FormulaArgument, FormulaFunction};

macro_rules! formula_function {
    ($id:literal, $name:literal, [$($alias:literal),* $(,)?], $category:literal, $signature:literal, $description:literal, $minimum:literal, $maximum:literal) => {
        FormulaFunctionDefinition {
            id: $id,
            name: $name,
            aliases: &[$($alias),*],
            category: $category,
            signature: $signature,
            description: $description,
            minimum_arguments: $minimum,
            maximum_arguments: $maximum,
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FormulaFunctionDefinition {
    id: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    category: &'static str,
    signature: &'static str,
    description: &'static str,
    minimum_arguments: usize,
    maximum_arguments: usize,
}

pub(crate) const POLARS_FORMULA_FUNCTIONS: &[FormulaFunctionDefinition] = &[
    formula_function!(
        "root.sum_horizontal",
        "sum_horizontal",
        ["row sum"],
        "Horizontal",
        "sum_horizontal([expressions], ignore_nulls=True)",
        "Sum expressions across each row.",
        1,
        64
    ),
    formula_function!(
        "root.mean_horizontal",
        "mean_horizontal",
        ["row mean"],
        "Horizontal",
        "mean_horizontal([expressions], ignore_nulls=True)",
        "Average expressions across each row.",
        1,
        64
    ),
    formula_function!(
        "root.min_horizontal",
        "min_horizontal",
        ["row minimum"],
        "Horizontal",
        "min_horizontal([expressions])",
        "Find the row-wise minimum.",
        1,
        64
    ),
    formula_function!(
        "root.max_horizontal",
        "max_horizontal",
        ["row maximum"],
        "Horizontal",
        "max_horizontal([expressions])",
        "Find the row-wise maximum.",
        1,
        64
    ),
    formula_function!(
        "root.coalesce",
        "coalesce",
        ["first non-null"],
        "Nulls",
        "coalesce([expressions])",
        "Return the first non-null expression.",
        1,
        64
    ),
    formula_function!(
        "root.sequence",
        "sequence",
        ["range", "number series", "date range", "Excel SEQUENCE"],
        "Generators",
        "sequence(start, stop=None, step=1, periods=None)",
        "Generate numbers or dates up to, but not including, stop. For a date fill, periods=frame.len() makes exactly one date per row.",
        1,
        3
    ),
    formula_function!(
        "root.recur",
        "recur",
        [
            "recurrence",
            "calculate down rows",
            "recursive column",
            "carry forward"
        ],
        "Row order",
        "recur(first, next, restart_by=[columns])",
        "Start a column with one value, then calculate each later row from previous() and the current row.",
        2,
        2
    ),
    formula_function!(
        "root.previous",
        "previous",
        ["previous result", "prior result", "row above"],
        "Row order",
        "previous()",
        "Read the result calculated for the preceding row inside recur(...).",
        0,
        0
    ),
    formula_function!(
        "root.frame_len",
        "frame.len",
        [
            "row count",
            "frame length",
            "rows",
            "n_rows",
            "frame.n_rows"
        ],
        "Generators",
        "frame.len()",
        "Count rows at the current point in this frame's transformation chain.",
        0,
        0
    ),
    formula_function!(
        "root.format",
        "format",
        ["concat", "concatenate", "join text", "text", "&"],
        "String namespace",
        "format(\"Q{}\", value)",
        "Fill the ‘{}’ in a pattern with values, written as this document writes them.",
        1,
        64
    ),
    formula_function!(
        "expr.cast",
        ".cast",
        [
            "convert",
            "type",
            "to text",
            "to number",
            "tonumber",
            "value"
        ],
        "Conversion",
        ".cast(\"string\")",
        "Convert to another type: \"string\", \"integer\", \"number\", \"date\" or \"boolean\".",
        1,
        1
    ),
    formula_function!(
        "expr.show",
        ".show",
        [
            "money",
            "currency",
            "dollars",
            "percent",
            "percentage",
            "plain",
            "format as",
            "display as"
        ],
        "Conversion",
        ".show(\"percent\")",
        "Write this number as \"money\", \"percent\", or \"plain\". Changes how the \
         answer reads, never what it is — and overrides whatever the arithmetic \
         worked out.",
        1,
        1
    ),
    formula_function!(
        "root.when",
        "when",
        ["conditional"],
        "Conditional",
        "when(condition).then(value).otherwise(value)",
        "Build a Polars conditional expression.",
        1,
        1
    ),
    formula_function!(
        "root.date",
        "date",
        ["make date"],
        "Dates",
        "date(year, month, day)",
        "Construct a Date expression.",
        3,
        3
    ),
    formula_function!(
        "root.today",
        "today",
        ["current date"],
        "Dates",
        "today()",
        "Today's date, read when the frame is read.",
        0,
        0
    ),
    formula_function!(
        "root.now",
        "now",
        ["current time"],
        "Dates",
        "now()",
        "The current date and time, read when the frame is read.",
        0,
        0
    ),
    formula_function!(
        "expr.is_between",
        ".is_between",
        ["range", "within"],
        "Comparison",
        ".is_between(lower, upper, closed=\"both\")",
        "Test whether values fall in a range, ends included.",
        2,
        2
    ),
    formula_function!(
        "expr.is_in",
        ".is_in",
        ["one of", "member of"],
        "Comparison",
        ".is_in([values])",
        "Test whether values appear in a list.",
        1,
        1
    ),
    formula_function!(
        "expr.filter",
        ".filter",
        [
            "sumif",
            "sumifs",
            "countif",
            "countifs",
            "averageif",
            "averageifs",
            "maxifs",
            "minifs",
            "conditional aggregate",
            "where",
        ],
        "Conditional",
        ".filter(predicate)",
        "Keep values whose matching rows meet a condition; chain .sum(), .mean(), or another aggregate.",
        1,
        1
    ),
    formula_function!(
        "expr.abs",
        ".abs",
        ["absolute"],
        "Numeric methods",
        ".abs()",
        "Return absolute values.",
        0,
        0
    ),
    formula_function!(
        "expr.sign",
        ".sign",
        ["signum"],
        "Numeric methods",
        ".sign()",
        "Return each value's sign.",
        0,
        0
    ),
    formula_function!(
        "expr.round",
        ".round",
        ["rounded"],
        "Numeric methods",
        ".round(decimals=0)",
        "Round numeric values.",
        0,
        1
    ),
    formula_function!(
        "expr.normalize",
        ".normalize",
        ["rescale", "scale", "heatmap"],
        "Numeric methods",
        ".normalize(low, high)",
        "Where each value sits between two numbers, from 0 to 1. With no arguments the column's own smallest and largest; with center= the given number lands at 0.5 and the two directions away from it get equal room.",
        0,
        2
    ),
    formula_function!(
        "expr.round_sig_figs",
        ".round_sig_figs",
        ["significant figures"],
        "Numeric methods",
        ".round_sig_figs(digits)",
        "Round to significant figures.",
        1,
        1
    ),
    formula_function!(
        "expr.truncate",
        ".truncate",
        ["trunc"],
        "Numeric methods",
        ".truncate(decimals=0)",
        "Truncate numeric values.",
        0,
        1
    ),
    formula_function!(
        "expr.floor",
        ".floor",
        [],
        "Numeric methods",
        ".floor()",
        "Round down.",
        0,
        0
    ),
    formula_function!(
        "expr.ceil",
        ".ceil",
        ["ceiling"],
        "Numeric methods",
        ".ceil()",
        "Round up.",
        0,
        0
    ),
    formula_function!(
        "expr.sqrt",
        ".sqrt",
        ["square root"],
        "Numeric methods",
        ".sqrt()",
        "Compute square roots.",
        0,
        0
    ),
    formula_function!(
        "expr.cbrt",
        ".cbrt",
        ["cube root"],
        "Numeric methods",
        ".cbrt()",
        "Compute cube roots.",
        0,
        0
    ),
    formula_function!(
        "expr.pow",
        ".pow",
        ["power"],
        "Numeric methods",
        ".pow(exponent)",
        "Raise values to a power.",
        1,
        1
    ),
    formula_function!(
        "expr.exp",
        ".exp",
        ["exponential"],
        "Logs",
        ".exp()",
        "Raise e to each value.",
        0,
        0
    ),
    formula_function!(
        "expr.log",
        ".log",
        ["ln", "logarithm"],
        "Logs",
        ".log(base=e)",
        "Compute logarithms.",
        0,
        1
    ),
    formula_function!(
        "expr.log1p",
        ".log1p",
        ["ln one plus"],
        "Logs",
        ".log1p()",
        "Compute ln(1+x).",
        0,
        0
    ),
    formula_function!(
        "expr.clip",
        ".clip",
        ["clamp"],
        "Numeric methods",
        ".clip(lower, upper)",
        "Clip values to bounds.",
        2,
        2
    ),
    formula_function!(
        "expr.clip_min",
        ".clip_min",
        ["at least"],
        "Numeric methods",
        ".clip_min(lower)",
        "Clip to a lower bound.",
        1,
        1
    ),
    formula_function!(
        "expr.clip_max",
        ".clip_max",
        ["at most"],
        "Numeric methods",
        ".clip_max(upper)",
        "Clip to an upper bound.",
        1,
        1
    ),
    formula_function!(
        "expr.floor_div",
        ".floor_div",
        ["floor division"],
        "Numeric methods",
        ".floor_div(divisor)",
        "Floor-divide values.",
        1,
        1
    ),
    formula_function!(
        "expr.sin",
        ".sin",
        ["sine"],
        "Trigonometry",
        ".sin()",
        "Compute sine.",
        0,
        0
    ),
    formula_function!(
        "expr.cos",
        ".cos",
        ["cosine"],
        "Trigonometry",
        ".cos()",
        "Compute cosine.",
        0,
        0
    ),
    formula_function!(
        "expr.tan",
        ".tan",
        ["tangent"],
        "Trigonometry",
        ".tan()",
        "Compute tangent.",
        0,
        0
    ),
    formula_function!(
        "expr.cot",
        ".cot",
        ["cotangent"],
        "Trigonometry",
        ".cot()",
        "Compute cotangent.",
        0,
        0
    ),
    formula_function!(
        "expr.arcsin",
        ".arcsin",
        ["inverse sine"],
        "Trigonometry",
        ".arcsin()",
        "Compute inverse sine.",
        0,
        0
    ),
    formula_function!(
        "expr.arccos",
        ".arccos",
        ["inverse cosine"],
        "Trigonometry",
        ".arccos()",
        "Compute inverse cosine.",
        0,
        0
    ),
    formula_function!(
        "expr.arctan",
        ".arctan",
        ["inverse tangent"],
        "Trigonometry",
        ".arctan()",
        "Compute inverse tangent.",
        0,
        0
    ),
    formula_function!(
        "expr.arctan2",
        ".arctan2",
        [],
        "Trigonometry",
        ".arctan2(x)",
        "Compute quadrant-aware inverse tangent.",
        1,
        1
    ),
    formula_function!(
        "expr.sinh",
        ".sinh",
        [],
        "Trigonometry",
        ".sinh()",
        "Compute hyperbolic sine.",
        0,
        0
    ),
    formula_function!(
        "expr.cosh",
        ".cosh",
        [],
        "Trigonometry",
        ".cosh()",
        "Compute hyperbolic cosine.",
        0,
        0
    ),
    formula_function!(
        "expr.tanh",
        ".tanh",
        [],
        "Trigonometry",
        ".tanh()",
        "Compute hyperbolic tangent.",
        0,
        0
    ),
    formula_function!(
        "expr.arcsinh",
        ".arcsinh",
        [],
        "Trigonometry",
        ".arcsinh()",
        "Compute inverse hyperbolic sine.",
        0,
        0
    ),
    formula_function!(
        "expr.arccosh",
        ".arccosh",
        [],
        "Trigonometry",
        ".arccosh()",
        "Compute inverse hyperbolic cosine.",
        0,
        0
    ),
    formula_function!(
        "expr.arctanh",
        ".arctanh",
        [],
        "Trigonometry",
        ".arctanh()",
        "Compute inverse hyperbolic tangent.",
        0,
        0
    ),
    formula_function!(
        "expr.degrees",
        ".degrees",
        ["to degrees"],
        "Trigonometry",
        ".degrees()",
        "Convert radians to degrees.",
        0,
        0
    ),
    formula_function!(
        "expr.radians",
        ".radians",
        ["to radians"],
        "Trigonometry",
        ".radians()",
        "Convert degrees to radians.",
        0,
        0
    ),
    formula_function!(
        "expr.is_null",
        ".is_null",
        ["is missing"],
        "Nulls",
        ".is_null()",
        "Test for null values.",
        0,
        0
    ),
    formula_function!(
        "expr.is_not_null",
        ".is_not_null",
        ["is present"],
        "Nulls",
        ".is_not_null()",
        "Test for non-null values.",
        0,
        0
    ),
    formula_function!(
        "expr.fill_null",
        ".fill_null",
        ["replace null"],
        "Nulls",
        ".fill_null(value)",
        "Replace null values.",
        1,
        1
    ),
    formula_function!(
        "expr.shift",
        ".shift",
        ["lag", "lead"],
        "Window",
        ".shift(periods)",
        "Shift values by row position.",
        1,
        1
    ),
    formula_function!(
        "expr.over",
        ".over",
        ["window", "partition"],
        "Window",
        ".over([columns])",
        "Evaluate over partition groups.",
        1,
        64
    ),
    formula_function!(
        "expr.sum",
        ".sum",
        ["total"],
        "Aggregation",
        ".sum()",
        "Sum a column expression.",
        0,
        0
    ),
    formula_function!(
        "expr.mean",
        ".mean",
        ["average"],
        "Aggregation",
        ".mean()",
        "Average a column expression.",
        0,
        0
    ),
    formula_function!(
        "expr.quantile",
        ".quantile",
        ["percentile", "quartile"],
        "Aggregation",
        ".quantile(fraction)",
        "Find a percentile using linear interpolation; 0.25 is the first quartile.",
        1,
        1
    ),
    formula_function!(
        "expr.min",
        ".min",
        ["minimum"],
        "Aggregation",
        ".min()",
        "Find a column minimum.",
        0,
        0
    ),
    formula_function!(
        "expr.max",
        ".max",
        ["maximum"],
        "Aggregation",
        ".max()",
        "Find a column maximum.",
        0,
        0
    ),
    formula_function!(
        "expr.count",
        ".count",
        [],
        "Aggregation",
        ".count()",
        "Count non-null values.",
        0,
        0
    ),
    formula_function!(
        "expr.len",
        ".len",
        ["length"],
        "Aggregation",
        ".len()",
        "Count values including nulls.",
        0,
        0
    ),
    formula_function!(
        "expr.null_count",
        ".null_count",
        [],
        "Aggregation",
        ".null_count()",
        "Count null values.",
        0,
        0
    ),
    formula_function!(
        "expr.rolling_mean",
        ".rolling_mean",
        ["moving average"],
        "Rolling",
        ".rolling_mean(window_size, min_periods=1, center=False)",
        "Compute a rolling mean.",
        1,
        1
    ),
    formula_function!(
        "expr.rolling_sum",
        ".rolling_sum",
        ["moving sum"],
        "Rolling",
        ".rolling_sum(window_size, min_periods=1, center=False)",
        "Compute a rolling sum.",
        1,
        1
    ),
    formula_function!(
        "expr.rolling_min",
        ".rolling_min",
        ["moving minimum"],
        "Rolling",
        ".rolling_min(window_size, min_periods=1, center=False)",
        "Compute a rolling minimum.",
        1,
        1
    ),
    formula_function!(
        "expr.rolling_max",
        ".rolling_max",
        ["moving maximum"],
        "Rolling",
        ".rolling_max(window_size, min_periods=1, center=False)",
        "Compute a rolling maximum.",
        1,
        1
    ),
    formula_function!(
        "dt.year",
        ".dt.year",
        [],
        "Date namespace",
        ".dt.year()",
        "Extract calendar year.",
        0,
        0
    ),
    formula_function!(
        "dt.iso_year",
        ".dt.iso_year",
        [],
        "Date namespace",
        ".dt.iso_year()",
        "Extract ISO year.",
        0,
        0
    ),
    formula_function!(
        "dt.quarter",
        ".dt.quarter",
        [],
        "Date namespace",
        ".dt.quarter()",
        "Extract quarter.",
        0,
        0
    ),
    formula_function!(
        "dt.month",
        ".dt.month",
        [],
        "Date namespace",
        ".dt.month()",
        "Extract month.",
        0,
        0
    ),
    formula_function!(
        "dt.week",
        ".dt.week",
        [],
        "Date namespace",
        ".dt.week()",
        "Extract ISO week.",
        0,
        0
    ),
    formula_function!(
        "dt.weekday",
        ".dt.weekday",
        [],
        "Date namespace",
        ".dt.weekday()",
        "Extract weekday.",
        0,
        0
    ),
    formula_function!(
        "dt.ordinal_day",
        ".dt.ordinal_day",
        ["day of year"],
        "Date namespace",
        ".dt.ordinal_day()",
        "Extract ordinal day.",
        0,
        0
    ),
    formula_function!(
        "dt.is_leap_year",
        ".dt.is_leap_year",
        [],
        "Date namespace",
        ".dt.is_leap_year()",
        "Test for leap years.",
        0,
        0
    ),
    formula_function!(
        "dt.days_in_month",
        ".dt.days_in_month",
        [],
        "Date namespace",
        ".dt.days_in_month()",
        "Count days in the month.",
        0,
        0
    ),
    formula_function!(
        "dt.date",
        ".dt.date",
        [],
        "Date namespace",
        ".dt.date()",
        "Extract Date from Datetime.",
        0,
        0
    ),
    formula_function!(
        "dt.month_start",
        ".dt.month_start",
        [],
        "Date namespace",
        ".dt.month_start()",
        "Move to the month's first day.",
        0,
        0
    ),
    formula_function!(
        "dt.month_end",
        ".dt.month_end",
        [],
        "Date namespace",
        ".dt.month_end()",
        "Move to the month's last day.",
        0,
        0
    ),
    formula_function!(
        "dt.offset_by",
        ".dt.offset_by",
        ["date offset", "add days", "shift date"],
        "Date namespace",
        ".dt.offset_by(offset)",
        "Offset dates by a duration string such as 1mo, or by an integer day count. `date + n` also adds n days.",
        1,
        1
    ),
    formula_function!(
        "str.to_uppercase",
        ".str.to_uppercase",
        ["uppercase"],
        "String namespace",
        ".str.to_uppercase()",
        "Convert strings to uppercase.",
        0,
        0
    ),
    formula_function!(
        "str.to_lowercase",
        ".str.to_lowercase",
        ["lowercase"],
        "String namespace",
        ".str.to_lowercase()",
        "Convert strings to lowercase.",
        0,
        0
    ),
    formula_function!(
        "str.to_date",
        ".str.to_date",
        [
            "ISO date",
            "parse ISO date",
            "text to date",
            "convert text date",
            "datevalue",
            "Excel DATEVALUE (ISO only)",
        ],
        "String namespace",
        ".str.to_date()",
        "Parse strict ISO date text (YYYY-MM-DD). For other formats, normalize the text first; this is the same conversion as .cast(\"date\").",
        0,
        0
    ),
    formula_function!(
        "str.contains",
        ".str.contains",
        ["string contains"],
        "String namespace",
        ".str.contains(pattern, strict=True)",
        "Test strings against a pattern.",
        1,
        1
    ),
];

pub fn formula_function_catalog() -> Vec<FormulaFunction> {
    let hand_written = POLARS_FORMULA_FUNCTIONS
        .iter()
        .map(|function| FormulaFunction {
            id: function.id.into(),
            name: function.name.into(),
            aliases: function
                .aliases
                .iter()
                .map(|alias| (*alias).into())
                .collect(),
            category: function.category.into(),
            signature: function.signature.into(),
            description: function.description.into(),
            minimum_arguments: function.minimum_arguments,
            maximum_arguments: function.maximum_arguments,
            return_type: formula_function_return_type(function.id).into(),
            null_behavior: formula_function_null_behavior(function.id).into(),
            arguments: formula_function_arguments(function.id, function.signature),
        });
    let generated = crate::formula::generated_bindings::GENERATED_FORMULA_FUNCTIONS
        .iter()
        .map(|function| FormulaFunction {
            id: function.id.into(),
            name: function.name.into(),
            aliases: Vec::new(),
            category: function.category.into(),
            signature: function.signature.into(),
            description: function.description.into(),
            minimum_arguments: function.minimum_arguments,
            maximum_arguments: function.maximum_arguments,
            return_type: function.return_type.into(),
            null_behavior: "native Polars behavior".into(),
            arguments: formula_function_arguments(function.id, function.signature),
        });
    hand_written.chain(generated).collect()
}

/// Argument help is derived beside the catalog entry, not in the editor. The
/// generated Polars surface supplies signatures automatically, while this
/// small vocabulary turns recurring names such as `periods`, `pattern`, and
/// `strict` into the thing a person should actually type. A special case is
/// only added where the generic name would hide an important contract.
pub fn formula_function_arguments(id: &str, signature: &str) -> Vec<FormulaArgument> {
    signature_parameter_labels(signature)
        .into_iter()
        .map(|label| {
            let required = !label.ends_with('?') && !label.contains('=');
            let name = label
                .trim_end_matches('?')
                .split('=')
                .next()
                .unwrap_or(&label)
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .next()
                .unwrap_or("")
                .trim_end_matches("...")
                .trim()
                .to_string();
            let (description, example) = argument_guidance(id, &name);
            FormulaArgument {
                name,
                required,
                description: description.into(),
                example: example.map(Into::into),
            }
        })
        .collect()
}

fn signature_parameter_labels(signature: &str) -> Vec<String> {
    let Some(opening) = signature.find('(') else {
        return Vec::new();
    };
    let Some(closing) = signature.rfind(')') else {
        return Vec::new();
    };
    if closing <= opening + 1 {
        return Vec::new();
    }
    let parameters = &signature[opening + 1..closing];
    let mut labels = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (offset, character) in parameters.char_indices() {
        match character {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                labels.push(parameters[start..offset].trim().to_string());
                start = offset + 1;
            }
            _ => {}
        }
    }
    labels.push(parameters[start..].trim().to_string());
    labels
        .into_iter()
        .filter(|label| !label.is_empty())
        .collect()
}

fn argument_guidance(id: &str, name: &str) -> (&'static str, Option<&'static str>) {
    match (id, name) {
        ("root.sequence", "start") => (
            "First number or date in the sequence.",
            Some("1 or 2026-01-01"),
        ),
        ("root.sequence", "stop") => (
            "Exclusive endpoint; use the same kind of value as start.",
            Some("13 or 2027-01-01"),
        ),
        ("root.sequence", "step") => (
            "Increment between values; dates need a duration.",
            Some("1, 2, or 1mo"),
        ),
        ("expr.shift", "periods") => (
            "Rows to move: positive reads an earlier row, negative a later one.",
            Some("1"),
        ),
        (_, "reverse") if id.starts_with("expr.cum_") => (
            "True accumulates from the last row upward; False accumulates top to bottom.",
            Some("False"),
        ),
        ("expr.pct_change", "n") => (
            "Rows back to compare; 1 means change from the preceding row.",
            Some("1"),
        ),
        ("expr.std", "ddof") | ("expr.var", "ddof") => (
            "Degrees of freedom: 1 gives the usual sample statistic, 0 the population one.",
            Some("1"),
        ),
        (_, "window_size") => ("Positive number of rows in each moving window.", Some("3")),
        (_, "min_periods") => (
            "Minimum non-null rows before a rolling result is returned.",
            Some("1"),
        ),
        (_, "center") => (
            "True centers a rolling window on its row; False uses prior rows.",
            Some("False"),
        ),
        ("root.format", "pattern") => (
            "Quoted text containing one {} for each value below.",
            Some("\"Q{}\""),
        ),
        (_, "expressions") | (_, "fields") => (
            "One or more column or value expressions; use [ ] where the signature shows it.",
            Some("[`Revenue`, `Cost`]"),
        ),
        (_, "condition") | (_, "predicate") => (
            "A true/false expression, usually a comparison on a column.",
            Some("`Region` == \"East\""),
        ),
        (_, "by") | (_, "group_by") => (
            "A column or expression that supplies the grouping or ordering key.",
            Some("`Customer`"),
        ),
        (_, "value")
        | (_, "values")
        | (_, "other")
        | (_, "rhs")
        | (_, "then")
        | (_, "otherwise")
        | (_, "fill_value") => (
            "A literal, column, or formula expression.",
            Some("0 or `Budget`"),
        ),
        (_, "pattern")
        | (_, "pat")
        | (_, "separator")
        | (_, "prefix")
        | (_, "suffix")
        | (_, "replace_with") => (
            "Quoted text; pattern arguments may use regular-expression syntax.",
            Some("\"CAD\""),
        ),
        (_, "periods")
        | (_, "n")
        | (_, "k")
        | (_, "length")
        | (_, "index")
        | (_, "decimals")
        | (_, "ddof")
        | (_, "group_index")
        | (_, "bin_count") => ("A whole number.", Some("1")),
        (_, "offset") if id.starts_with("dt.") => (
            "A calendar duration such as days, months, or years.",
            Some("1mo"),
        ),
        (_, "every") | (_, "period") | (_, "duration") => (
            "A duration; use d, w, mo, or y for calendar units.",
            Some("7d or 1mo"),
        ),
        (_, "strict")
        | (_, "descending")
        | (_, "reverse")
        | (_, "normalize")
        | (_, "literal")
        | (_, "ignore_nulls")
        | (_, "nulls_last")
        | (_, "closed") => ("True or False.", Some("True")),
        (_, "dtype") | (_, "data_type") => (
            "A FrameWork data type name.",
            Some("\"integer\" or \"date\""),
        ),
        (_, "min") | (_, "max") | (_, "lower_bound") | (_, "upper_bound") => (
            "A number or formula expression defining the bound.",
            Some("0"),
        ),
        (_, "quantile") | (_, "fraction") | (_, "frac") => {
            ("A decimal proportion between 0 and 1.", Some("0.5"))
        }
        _ => ("The value for this parameter.", None),
    }
}

fn formula_function_return_type(id: &str) -> &'static str {
    match id {
        "root.coalesce" | "root.when" | "root.sequence" | "root.recur" | "root.previous"
        | "expr.fill_null" | "expr.shift" | "expr.filter" | "expr.over" => "dynamic",
        "expr.is_null" | "expr.is_not_null" | "expr.is_between" | "expr.is_in"
        | "dt.is_leap_year" | "str.contains" => "boolean",
        "root.frame_len" => "integer",
        "root.date" | "root.today" | "root.now" | "dt.date" | "dt.month_start" | "dt.month_end"
        | "dt.offset_by" | "str.to_date" => "date",
        "str.to_uppercase" | "str.to_lowercase" | "root.format" => "string",
        // Whatever it was asked to become — the one function whose answer is
        // named by its own argument.
        "expr.cast" => "dynamic",
        // Still a number, whichever way it is written.
        "expr.show" => "number",
        _ => "number",
    }
}

fn formula_function_null_behavior(id: &str) -> &'static str {
    match id {
        "root.today" | "root.now" => "never null",
        "root.coalesce" => "returns first non-null",
        "expr.is_null" | "expr.is_not_null" => "inspects null",
        "expr.fill_null" => "replaces nulls",
        "root.sum_horizontal"
        | "root.mean_horizontal"
        | "root.min_horizontal"
        | "root.max_horizontal" => "configurable Polars behavior",
        _ => "propagates null",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_argument_help_explains_number_and_date_inputs() {
        let arguments =
            formula_function_arguments("root.sequence", "sequence(start, stop, step=1)");
        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0].name, "start");
        assert!(arguments[0].required);
        assert_eq!(arguments[1].example.as_deref(), Some("13 or 2027-01-01"));
        assert!(!arguments[2].required);
        assert_eq!(arguments[2].example.as_deref(), Some("1, 2, or 1mo"));
    }

    #[test]
    fn signature_parser_keeps_list_arguments_together() {
        let arguments = formula_function_arguments(
            "root.concat_str",
            "concat_str([expressions, ...], separator)",
        );
        assert_eq!(
            arguments
                .iter()
                .map(|argument| argument.name.as_str())
                .collect::<Vec<_>>(),
            vec!["expressions", "separator"]
        );
        assert!(arguments[0].description.contains("column or value"));
    }

    #[test]
    fn familiar_polars_parameters_get_formula_level_guidance() {
        let cumulative = formula_function_arguments("expr.cum_sum", ".cum_sum(reverse)");
        assert!(cumulative[0].description.contains("last row"));

        let percentage_change = formula_function_arguments("expr.pct_change", ".pct_change(n)");
        assert!(percentage_change[0].description.contains("preceding row"));

        let rolling = formula_function_arguments(
            "expr.rolling_mean",
            ".rolling_mean(window_size, min_periods=1, center=False)",
        );
        assert!(rolling[0].description.contains("number of rows"));
        assert!(rolling[1].description.contains("non-null"));
        assert!(rolling[2].description.contains("centers"));
    }

    #[test]
    fn filter_is_catalogued_as_the_conditional_aggregate_primitive() {
        let filter = formula_function_catalog()
            .into_iter()
            .find(|function| function.id == "expr.filter")
            .expect("filter is discoverable");
        assert_eq!(filter.signature, ".filter(predicate)");
        assert!(filter.aliases.iter().any(|alias| alias == "sumif"));
        assert_eq!(filter.arguments.len(), 1);
        assert_eq!(filter.arguments[0].name, "predicate");
        assert!(filter.arguments[0].description.contains("true/false"));
    }

    #[test]
    fn string_to_date_documents_the_same_strict_iso_conversion_as_cast() {
        let to_date = formula_function_catalog()
            .into_iter()
            .find(|function| function.id == "str.to_date")
            .expect("string-to-date conversion is discoverable");
        assert_eq!(to_date.signature, ".str.to_date()");
        assert_eq!(to_date.return_type, "date");
        assert!(to_date.arguments.is_empty());
        assert!(to_date.aliases.iter().any(|alias| alias == "datevalue"));
        assert!(to_date.description.contains("YYYY-MM-DD"));
        assert!(to_date.description.contains(".cast(\"date\")"));
    }
}
