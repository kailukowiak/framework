//! Fully resolved edits replay fitted state instead of rerunning training.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    SetModelSpec {
        model_id: Id,
        spec: crate::ModelSpec,
    },
    SetModelFit {
        model_id: Id,
        fitted: Option<crate::ModelFit>,
    },
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
    SetCalculationMatrix {
        object_id: Id,
        rows: Vec<CalculationMatrixAxisFormula>,
        columns: Vec<CalculationMatrixAxisFormula>,
        body: CalculationMatrixBody,
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
    SetFrameDisplayPinnedColumns {
        frame_id: Id,
        pinned_columns: u32,
    },
    /// Widths already held to the drawable range, so every replica stores
    /// the same number. Its own inverse, carrying each column's prior
    /// layout.
    SetFrameDisplayColumnLayout {
        frame_id: Id,
        columns: Vec<FrameColumnLayoutUpdate>,
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
    /// `before_row_id: None` appends. Named for the row that follows because
    /// the inverse of deleting the first row has to say "at the top", which
    /// a row to follow cannot.
    AddRow {
        frame_id: Id,
        row: Row,
        before_row_id: Option<Id>,
    },
    /// The rebuilt columns and rows, resolved while preparing so every
    /// replica gets the same IDs and the same inferred types.
    SetFrameContent {
        frame_id: Id,
        columns: Vec<Column>,
        rows: Vec<Row>,
    },
    /// Cell writes plus any rows and columns the paste had to add, as one
    /// edit. The columns go on first, at the right-hand end, so every
    /// appended row and every cell write names a column that exists.
    PasteCells {
        frame_id: Id,
        cells: Vec<CellUpdate>,
        appended_rows: Vec<Row>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        added_columns: Vec<Column>,
    },
    DeleteRow {
        frame_id: Id,
        row_id: Id,
    },
    /// Several of the frame's own rows at once. The ids are settled at
    /// prepare — unique, and every one present — so a replica removes exactly
    /// the rows the author selected.
    DeleteRows {
        frame_id: Id,
        row_ids: Vec<Id>,
    },
    AddColumn {
        frame_id: Id,
        column: Column,
        after_column_id: Option<Id>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        before_column_id: Option<Id>,
        /// Settled at prepare (rows exist, values valid), so a replica fills
        /// exactly the cells the author did. Empty for every older history.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        values: BTreeMap<Id, String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale: Option<u8>,
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
    /// The resolved form of `SetFramePeriod`. The declaration carries no
    /// minted ids, so prepare hands it through unchanged after validating
    /// it; clearing is the same variant with `period: None`.
    SetFramePeriod {
        frame_id: Id,
        period: Option<FramePeriod>,
    },
    /// The resolved form of `AddCalendar` and `UpdateCalendar`: the whole
    /// validated calendar with its minted id. Undo of a removal carries
    /// the calendar back through here.
    AddCalendar {
        calendar: Calendar,
    },
    UpdateCalendar {
        calendar: Calendar,
    },
    RemoveCalendar {
        calendar_id: Id,
    },
    SetDefaultCalendar {
        calendar_id: Option<Id>,
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
    /// The whole scenario, values and all, so a replica adds the same
    /// bundle rather than re-resolving `copy_from` against a document that
    /// may have moved on.
    AddScenario {
        scenario: Scenario,
    },
    RemoveScenario {
        scenario_id: Id,
    },
    RenameScenario {
        scenario_id: Id,
        name: String,
    },
    SetScenarioValue {
        scenario_id: Id,
        value_id: Id,
        raw: Option<String>,
    },
    ActivateScenario {
        scenario_id: Option<Id>,
    },
    AdoptFrameRows {
        frame_id: Id,
        artifact: DataArtifact,
    },
    /// Every frame named here is pointed at the artifact written for it,
    /// which is what clears the patches it was carrying. Resolved while
    /// preparing, so each replica folds exactly the same set.
    FoldRowPatches {
        folded: Vec<(Id, DataArtifact)>,
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
    /// Which base rows are struck out, and how many rows are added past the
    /// base's end — set together, the way a pipeline is.
    ///
    /// Carrying both whole lists rather than one delta makes the inverse the
    /// previous pair and nothing else, which is the same bargain
    /// `SetFramePipeline` makes and for the same reason: these lists are small
    /// by construction, and a delta would need an ordering story.
    SetRowPatches {
        frame_id: Id,
        deleted_rows: Vec<u32>,
        appended_rows: u32,
    },
    /// A value typed over one cell of a base this document reads rather than
    /// holds, recorded as a patch instead of rewriting that base.
    ///
    /// `raw: None` removes the patch, which is what makes this its own inverse:
    /// undoing a correction either restores the correction that was there
    /// before or takes the cell back to what the base says.
    SetOverlayCell {
        frame_id: Id,
        row_ordinal: u32,
        column_id: Id,
        raw: Option<String>,
    },
    /// Several typed-over values on one base read, as one edit — what a range
    /// clear, a fill, a cut or a pasted block becomes on a frame whose rows
    /// live in a file.
    ///
    /// Its own operation rather than a run of `SetOverlayCell`s because an
    /// edit is one replicated operation and one history entry: a pasted block
    /// that took twenty undos to take back, or that a peer could receive half
    /// of, would not be the one rectangle somebody pasted.
    ///
    /// `appended_rows` is the count of rows past the base's end once this has
    /// applied, or `None` to leave it alone. A paste that runs past the last
    /// row is the only edit that sets it; leaving it absent otherwise keeps a
    /// range clear from overwriting a row a peer added meanwhile. Each patch
    /// takes `raw: None` to withdraw itself, so the inverse is this same
    /// operation carrying the prior count and each cell's prior patch — which
    /// is how undo tells a cell nobody touched from one somebody cleared.
    SetOverlayCells {
        frame_id: Id,
        appended_rows: Option<u32>,
        cells: Vec<OverlayCellPatch>,
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
    /// Puts the scenario list and the activation back exactly as they were.
    ///
    /// The inverse of the two edits that destroy more than they name:
    /// deleting a scenario, which takes every override in it and possibly
    /// the activation as well, and deleting a *value*, which silently drops
    /// that value's override from every scenario at once. Neither has a
    /// forward operation that can describe what was there, and the list is
    /// a handful of names and literals, so it travels whole.
    RestoreScenarios {
        scenarios: Vec<Scenario>,
        active_scenario: Option<Id>,
    },
    /// Replaces runs of a frame's direct formats, in place.
    ///
    /// The inverse of formatting a range, which can trim, remove or split
    /// the cell formats and earlier ranges it covers, and push itself on
    /// the end. Order matters among ranges — a later one wins over an
    /// earlier one — so no set of single-target edits replays that
    /// faithfully. It used to travel as the whole list, which undid a
    /// collaborator's formatting made in between along with this writer's
    /// own; splices name only the entries the edit touched, so the rest of
    /// the list, including anything added since, stays as it is. A splice
    /// whose entries a later edit has changed is skipped rather than
    /// forced (see `FrameDisplay::resolve_style_splices`).
    SpliceFrameStyles {
        frame_id: Id,
        splices: Vec<FrameStyleSplice>,
    },
    /// Several edits as one: one event, one revision, one undo entry.
    ///
    /// Each was prepared against the document as the ones before it left
    /// it, so applying them in order on any replica reproduces the same
    /// state, and they apply all or not at all. See `Operation::Batch`.
    Batch {
        operations: Vec<ReplicatedOperation>,
    },
}

/// One cell of a [`ReplicatedOperation::SetOverlayCells`]: the row by its
/// ordinal in the base read, and the text typed over it, or `None` to
/// withdraw the patch and let the base's value show again.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCellPatch {
    pub row_ordinal: u32,
    pub column_id: Id,
    pub raw: Option<String>,
}
