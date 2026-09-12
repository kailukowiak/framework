pub use super::replicated::ReplicatedOperation;
use crate::Id;
use crate::model::data_artifact::{ConnectorRecipe, DataArtifact};
use crate::model::derivation::{DerivedSort, FrameJoinType};
use crate::model::frame::{
    CellUpdate, CrosstabDisplay, FrameCellStyle, FrameStyleTarget, FrameViewOrientation,
    SummaryOperation,
};
use crate::model::value::{ColumnFormat, DataType, FrozenValue, ScalarValue};
use crate::operation::input::{
    CalculationMatrixFormulaInput, FrameStepInput, FrameStyleRuleInput, JoinColumnInput,
    NamedFormulaInput,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Operation {
    AddStatisticalAnalysis {
        name: String,
        source_frame_id: Id,
        x_column_id: Id,
        #[serde(default)]
        #[ts(optional = nullable)]
        y_column_id: Option<Id>,
        request: framework_ml::StatsRequest,
        x: f64,
        y: f64,
    },
    AddModel {
        name: String,
        spec: crate::ModelSpec,
        x: f64,
        y: f64,
    },
    SetModelSpec {
        model_id: Id,
        spec: crate::ModelSpec,
    },
    FitModel {
        model_id: Id,
    },
    ImportOnnxModel {
        name: String,
        source_frame_id: Id,
        feature_column_ids: Vec<Id>,
        bytes: Vec<u8>,
        x: f64,
        y: f64,
    },
    ImportModel {
        name: String,
        source_frame_id: Id,
        feature_column_ids: Vec<Id>,
        json: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        iteration_range: Option<framework_ml::IterationRange>,
        x: f64,
        y: f64,
    },
    AddModelSummary {
        model_id: Id,
        kind: crate::ModelSummaryKind,
        name: String,
        x: f64,
        y: f64,
    },
    AddModelPredictions {
        model_id: Id,
        source_frame_id: Id,
        feature_column_ids: Vec<Id>,
        name: String,
        x: f64,
        y: f64,
    },
    /// One compact named formula directly on the canvas. It may evaluate to
    /// one value or a vector, using the same formula language as Scratchwork.
    AddVariable {
        name: String,
        formula: String,
        x: f64,
        y: f64,
    },
    AddValue {
        name: String,
        raw: String,
        x: f64,
        y: f64,
        /// The container to put it straight into. Adding something to a
        /// heading is one act, not an add followed by a move.
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
    },
    /// A computed value: the formula is the object, the answer is worked out
    /// live. `formula` is source text in the formula language, parsed in the
    /// scalar scope — canvas objects and `` `Frame`.`Column` `` references,
    /// no bare columns.
    AddResult {
        name: String,
        formula: String,
        x: f64,
        y: f64,
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
    },
    /// Replaces a result's formula, parsed the same way `AddResult` parses
    /// one.
    SetResultFormula {
        object_id: Id,
        formula: String,
    },
    /// A formula block: an ordered scratchpad of expression lines. Starts
    /// empty, to be typed into.
    AddBlock {
        name: String,
        x: f64,
        y: f64,
    },
    AddCalculationMatrix {
        name: String,
        x: f64,
        y: f64,
    },
    SetCalculationMatrix {
        object_id: Id,
        rows: Vec<CalculationMatrixFormulaInput>,
        columns: Vec<CalculationMatrixFormulaInput>,
        body: String,
    },
    /// The block retyped. `source` is the whole card's text — one line per
    /// line, `x = 10` naming a line as it defines it — and what comes back
    /// is the block's whole list of lines, with the ids of the lines that
    /// survived the edit still on them.
    ///
    /// Whole-text rather than per-line because that is the gesture: someone
    /// typing down a page does not tell you which line they are on, and a
    /// newline is not an operation.
    ///
    /// `editing` is the line the cursor is on, if anybody is holding one.
    /// Naming a line renames it for every line that reads it, and half a
    /// name is not a rename — so the line under the cursor keeps the name it
    /// already answered to until the cursor leaves. Nobody typing has to
    /// send this; without it every line is taken as finished.
    SetBlockSource {
        block_id: Id,
        source: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        editing: Option<usize>,
    },
    /// A card of prose on the canvas. Starts empty, to be typed into,
    /// exactly as a block does.
    AddText {
        x: f64,
        y: f64,
    },
    /// The card retyped: markdown with `{{…}}` holes, taken whole. Each
    /// hole is parsed as a scalar formula while preparing; one that does
    /// not parse is kept as text with its complaint, never refused.
    SetTextSource {
        object_id: Id,
        source: String,
    },
    /// A heading to keep values and lists under.
    AddContainer {
        name: String,
        x: f64,
        y: f64,
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
    },
    /// Puts an object into a container, or takes it out. `None` takes it out
    /// onto the canvas. Leaving whatever container held it before is part of
    /// the same move, so an object is never in two.
    MoveIntoContainer {
        object_id: Id,
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
    },
    /// A named list. `values` is text as it was typed or pasted — a bracketed
    /// list, a NumPy or R repr, or one value per line — and is read by
    /// `parse_list_text` rather than by each caller.
    AddSeries {
        name: String,
        values: String,
        x: f64,
        y: f64,
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
    },
    /// A named list read out of one column of a file, so a list that already
    /// exists somewhere need not be retyped. `column` names it; `None` takes
    /// the first.
    ImportSeriesFromFile {
        #[serde(default)]
        #[ts(optional = nullable)]
        container_id: Option<Id>,
        name: String,
        path: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        column: Option<String>,
        x: f64,
        y: f64,
    },
    /// Replaces a list's contents, read the same way `AddSeries` reads them.
    SetSeries {
        object_id: Id,
        values: String,
    },
    /// A list's type, when the one inferred from its values is not the one
    /// meant — postcodes that look like numbers being the usual case.
    SetSeriesType {
        object_id: Id,
        data_type: DataType,
    },
    /// A compact two-column frame with an enforced unique key.
    AddDictionary {
        name: String,
        x: f64,
        y: f64,
    },
    AddFrame {
        name: String,
        grid: Vec<Vec<String>>,
        x: f64,
        y: f64,
    },
    /// A frame made from the clipboard, dropped on the canvas: what pasting
    /// with nothing selected means.
    ///
    /// The text is read by the same Polars reader as `SetFrameFromPastedText`,
    /// so the headers, column count, and types all come from the clipboard
    /// rather than from a 2×2 placeholder that then has to be replaced —
    /// and the frame arrives in one operation, so one undo takes it back.
    AddFrameFromPastedText {
        name: String,
        text: String,
        x: f64,
        y: f64,
    },
    /// A frame whose rows are a rule instead of data: `sequence(0, 16)`,
    /// `sequence(2026-01-01, 2026-02-01, 1d)`, or bounds that name a value
    /// so the rows follow it. The table-shaped generator that Expand steps
    /// multiply against, without a hand-written list to keep up to date.
    AddGeneratorFrame {
        name: String,
        /// The rule as formula text, scalar-scoped like a scratchpad line.
        formula: String,
        /// What to call the generated column. Defaults to the frame's name.
        column_name: Option<String>,
        x: f64,
        y: f64,
    },
    /// Replaces a generated frame's rule. The column's type follows the new
    /// rule — a day-offset generator rewritten as a date range becomes a
    /// date column, and everything reading it sees dates.
    SetFrameGenerator {
        frame_id: Id,
        formula: String,
    },
    /// A hand-entered column on a computed frame, keyed by row identity.
    ///
    /// A derived or generated frame's rows regrow, so a typed value cannot
    /// live at a row position — it lives against the values of the key
    /// columns, and every read joins it back on. The key columns are
    /// enforced unique as part of the add (each entry has to land on
    /// exactly one row), so a frame whose data holds duplicates under them
    /// refuses the column.
    AddEntryColumn {
        frame_id: Id,
        name: String,
        data_type: DataType,
        key_column_ids: Vec<Id>,
    },
    /// One entered value in an entry column, addressed by its row's key
    /// column raws (in the entry column's key order). Empty text removes
    /// the entry.
    SetEntryValue {
        frame_id: Id,
        column_id: Id,
        key: Vec<String>,
        raw: String,
    },
    /// Re-saves a frame's chain exactly as it stands, so steps whose
    /// outputs were baked from data — pivot columns especially — are
    /// re-discovered against the data as it is now. Ids of surviving
    /// outputs are kept, so formulas keep meaning what they meant. Use
    /// after a parameter change moves what a pivot spreads into columns.
    RefreshFramePipeline {
        frame_id: Id,
    },
    /// Rename a set of columns atomically, including swaps of existing names.
    RenameColumns {
        frame_id: Id,
        names: Vec<(Id, String)>,
    },
    /// Resolve current header names through a keyed frame, then rename once.
    /// This is an undoable edit, not a live dependency on the mapping frame.
    RenameColumnsUsingMapping {
        frame_id: Id,
        mapping_frame_id: Id,
        key_column_id: Id,
        value_column_id: Id,
    },
    ImportFrameFromFile {
        name: String,
        path: String,
        x: f64,
        y: f64,
    },
    ImportFrameFromArtifact {
        name: String,
        artifact: DataArtifact,
        connector: Option<ConnectorRecipe>,
        /// The delimited file this artifact was staged from, when the
        /// document should be able to write a result back to it.
        ///
        /// A path rather than a resolved origin because the record an update
        /// needs — which column of this document each physical field of the
        /// file is — cannot exist until the frame does, and the frame is
        /// built here. Preparation reads the header, hashes the file, and
        /// binds the two together.
        ///
        /// Set for a stored CSV or TSV and nothing else. A connector says
        /// this frame can be *refreshed* from somewhere; this says it can be
        /// *written back* to somewhere, which is a different permission and
        /// a different file format contract — see `separator`, which is why
        /// no other format ever gets one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        file_origin: Option<String>,
        x: f64,
        y: f64,
    },
    RefreshFrameArtifact {
        frame_id: Id,
        artifact: DataArtifact,
    },
    /// Points an imported frame at a different file, keeping the frame.
    ///
    /// The distinction from `RefreshFrameArtifact` is which of the two
    /// changes: refresh re-reads the file the connector already names, this
    /// replaces the connector as well. Fields that still exist keep their
    /// column IDs, new fields receive IDs, and missing referenced fields stay
    /// in the schema so downstream failures still say what disappeared.
    SetFrameSource {
        frame_id: Id,
        artifact: DataArtifact,
        connector: ConnectorRecipe,
    },
    AddPlot {
        name: String,
        source_frame_id: Id,
        #[ts(type = "Record<string, unknown>")]
        spec: serde_json::Value,
        x: f64,
        y: f64,
        /// The card to add the plot to as a tab, when it should live beside
        /// the frame it draws rather than in a window of its own. `x`/`y`
        /// are ignored then — a tab has no position.
        #[serde(default)]
        #[ts(optional = nullable)]
        view_id: Option<Id>,
    },
    RenameObject {
        object_id: Id,
        name: String,
    },
    DeleteObject {
        object_id: Id,
    },
    SetValue {
        object_id: Id,
        raw: String,
    },
    /// Change the selected value of a slider/dropdown/date_input constructor
    /// on a named variable. Configuration and value remain in its formula.
    SetParameterValue {
        object_id: Id,
        value: ScalarValue,
    },
    SetPlotSpec {
        plot_id: Id,
        #[ts(type = "Record<string, unknown>")]
        spec: serde_json::Value,
    },
    MoveView {
        view_id: Id,
        x: f64,
        y: f64,
    },
    ResizeView {
        view_id: Id,
        width: f64,
        height: f64,
    },
    SetViewCollapsed {
        view_id: Id,
        collapsed: bool,
    },
    /// Lays every window out in lineage order with even gutters. Takes no
    /// arguments: the arrangement is a function of the document, so asking
    /// for it twice asks for the same thing.
    TidyLayout,
    SetFrameDisplayOrientation {
        frame_id: Id,
        orientation: FrameViewOrientation,
    },
    /// Freezes the first `pinned_columns` columns of a frame so they stay in
    /// view while the rest of the grid scrolls sideways. `0` unfreezes.
    /// Positional, like every spreadsheet's frozen panes, and refused above
    /// the frame's column count so the number always names real columns when
    /// it is set.
    SetFrameDisplayPinnedColumns {
        frame_id: Id,
        pinned_columns: u32,
    },
    /// Shows a long frame wide: one column per value of the names column,
    /// cells from the values column. Display only — the data stays long,
    /// nothing downstream sees a different schema, and `None` folds the
    /// view back to rows. Pair with an entry column for an editable grid
    /// whose entries survive regeneration.
    SetFrameDisplayCrosstab {
        frame_id: Id,
        crosstab: Option<CrosstabDisplay>,
    },
    /// Adds a tab to a card by creating a pass-through child of `frame_id`:
    /// a derived frame with an empty wrangle chain and its own display
    /// layer. Cheap, because nothing computes until someone reads it.
    BranchFrame {
        view_id: Id,
        frame_id: Id,
    },
    /// Reorders a card's tabs, or moves one to another card. Both are the
    /// same edit to `tab_object_ids`, so they are the same operation.
    MoveTab {
        source_view_id: Id,
        target_view_id: Id,
        object_id: Id,
        target_index: usize,
    },
    DetachTab {
        view_id: Id,
        object_id: Id,
        x: f64,
        y: f64,
    },
    SetActiveTab {
        view_id: Id,
        object_id: Id,
    },
    SetFrameDisplayFilter {
        frame_id: Id,
        filters: Vec<String>,
        #[serde(default = "default_true")]
        filter_match_all: bool,
    },
    SetFrameDisplaySort {
        frame_id: Id,
        keys: Vec<DerivedSort>,
    },
    /// Configures the profile rows immediately below a frame. Each operation
    /// is evaluated live over every compatible displayed column.
    SetFrameSummaryRows {
        frame_id: Id,
        summary_rows: Vec<SummaryOperation>,
    },
    /// Expands/collapses the profile independently of the chosen rows and
    /// remembers the vertical space the reader gave it.
    SetFrameSummaryDrawer {
        frame_id: Id,
        open: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        height: Option<f64>,
    },
    SetFrameStyle {
        frame_id: Id,
        target: FrameStyleTarget,
        style: FrameCellStyle,
    },
    SetFrameStyleRules {
        frame_id: Id,
        rules: Vec<FrameStyleRuleInput>,
    },
    SetCell {
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        raw: String,
    },
    SetCells {
        frame_id: Id,
        cells: Vec<CellUpdate>,
    },
    AddRow {
        frame_id: Id,
        values: BTreeMap<Id, String>,
    },
    /// Rebuilds an empty frame from pasted text.
    ///
    /// The text is parsed by the same Polars reader a file import uses, so
    /// a pasted column of dates becomes a date column for the same reason
    /// an imported one does. Only an empty literal frame accepts this —
    /// there is nothing to lose in one, which is what makes replacing its
    /// columns outright safe.
    SetFrameFromPastedText {
        frame_id: Id,
        text: String,
    },
    /// Writes a block of values starting at one cell, growing the frame
    /// downward when the block is taller than what is left.
    ///
    /// Distinct from `SetCells`, which can only write cells that already
    /// exist: pasting ten rows into a two-row frame should leave ten rows,
    /// not silently drop eight.
    PasteCells {
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        grid: Vec<Vec<String>>,
    },
    DeleteRow {
        frame_id: Id,
        row_id: Id,
    },
    AddColumn {
        frame_id: Id,
        name: String,
        data_type: DataType,
        after_column_id: Option<Id>,
    },
    DeleteColumn {
        frame_id: Id,
        column_id: Id,
    },
    RenameColumn {
        frame_id: Id,
        column_id: Id,
        name: String,
    },
    SetColumnType {
        frame_id: Id,
        column_id: Id,
        data_type: DataType,
    },
    SetColumnCategories {
        frame_id: Id,
        column_id: Id,
        categories: Vec<String>,
    },
    SetColumnFormat {
        frame_id: Id,
        column_id: Id,
        format: Option<ColumnFormat>,
    },
    AddComputedColumn {
        frame_id: Id,
        name: String,
        formula: String,
        /// `None` preserves the original append behavior for older clients.
        #[serde(default)]
        #[ts(optional = nullable)]
        after_column_id: Option<Id>,
    },
    SetColumnFormula {
        frame_id: Id,
        column_id: Id,
        formula: String,
    },
    SetCellOverride {
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        formula: Option<String>,
    },
    AddSummary {
        frame_id: Id,
        column_id: Id,
        operation: SummaryOperation,
    },
    AddDerivedFrame {
        source_frame_id: Id,
        name: String,
        group_keys: Vec<NamedFormulaInput>,
        aggregates: Vec<NamedFormulaInput>,
        maintain_order: bool,
        x: f64,
        y: f64,
    },
    AddLinkedFrame {
        source_frame_id: Id,
        name: String,
        x: f64,
        y: f64,
    },
    /// Renames the document itself. Save As proposes the new file's name,
    /// but the two are allowed to differ once set deliberately.
    RenameDocument {
        name: String,
    },
    /// Caches a derived frame to a parquet snapshot, or refreshes the one
    /// it already has. The frame stays derived, so it can be recomputed or
    /// set back to reading live.
    SetFrameMaterialization {
        frame_id: Id,
        artifact: DataArtifact,
    },
    /// Drops a frame's snapshot and returns it to reading live.
    ClearFrameMaterialization {
        frame_id: Id,
    },
    /// Writes down a value's answer, or lets it go back to being worked out.
    ///
    /// The artifact is written before this is applied, the way a frame's
    /// snapshot is: reading live is the caller's job, and every replica is
    /// handed the same recorded answer rather than each computing its own
    /// from data it may not have.
    SetFrozenValue {
        object_id: Id,
        frozen: Option<FrozenValue>,
    },
    /// A named assumption bundle — Base, Upside, Downside — over the
    /// document's value objects.
    ///
    /// It starts empty and agreeing with the base about everything, unless
    /// `copy_from` names another scenario, which is how a Downside is
    /// usually written: take the Upside and change the two numbers that
    /// differ.
    AddScenario {
        /// Minted when absent. Supplying one is for a caller that has to
        /// know the id before the edit lands.
        #[serde(default)]
        #[ts(optional = nullable)]
        scenario_id: Option<Id>,
        name: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        copy_from: Option<Id>,
    },
    /// Deletes a scenario and every override in it. If it was the active
    /// one, the document goes back to reading the base.
    RemoveScenario {
        scenario_id: Id,
    },
    RenameScenario {
        scenario_id: Id,
        name: String,
    },
    /// What one value holds in one scenario. `None` — or empty text — drops
    /// the override, so the value reads its own number there again.
    ///
    /// The raw is checked against the value's own data type rather than
    /// re-inferred the way `SetValue` re-infers one: a scenario disagrees
    /// with the base about a number, never about what kind of thing the
    /// value is, and a `Downside` that quietly turned a currency into text
    /// would break every formula reading it the moment it was activated.
    SetScenarioValue {
        scenario_id: Id,
        value_id: Id,
        #[serde(default)]
        #[ts(optional = nullable)]
        raw: Option<String>,
    },
    /// Switches the whole document onto a scenario, or `None` back to the
    /// base. Every result, calculated column and Scratchwork line reading an
    /// overridden value recomputes.
    ActivateScenario {
        #[serde(default)]
        #[ts(optional = nullable)]
        scenario_id: Option<Id>,
    },
    /// Cuts every outside dependency the document has, in one edit.
    ///
    /// What you want before sending a document to somebody: every connector
    /// dropped, so nothing refreshes from a file that only exists on this
    /// machine, and every frame that was reading a path directly given data
    /// of its own. Afterwards the document and its sidecar are the whole of
    /// it — which is also what makes every frame in it editable.
    ///
    /// `adopted` carries the artifacts written for frames that had no data
    /// of their own to keep, since the model writes no files itself. The
    /// connectors are worked out here rather than passed in: the document
    /// knows which frames have one.
    PackageDocument {
        adopted: Vec<(Id, DataArtifact)>,
    },
    /// Makes the frame's current values the document's own data.
    ///
    /// The values are written to a parquet in the document's sidecar and the
    /// frame is pointed at it as an ordinary import — of itself. Whatever it
    /// read from before is let go: the connector that would have refreshed
    /// over these values, the chain that would have recomputed them, the
    /// snapshot that was only ever a cache of them.
    ///
    /// The point is ownership, not format. A cached snapshot is a copy the
    /// document keeps of something that lives elsewhere, and anything typed
    /// into it is discarded by the next refresh. This is the same parquet
    /// with nothing left to be refreshed *from*, which is what makes it
    /// something a person can edit and expect to keep.
    ///
    /// The artifact is written by the caller, like a snapshot: the document
    /// model does no file I/O of its own.
    AdoptFrameRows {
        frame_id: Id,
        artifact: DataArtifact,
    },
    /// Settles the corrections typed over a frame's base read by writing a
    /// file that already contains them.
    ///
    /// A typed-over cell, a struck-out row and a row added past the end are
    /// notes against a file nobody rewrote, which is what makes an edit cost
    /// the edit rather than the table. They accumulate, though, and this is
    /// the request to fold them in: each frame is pointed at a fresh
    /// artifact that reads the same as the base did with its patches
    /// applied, and pointing it there is what drops them.
    ///
    /// One operation for every frame folded, so settling a document costs one
    /// undo rather than one per table — the same reason `PackageDocument` is
    /// one operation. The artifacts are written by the caller, like a
    /// snapshot's: the document model does no file I/O of its own.
    FoldRowPatches {
        folded: Vec<(Id, DataArtifact)>,
    },
    /// Moves a frame's display filter and sort into its wrangle chain, so
    /// what was presentation becomes lineage and every frame derived from
    /// this one starts seeing it. The one-way door between the View tab and
    /// the Wrangle tab, taken deliberately.
    PromoteDisplayToSteps {
        frame_id: Id,
    },
    SetUniqueKey {
        frame_id: Id,
        column_ids: Vec<Id>,
        enabled: bool,
    },
    AddJoinFrame {
        primary_frame_id: Id,
        lookup_frame_id: Id,
        primary_key_column_ids: Vec<Id>,
        lookup_key_column_ids: Vec<Id>,
        join_type: FrameJoinType,
        columns: Vec<JoinColumnInput>,
        name: String,
        x: f64,
        y: f64,
    },
    /// Changes the relationship behind an existing joined frame without
    /// replacing its output columns or the Wrangle steps built on them.
    SetFrameJoinKeys {
        frame_id: Id,
        primary_key_column_ids: Vec<Id>,
        lookup_key_column_ids: Vec<Id>,
    },
    /// Replaces a derived frame's whole transformation chain.
    ///
    /// Formulas arrive as text and are parsed against the schema at their
    /// own position in the chain, so a step may reference a column an
    /// earlier step produced. The declared columns are recomputed from what
    /// the final step actually leaves behind.
    SetFramePipeline {
        frame_id: Id,
        steps: Vec<FrameStepInput>,
    },
    /// A remark pinned to the frame itself — what this frame *is*, for the
    /// next person to open the document. Markdown, never parsed. `None`
    /// (or a blank string, normalized while preparing) removes it, and with
    /// it the icon that announces one.
    SetFrameComment {
        frame_id: Id,
        #[serde(default)]
        #[ts(optional = nullable)]
        comment: Option<String>,
    },
}

fn default_true() -> bool {
    true
}
