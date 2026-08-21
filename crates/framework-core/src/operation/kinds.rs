use crate::Id;
use crate::formula::ast::Formula;
use crate::model::data_artifact::{ConnectorRecipe, DataArtifact};
use crate::model::derivation::{
    DerivedSort, FrameDerivation, FrameJoinType, FrameStep, Materialization, UniqueKeyConstraint,
};
use crate::model::document::CanvasView;
use crate::model::document::DataObject;
use crate::model::frame::{
    CellUpdate, Column, CrosstabDisplay, EntryValue, FrameCellStyle, FrameGenerator, FrameObject,
    FrameStyleRule, FrameStyleTarget, FrameViewOrientation, Row, Summary, SummaryOperation,
};
use crate::model::layout::ViewPlacement;
use crate::model::value::BlockLine;
use crate::model::value::ColumnFormat;
use crate::model::value::DataType;
use crate::model::value::FrozenValue;
use crate::model::value::TextSegment;
use crate::operation::input::{
    FrameStepInput, FrameStyleRuleInput, JoinColumnInput, NamedFormulaInput,
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
    AddFrame {
        name: String,
        grid: Vec<Vec<String>>,
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

/// A fully resolved mutation that can be serialized once and replayed on
/// another replica without generating new IDs or resolving formula names
/// against a different document state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
// Measured: 840 bytes, against 352 for the next largest. The whole difference
// is `AddObject`, which carries a `DataObject` (704) plus a `CanvasView` (136);
// see the note on [`DataObject`] for why that is not boxed. 840 bytes moved per
// applied edit is far below anything interactive editing notices.
#[allow(clippy::large_enum_variant)]
pub enum ReplicatedOperation {
    AddObject {
        object: DataObject,
        view: CanvasView,
        /// The container it lands in, when it was created inside one.
        #[serde(default)]
        container_id: Option<Id>,
    },
    RefreshFrameArtifact {
        frame_id: Id,
        artifact: DataArtifact,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
    },
    SetFrameSource {
        frame_id: Id,
        artifact: DataArtifact,
        connector: ConnectorRecipe,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
    },
    RenameObject {
        object_id: Id,
        name: String,
        /// Every block whose text spelled the old name, with its lines
        /// rewritten to spell the new one.
        ///
        /// A reference is an id everywhere else in the document, so a formula
        /// that reads the renamed thing needs no edit — it is written back
        /// out under whatever the name is now. A block line is the one
        /// exception, because its text is kept as the author typed it: that
        /// text is the thing they are looking at. So a rename has to edit it,
        /// or the next keystroke in that block re-parses a name that is no
        /// longer there and the line breaks for no visible reason.
        #[serde(default)]
        blocks: Vec<(Id, Vec<BlockLine>)>,
    },
    DeleteObject {
        object_id: Id,
    },
    SetValue {
        object_id: Id,
        raw: String,
    },
    /// Already parsed, so every replica holds the same expression with the
    /// same references bound by id, whatever its names have been changed to
    /// since the text was typed.
    SetResultFormula {
        object_id: Id,
        formula: Formula,
    },
    /// Every line of the block, fully determined: ids settled, names worked
    /// out, and each formula parsed where it parsed.
    ///
    /// The whole list travels rather than a diff of it. A block is small and
    /// its lines mean something only in order, so a replica applying this
    /// ends up with the text the author is looking at, which a sequence of
    /// per-line edits could only approximate.
    SetBlockLines {
        block_id: Id,
        lines: Vec<BlockLine>,
    },
    /// Already split and parsed, so every replica holds the same holes
    /// bound to the same ids.
    SetTextSegments {
        object_id: Id,
        segments: Vec<TextSegment>,
    },
    /// Already read into values and typed, so every replica holds the same
    /// list rather than each re-reading text that may not parse the same way
    /// twice.
    SetSeries {
        object_id: Id,
        values: Vec<String>,
        data_type: DataType,
    },
    SetSeriesType {
        object_id: Id,
        data_type: DataType,
    },
    /// Every container whose membership the move changed, resolved while
    /// preparing so a replica reproduces the same arrangement rather than
    /// working out for itself which container an object used to be in.
    SetContainerMembers {
        members: Vec<(Id, Vec<Id>)>,
    },
    SetPlotSpec {
        plot_id: Id,
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
    /// Every window's tidied position, resolved while preparing so a replica
    /// reproduces the arrangement rather than recomputing one from a
    /// document that may since have moved on.
    SetViewLayout {
        placements: Vec<ViewPlacement>,
    },
    SetFrameDisplayOrientation {
        frame_id: Id,
        orientation: FrameViewOrientation,
    },
    SetFrameDisplayCrosstab {
        frame_id: Id,
        crosstab: Option<CrosstabDisplay>,
    },
    /// Adds an object to a card's tab strip and selects it.
    ///
    /// The object is fully determined here — its id, and a frame's column
    /// ids, are minted while preparing — so every replica adds the same one.
    /// Branching a frame and plotting one into the same card are the same
    /// edit to the strip, so they resolve to the same operation.
    AddTab {
        view_id: Id,
        object: DataObject,
    },
    MoveTab {
        source_view_id: Id,
        target_view_id: Id,
        object_id: Id,
        target_index: usize,
    },
    DetachTab {
        source_view_id: Id,
        object_id: Id,
        new_view: CanvasView,
    },
    SetActiveTab {
        view_id: Id,
        object_id: Id,
    },
    SetFrameDisplayFilter {
        frame_id: Id,
        filters: Vec<Formula>,
        filter_match_all: bool,
    },
    SetFrameDisplaySort {
        frame_id: Id,
        keys: Vec<DerivedSort>,
    },
    SetFrameSummaryRows {
        frame_id: Id,
        summary_rows: Option<Vec<SummaryOperation>>,
    },
    SetFrameSummaryDrawer {
        frame_id: Id,
        open: bool,
        height: Option<f64>,
    },
    SetFrameStyle {
        frame_id: Id,
        target: FrameStyleTarget,
        style: FrameCellStyle,
    },
    SetFrameStyleRules {
        frame_id: Id,
        rules: Vec<FrameStyleRule>,
    },
    PromoteDisplayToSteps {
        frame_id: Id,
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
        row: Row,
        after_row_id: Option<Id>,
    },
    /// The rebuilt columns and rows, resolved while preparing so every
    /// replica gets the same IDs and the same inferred types.
    SetFrameContent {
        frame_id: Id,
        columns: Vec<Column>,
        rows: Vec<Row>,
    },
    /// Cell writes plus any rows the paste had to add, as one edit.
    PasteCells {
        frame_id: Id,
        cells: Vec<CellUpdate>,
        appended_rows: Vec<Row>,
    },
    DeleteRow {
        frame_id: Id,
        row_id: Id,
    },
    AddColumn {
        frame_id: Id,
        column: Column,
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
    SetColumnFormula {
        frame_id: Id,
        column_id: Id,
        formula: Formula,
        data_type: DataType,
    },
    SetCellOverride {
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        formula: Option<Formula>,
    },
    AddSummary {
        frame_id: Id,
        summary: Summary,
    },
    SetUniqueKeys {
        frame_id: Id,
        unique_keys: Vec<UniqueKeyConstraint>,
    },
    /// The resolved form of `SetFrameGenerator`: the parsed rule, and the
    /// frame's columns with the generated column's type already following
    /// it. Carried together so every replica re-types the column the same
    /// way without re-evaluating the rule.
    SetFrameGenerator {
        frame_id: Id,
        generator: FrameGenerator,
        columns: Vec<Column>,
    },
    /// The resolved form of `AddEntryColumn`. `entries` is empty on a fresh
    /// add; undo of a removal carries the values back through here.
    /// `unique_key` is the key constraint minted when the frame did not
    /// already enforce one over the key columns — minted at prepare, not
    /// apply, so every replica adds the same key.
    AddEntryColumn {
        frame_id: Id,
        column: Column,
        key_column_ids: Vec<Id>,
        entries: Vec<EntryValue>,
        unique_key: Option<UniqueKeyConstraint>,
    },
    RemoveEntryColumn {
        frame_id: Id,
        column_id: Id,
    },
    SetEntryValue {
        frame_id: Id,
        column_id: Id,
        key: Vec<String>,
        raw: String,
    },
    SetFrameDerivation {
        frame_id: Id,
        name: String,
        columns: Vec<Column>,
        derivation: FrameDerivation,
    },
    /// A source frame's own chain. Empty `steps` clears it, in which case
    /// `columns` is the frame's data schema again and `base_columns` empty.
    SetFrameSteps {
        frame_id: Id,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
        steps: Vec<FrameStep>,
    },
    /// `None` returns the frame to reading live.
    SetFrameMaterialization {
        frame_id: Id,
        materialization: Option<Materialization>,
    },
    SetFrameComment {
        frame_id: Id,
        comment: Option<String>,
    },
    SetFrozenValue {
        object_id: Id,
        frozen: Option<FrozenValue>,
    },
    AdoptFrameRows {
        frame_id: Id,
        artifact: DataArtifact,
    },
    /// Every frame named here loses its connector; every frame in `adopted`
    /// is given the artifact written for it. Resolved while preparing, so
    /// each replica packages exactly the same set.
    PackageDocument {
        unlinked: Vec<Id>,
        adopted: Vec<(Id, DataArtifact)>,
    },
    /// One cell of a frame whose values live in a parquet the document owns.
    ///
    /// Addressed by ordinal rather than row id because a scanned row has no
    /// id of its own — the ordinal is its identity, and it is stable exactly
    /// as long as nothing rewrites the file underneath it, which for an
    /// owned artifact means "until the next edit, which is this one".
    ///
    /// Applying it rewrites the parquet. That is the cost of the design and
    /// it is deliberate: the alternative is an overlay keyed to row
    /// identity, which is a larger machine and buys nothing for a file
    /// nobody else is writing to.
    SetArtifactCell {
        frame_id: Id,
        row_ordinal: usize,
        column_id: Id,
        raw: String,
    },
    RenameDocument {
        name: String,
    },

    // The three below exist only to be inverses. Undo is an ordinary edit
    // applied forward, so every operation needs one — and most get it from
    // the operation itself, replayed with the values it replaced. These
    // cover the cases where that is impossible: an edit that destroyed
    // structure, or rebuilt something wholesale, leaves no earlier state
    // for its own operation to describe.
    //
    // They carry a subtree rather than a document, which is what keeps this
    // different from the snapshot history it replaces. The payloads are
    // bounded: a literal frame's rows are small by construction, and an
    // imported frame keeps none in the document at all.
    /// Puts a frame back exactly as it was.
    ///
    /// The inverse of every edit that rewrites one frame's shape rather
    /// than its values — dropping a column (which also takes that column's
    /// summaries and cell overrides with it), replacing its content,
    /// promoting its display layer into its chain, repointing its source.
    RestoreFrame {
        frame: FrameObject,
    },
    /// Puts a deleted object back, with every card that showed it.
    RestoreObject {
        object: DataObject,
        views: Vec<CanvasView>,
    },
    /// Puts the canvas back: which cards exist, what each shows, and which
    /// of its tabs is selected. The inverse of the tab operations, where a
    /// card can appear or disappear as its strip empties or fills.
    RestoreViews {
        views: Vec<CanvasView>,
    },
}

fn default_true() -> bool {
    true
}
