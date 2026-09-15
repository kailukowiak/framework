export type HelpScope = "formulas" | "guide";

export type HelpExample = {
  label: string;
  code: string;
};

export type HelpFact = {
  term: string;
  description: string;
};

export type HelpGuide = {
  id: string;
  scopes: HelpScope[];
  kind: "Guide" | "Rule" | "Reference";
  category?: string;
  title: string;
  summary: string;
  searchTerms: string[];
  questions?: string[];
  body: string[];
  facts?: HelpFact[];
  steps?: string[];
  examples?: HelpExample[];
  surfaces?: string[];
  related?: string[];
};

/**
 * Product guidance that cannot be derived from a function signature.
 *
 * Formula availability still comes only from framework-core's catalog. These
 * entries explain how the pieces fit together: which surface owns a job, why
 * two similarly-shaped values cannot be paired by position, and how someone
 * arriving with spreadsheet or programming vocabulary finds the FrameWork
 * gesture. Questions are search language, not a second formula language.
 */
export const HELP_GUIDES: HelpGuide[] = [
  {
    id: "formula.generate-series-variable",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Generate a series in a variable",
    summary: "Make one live variable hold a generated vector of numbers or dates.",
    searchTerms: [
      "iterator", "iterate", "loop", "range", "sequence", "list", "array",
      "vector", "series", "generator", "variable", "number series", "date range",
    ],
    questions: [
      "How do I make an iterator in a variable?",
      "How do I create a list of numbers?",
      "How do I generate every month?",
    ],
    body: [
      "A variable may answer with one value or a vector; it does not need a different variable type. sequence() includes its start and excludes its stop.",
      "A vector is still one named value. If every generated value should become a table row, create a generator frame instead.",
    ],
    steps: [
      "Add a Variable and give it the name you want to reference.",
      "Use sequence(start, stop, step) as its formula.",
      "Reference the result by its variable name, or use .at(position) to pick one value.",
    ],
    examples: [
      { label: "Months 1–12", code: "sequence(1, 13)" },
      { label: "Even numbers", code: "sequence(0, 10, step=2)" },
      {
        label: "Monthly dates",
        code: "sequence(2026-01-01, 2027-01-01, step=1mo)",
      },
      { label: "Third value", code: "`months`.at(3)" },
    ],
    surfaces: ["Variable", "Scratchwork"],
    related: ["Lists, arrays, vectors, and series", "Fill a frame column in order"],
  },
  {
    id: "formula.collection-vocabulary",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Lists, arrays, vectors, and series",
    summary: "Choose the collection shape you mean without having to know Polars terminology first.",
    searchTerms: [
      "list", "array", "vector", "series", "range", "iterator", "collection",
      "list namespace", "array namespace", "arr", "sequence",
    ],
    questions: [
      "What is the difference between a list and an array?",
      "Why did array search return sequence?",
      "How do I pick one value from a vector?",
    ],
    body: [
      "In ordinary FrameWork work, a named collection of values is a vector. Write a fixed vector with [1, 2, 3], or generate one with sequence().",
      ".list methods operate on list-valued cells inside a column. .arr methods are the corresponding Polars namespace for fixed-width array-valued cells. They are not required to make an ordinary variable hold several values.",
      "FrameWork never pairs an independent vector with a frame column merely because their lengths happen to match. Make a keyed relationship, aggregate one side, or deliberately turn vectors into a frame.",
    ],
    examples: [
      { label: "Written vector", code: "[1, 2, 3]" },
      { label: "Generated vector", code: "sequence(1, 13)" },
      { label: "Pick one value", code: "`months`.at(3)" },
    ],
    surfaces: ["Variable", "Scratchwork", "Formula arguments"],
    related: ["Generate a series in a variable", "Shapes do not imply relationships"],
  },
  {
    id: "formula.fill-frame-sequence",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Fill a frame column in order",
    summary: "Generate one value per row only after the row order has been declared.",
    searchTerms: [
      "fill", "autofill", "row number", "number rows", "series", "sequence",
      "frame len", "n rows", "ordered", "sort",
    ],
    questions: ["How do I number every row?", "How do I fill a date series down a column?"],
    body: [
      "A generated column is positional, so a Sort step must appear before it. The formula then binds the sequence length to the frame at that point in Wrangle.",
      "A later display sort does not redefine the calculation; the declared transformation order is the contract.",
    ],
    steps: [
      "Open Wrangle and add or confirm the Sort that defines row order.",
      "Add a calculated column below that Sort.",
      "Use frame.len() to make exactly one generated value per row.",
    ],
    examples: [
      { label: "Row numbers", code: "sequence(1, frame.len() + 1)" },
      {
        label: "Monthly row dates",
        code: "sequence(2026-01-01, periods=frame.len(), step=1mo)",
      },
    ],
    surfaces: ["Wrangle"],
    related: ["Row order is part of the calculation", "Generate a series in a variable"],
  },
  {
    id: "formula.conditional-aggregate",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Conditional totals, counts, and averages",
    summary: "Express SUMIF-style work by filtering a column and then aggregating it.",
    searchTerms: [
      "sumif", "sumifs", "countif", "countifs", "averageif", "averageifs",
      "conditional aggregate", "where", "filter sum",
    ],
    questions: ["What is the FrameWork version of SUMIF?", "How do I total only West rows?"],
    body: [
      "The filter keeps values whose matching rows satisfy a Boolean expression. Finish it with .sum(), .count(), .mean(), or another aggregate.",
      "This reads the current live frame; it does not require a snapshot merely because the answer summarizes many rows.",
    ],
    examples: [
      {
        label: "SUMIF",
        code: "`Amount`.filter(`Region` == \"West\").sum()",
      },
      {
        label: "COUNTIF",
        code: "`Amount`.filter(`Amount` > 100).count()",
      },
    ],
    surfaces: ["Scratchwork", "Variable", "Wrangle"],
    related: ["Live means current, not unsafe", "Formula references use names"],
  },
  {
    id: "formula.previous-and-running",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Previous rows and running calculations",
    summary: "Use shift or cumulative methods when possible; use Calculate down rows for a true recurrence.",
    searchTerms: [
      "previous", "prior", "row above", "lag", "lead", "running total", "cumulative",
      "loop", "recurrence", "recursive", "carry forward", "recur", "shift",
    ],
    questions: [
      "How do I read the previous row?",
      "How do I make a running balance?",
      "How do I loop down rows?",
    ],
    body: [
      ".shift(1) reads the preceding row of another column. Cumulative methods cover running sums, counts, minima, and maxima efficiently.",
      "When each answer genuinely depends on the answer just calculated, use Calculate down rows. It stores a recurrence with a first-row expression and a later-row expression containing previous().",
    ],
    examples: [
      { label: "Previous row", code: "`Revenue`.shift(1)" },
      { label: "Running total", code: "`Amount`.cum_sum(False)" },
      {
        label: "Recurrence",
        code: "recur(`Opening`, previous() + `Change`)",
      },
    ],
    surfaces: ["Ordered Wrangle"],
    related: ["Row order is part of the calculation", "Fill a frame column in order"],
  },
  {
    id: "formula.lookup",
    scopes: ["formulas", "guide"],
    kind: "Guide",
    title: "Look up related data",
    summary: "Use a keyed Join instead of a positional lookup formula.",
    searchTerms: [
      "vlookup", "xlookup", "lookup", "match", "index match", "join", "relationship",
      "bring columns", "key",
    ],
    questions: ["What replaces VLOOKUP?", "How do I bring a column from another table?"],
    body: [
      "A lookup is a relationship between keys, so FrameWork represents it as a Join rather than as a formula that searches row positions.",
      "Drag the columns you want from the lookup frame onto the key header in the receiving frame, or choose Combine with… from the frame menu. The joined result shows its keys as the first compact step in Wrangle.",
    ],
    steps: [
      "Identify the key column shared by both frames.",
      "Drag the lookup columns onto that key, or choose Combine with… from the frame menu.",
      "Review missing and duplicate-key counts before accepting the relationship.",
    ],
    surfaces: ["Wrangle", "Canvas header drag"],
    related: ["Join", "Shapes do not imply relationships", "Ownership and literal editing"],
  },
  {
    id: "rule.work-belongs",
    scopes: ["guide"],
    kind: "Rule",
    title: "Put work where its scope belongs",
    summary: "Use a Variable for one reusable assumption, Scratchwork for one-off work, and Wrangle for every-row transformations.",
    searchTerms: [
      "variable", "scratchwork", "formula bar", "wrangle", "calculated column",
      "where", "one off", "assumption", "transformation",
    ],
    questions: ["Should this be a variable or Scratchwork?", "Where do formulas go?"],
    body: [
      "A Variable is one compact named formula on the canvas. Scratchwork is the dense home for several related calculations. A formula that applies to every row is one calculated-column declaration in Wrangle.",
      "Typing a one-off calculation into the top formula bar sends it to Scratchwork. A calculated cell points back to its column declaration rather than hiding a separate formula in the grid.",
    ],
    related: ["Calculated columns have one declaration", "Live means current, not unsafe"],
  },
  {
    id: "rule.live",
    scopes: ["guide"],
    kind: "Rule",
    title: "Live means current, not unsafe",
    summary: "Semantic formulas work on live and derived data and recompute when their inputs change.",
    searchTerms: [
      "live", "derived", "imported", "refresh", "recompute", "snapshot", "freeze",
      "cache", "aggregate", "current",
    ],
    questions: ["Do I need to freeze data before using it?", "Will a live total update?"],
    body: [
      "Filters, lookups, calculated columns, aggregates, and other semantic operations work against live frames. A refresh that changes the answer is honest live computation.",
      "Freezing is an explicit historical capture and caching is a performance choice. Neither is a prerequisite for correctness.",
    ],
    related: ["Ownership and literal editing", "Specific cells are the positional exception"],
  },
  {
    id: "rule.shapes",
    scopes: ["guide", "formulas"],
    kind: "Rule",
    title: "Shapes do not imply relationships",
    summary: "A scalar, vector, frame column, and frame are distinct shapes; equal lengths do not make two collections related.",
    searchTerms: [
      "shape", "scalar", "list", "array", "vector", "series", "column", "frame",
      "length", "positional", "align", "broadcast", "pair", "zip",
    ],
    questions: ["Why can't I pair this vector with a column?", "What is a scalar?"],
    body: [
      "A scalar may be used wherever one value is required. A vector or column may stay many-valued in Scratchwork, or be reduced with an aggregate such as .sum().",
      "Two independent collections need a key before values can be matched. FrameWork refuses silent position-by-position pairing even when the lengths happen to agree.",
    ],
    related: ["Lists, arrays, vectors, and series", "Look up related data"],
  },
  {
    id: "guide.calculation-matrix",
    scopes: ["guide", "formulas"],
    kind: "Guide",
    title: "Build a Calculation Matrix",
    summary: "Use row and column vectors plus one formula for a visible nested calculation.",
    searchTerms: [
      "calculation matrix", "matrix", "sensitivity", "data table", "what if",
      "for loop", "nested loop", "rows", "columns", "cartesian", "grid",
    ],
    questions: [
      "How do I calculate every row value against every column value?",
      "What is FrameWork's answer to a nested for loop?",
      "How do I make a sensitivity table?",
    ],
    body: [
      "Add a Calculation Matrix, then drag vectors into Rows and Columns or type their formulas there. Several fields in one axis travel together by position; a shorter field repeats only when it divides the longest field evenly.",
      "Rows cross with Columns. The shared cell formula reads the current row and column field names, and fills the grid only after that formula is valid. Until then the result stays blank instead of exposing a partial frame.",
      "The card renders the answer as a wide grid and keeps a stable long-form result containing the row fields, column fields, and calculated value.",
    ],
    examples: [
      {
        label: "Axes",
        code: "Rows: `Scenario`, `Multiplier`\nColumns: `Metric`, `Base amount`",
      },
      { label: "Cell formula", code: "`Base amount` * `Multiplier`" },
    ],
    steps: [
      "Add a Calculation Matrix from the canvas rail.",
      "Drag or type one or more vector formulas into Rows and Columns.",
      "Write the one shared cell formula that combines the current row and column values.",
    ],
    surfaces: ["Calculation Matrix", "Top formula bar", "Vector drag"],
    related: ["Lists, arrays, vectors, and series", "Shapes do not imply relationships"],
  },
  {
    id: "rule.order",
    scopes: ["guide", "formulas"],
    kind: "Rule",
    title: "Row order is part of the calculation",
    summary: "Shift, cumulative, sequence-fill, and recurrence formulas need a Sort before them in Wrangle.",
    searchTerms: [
      "order", "sort", "previous row", "next row", "shift", "cumulative", "sequence",
      "recurrence", "running", "time series",
    ],
    questions: ["Why does this formula require a sort?", "Does header sorting change shift?"],
    body: [
      "A formula that reads row position must say which ordering gives that position meaning. Put the Sort in the transformation chain before the calculation.",
      "Sorting the displayed grid later changes presentation, not the accepted calculation's declared order.",
    ],
    related: ["Previous rows and running calculations", "Fill a frame column in order"],
  },
  {
    id: "rule.ownership",
    scopes: ["guide"],
    kind: "Rule",
    title: "Ownership controls literal editing",
    summary: "Document-owned frames accept typed cell edits; imported, artifact-backed, and derived rows do not.",
    searchTerms: [
      "ownership", "owned", "read only", "can't edit", "cannot edit", "imported",
      "artifact", "derived", "adopt data", "take ownership", "freeze",
    ],
    questions: ["Why can't I edit this cell?", "Can I add a calculated column to live data?"],
    body: [
      "Ownership answers whether literal rows live in the document and may be typed into. It is separate from whether data is live, cached, frozen, or derived.",
      "Source-backed frames may still take calculated columns in Wrangle because a formula is a plan over the data, not a mutation of the source.",
    ],
    related: ["Calculated columns have one declaration", "Live means current, not unsafe"],
  },
  {
    id: "rule.calculated-column",
    scopes: ["guide"],
    kind: "Rule",
    title: "Calculated columns have one declaration",
    summary: "Create and edit every-row formulas in the frame's Wrangle chain.",
    searchTerms: [
      "calculated column", "computed column", "formula column", "formula cell",
      "wrangle", "with columns", "add column",
    ],
    questions: ["How do I add a calculated column?", "Where do I edit a calculated cell?"],
    body: [
      "Use Add calculated column from the frame menu or Wrangle. The column appears immediately and its formula remains visible as one transformation step.",
      "Double-clicking a calculated cell opens that shared declaration. A calculation needed only once belongs in Scratchwork.",
    ],
    related: ["Put work where its scope belongs", "Ownership controls literal editing"],
  },
  {
    id: "rule.references",
    scopes: ["guide", "formulas"],
    kind: "Rule",
    title: "Formula references use names",
    summary: "Click data while editing or write exact names in backticks; stable identity survives renaming.",
    searchTerms: [
      "reference", "backtick", "name", "rename", "click", "point", "column", "frame",
      "qualified", "formula syntax",
    ],
    questions: ["How do I reference a column?", "Will renaming break formulas?"],
    body: [
      "Write `Amount` for an in-scope column and `Orders`.`Amount` when the frame must be named. While a formula editor is active, clicking a column inserts the reference.",
      "Names are rendered aliases over stable object and column identities, so an accepted formula continues to refer to the same thing after a rename.",
    ],
    examples: [
      { label: "Column", code: "`Amount`" },
      { label: "Qualified column", code: "`Orders`.`Amount`" },
      { label: "Named Scratchwork line", code: "`Assumptions`.rate" },
    ],
    related: ["Specific cells are the positional exception", "Put work where its scope belongs"],
  },
  {
    id: "rule.cells",
    scopes: ["guide"],
    kind: "Rule",
    title: "Specific cells are the positional exception",
    summary: "Semantic column formulas work broadly; a clicked individual cell requires stable document-owned row identity.",
    searchTerms: [
      "specific cell", "cell reference", "click cell", "live cell", "derived cell",
      "imported cell", "position", "row id",
    ],
    questions: ["Why can't I reference this live cell?", "Can I click a cell into Scratchwork?"],
    body: [
      "With Scratchwork active, clicking a cell in an internal document-owned frame inserts its scalar reference. Imported, live, and derived results do not expose a position as though it were durable identity.",
      "This exception does not disable whole-column formulas, aggregates, profile statistics, joins, or other semantic references on live data.",
    ],
    related: ["Live means current, not unsafe", "Formula references use names"],
  },
  {
    id: "rule.nulls",
    scopes: ["guide", "formulas"],
    kind: "Rule",
    title: "Null is a missing value, not a datatype",
    summary: "Every column type may contain null; malformed non-empty input is an error instead.",
    searchTerms: [
      "null", "none", "blank", "missing", "empty", "fill null", "coalesce", "type",
    ],
    questions: ["How do I handle blanks?", "What type is null?"],
    body: [
      "Use null or None when a value is absent. Test with .is_null(), replace with .fill_null(), or choose the first present value with coalesce().",
      "A malformed value is not silently converted into null; fix it or explicitly convert it so the column keeps an honest type.",
    ],
    examples: [
      { label: "Replace missing values", code: "`Amount`.fill_null(0)" },
      { label: "First present value", code: "coalesce([`Actual`, `Budget`])" },
    ],
    related: ["Formula references use names", "Ownership controls literal editing"],
  },
  {
    id: "rule.accounting-type",
    scopes: ["guide", "formulas"],
    kind: "Rule",
    title: "Money is a column type, not a format",
    summary: "An Accounting column holds exact amounts at the decimal places it declares.",
    searchTerms: [
      "accounting", "amount", "money", "exact", "decimal", "decimal places", "scale",
      "cents", "rounding", "sum", "footing", "currency",
    ],
    questions: ["How do I store money?", "Why does my total not foot?"],
    body: [
      "Accounting is an exact type: amounts add up to the cent because they are never floats. The decimal places live on the column, so the cells display in accounting style at exactly those places without anyone setting a format.",
      "Money is never inferred — a Number or Currency column becomes exact only when you say so, with `.cast(\"accounting\")` for two places or `.cast(\"accounting\", places)` for any other number of them.",
    ],
    examples: [
      { label: "Read a column as exact amounts", code: "`Invoiced`.cast(\"accounting\")" },
      { label: "Four decimal places", code: "`Rate`.cast(\"accounting\", 4)" },
    ],
    related: ["Number formats", "Null is a missing value, not a datatype"],
  },
];
