//! Turning imported frames and literal grids into frame objects.

use crate::*;
use polars::prelude as pl;
use polars::prelude::SerReader;
use std::fs;
use std::path::Path;

/// Everything a frame card draws above and below its rows: the card border
/// (2), the drag handle (32), the tab strip (36), the title row (50), the
/// grid's bottom padding (12), and inside the scroller the column header
/// (34), the type row (26), the draft "type here" row an owned frame always
/// shows (34) and the four 2px gaps the table leaves around them.
///
/// The gaps are the part that keeps getting missed: the grid's table is in
/// the separated-border model with the user agent's own 2px `border-spacing`
/// (the stylesheet's reset names `frame`, not `table`, so it never applies),
/// which makes a row cost 36 rather than 34 and leaves one spacing over at
/// the bottom. The tally was 184 for a long time and then 230, and both times
/// a six-row table arrived showing five rows and an in-card scrollbar — sized
/// for rows it had no room to draw. `frameCardHeight` in
/// `src/lib/cardPlacement.ts` is the same arithmetic on the other side of the
/// wire, for the paste path that resizes a card the front end already owns;
/// the two have to move together.
const FRAME_CHROME_HEIGHT: f64 = 234.0;
const FRAME_ROW_HEIGHT: f64 = 36.0;
/// How many rows a card sizes itself to before it becomes a thing to scroll.
const FRAME_AUTOMATIC_ROW_CAP: usize = 12;
const FRAME_MIN_HEIGHT: f64 = 300.0;

/// A frame card tall enough to show what arrived, up to a dozen rows: a card
/// that clips the last row of a six-row paste is the first thing a person has
/// to fix, every time.
/// The width a frame card opens at: room for its columns, within reason.
pub(crate) fn frame_card_width(column_count: usize) -> f64 {
    (column_count.max(1) as f64 * 150.0 + 48.0).clamp(420.0, 900.0)
}

pub(crate) fn frame_card_height(row_count: usize) -> f64 {
    let maximum = FRAME_CHROME_HEIGHT + FRAME_AUTOMATIC_ROW_CAP as f64 * FRAME_ROW_HEIGHT;
    (FRAME_CHROME_HEIGHT + row_count.min(FRAME_AUTOMATIC_ROW_CAP) as f64 * FRAME_ROW_HEIGHT)
        .clamp(FRAME_MIN_HEIGHT, maximum)
}

impl Document {
    pub(crate) fn build_frame(
        name: String,
        grid: Vec<Vec<String>>,
        x: f64,
        y: f64,
    ) -> (FrameObject, CanvasView) {
        let width = grid.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let data_types = (0..width)
            .map(|index| infer_column_type(&grid, index))
            .collect();
        Self::build_frame_with_types(name, grid, data_types, x, y)
    }

    /// A frame read out of pasted text, typed by the same reader a paste
    /// into an empty frame uses.
    pub(crate) fn build_frame_from_pasted_text(
        name: String,
        text: &str,
        x: f64,
        y: f64,
    ) -> Result<(FrameObject, CanvasView), CoreError> {
        let (columns, rows) = Self::frame_content_from_frame(&read_pasted_frame(text)?);
        if columns.is_empty() {
            return Err(CoreError::Import("There is nothing to paste".into()));
        }
        Ok(Self::frame_with_view(name, columns, rows, x, y))
    }

    /// Build a new source frame from a CSV, TSV, or Parquet file.
    ///
    /// The file's Polars schema decides each column type, so a Parquet string
    /// column of digits stays text instead of being re-inferred from raw
    /// values. Unmappable dtypes fall back to text via cast.
    pub(crate) fn build_imported_frame(
        name: String,
        path: &Path,
        x: f64,
        y: f64,
    ) -> Result<(FrameObject, CanvasView), CoreError> {
        // An MCP client quite reasonably says `actuals.csv` when the file is
        // beside its working document. That spelling is meaningful only to
        // the server process that handled the import, though: after a restart
        // the process may have a different working directory while the frame
        // still means the same source. Resolve the path at the moment we know
        // it exists so persistence records the file that was actually read,
        // not an ambient-process instruction that expires with this session.
        let source_path = fs::canonicalize(path).map_err(|error| {
            CoreError::Import(format!("Could not resolve imported file: {error}"))
        })?;
        let frame = read_import_frame(&source_path)?;
        let mut data_types = Vec::new();
        let mut categories = Vec::new();
        for column in frame.columns() {
            let series = column.as_materialized_series();
            let data_type = framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
            data_types.push(data_type);
            categories.push(declared_categories(series.dtype()));
        }
        let headers: Vec<String> = frame
            .get_column_names()
            .iter()
            .map(|column_name| column_name.to_string())
            .collect();
        let frame_id = id();
        let row_count = frame.height();
        let columns: Vec<Column> = headers
            .iter()
            .enumerate()
            .map(|(index, header)| {
                let name = if header.trim().is_empty() {
                    format!("Column {}", index + 1)
                } else {
                    header.clone()
                };
                Column {
                    id: column_id(&name),
                    name: name.clone(),
                    source_name: Some(header.clone()),
                    data_type: data_types.get(index).copied().unwrap_or(DataType::String),
                    scale: None,
                    // A file that names its own allowed values arrives with the
                    // dropdown already filled in.
                    categories: categories.get(index).cloned().unwrap_or_default(),
                    format: None,
                    formula: None,
                }
            })
            .collect();
        let frame = FrameObject {
            id: frame_id.clone(),
            name,
            columns,
            source_file: Some(source_path.to_string_lossy().to_string()),
            ..FrameObject::default()
        };
        let width = data_types.len().max(1);
        let view = CanvasView {
            id: id(),
            object_id: frame_id,
            x,
            y,
            width: frame_card_width(width),
            height: frame_card_height(row_count),
            collapsed: false,
            tab_object_ids: Vec::new(),
        };
        Ok((frame, view))
    }

    pub(crate) fn build_artifact_frame(
        name: String,
        mut artifact: DataArtifact,
        connector: Option<ConnectorRecipe>,
        x: f64,
        y: f64,
    ) -> Result<(FrameObject, CanvasView), CoreError> {
        if artifact.format != ArtifactFormat::Parquet {
            return Err(CoreError::Import("Unsupported artifact format".into()));
        }
        let file =
            fs::File::open(&artifact.path).map_err(|error| CoreError::Import(error.to_string()))?;
        let mut reader = pl::ParquetReader::new(file).with_slice(Some((0, 0)));
        artifact.row_count = reader
            .num_rows()
            .map_err(|error| CoreError::Import(error.to_string()))?;
        let row_count = artifact.row_count;
        let frame = reader
            .finish()
            .map_err(|error| CoreError::Import(error.to_string()))?;
        let frame_id = id();
        let columns = frame
            .columns()
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let source_name = source.name().to_string();
                let name = if source_name.trim().is_empty() {
                    format!("Column {}", index + 1)
                } else {
                    source_name.clone()
                };
                Column {
                    id: column_id(&name),
                    name,
                    source_name: Some(source_name),
                    data_type: framework_type_from_polars(source.dtype())
                        .unwrap_or(DataType::String),
                    scale: decimal_scale_from_polars(source.dtype()),
                    categories: declared_categories(source.dtype()),
                    format: None,
                    formula: None,
                }
            })
            .collect::<Vec<_>>();
        let width = columns.len().max(1);
        let frame = FrameObject {
            id: frame_id.clone(),
            name,
            columns,
            artifact: Some(artifact),
            connector,
            ..FrameObject::default()
        };
        let view = CanvasView {
            id: id(),
            object_id: frame_id,
            x,
            y,
            width: frame_card_width(width),
            // A frozen or connected frame is paged rather than held in the
            // document, but the card still has to show rows: it was fixed at
            // the minimum height, which is two rows once the chrome is
            // counted honestly.
            height: frame_card_height(row_count),
            collapsed: false,
            tab_object_ids: Vec::new(),
        };
        Ok((frame, view))
    }

    /// Columns and rows for a frame rebuilt from a pasted frame.
    ///
    /// Unlike the derived-frame path this keys cells positionally: a pasted
    /// frame is named by its own headers, not by this document's column IDs,
    /// which are being minted right here.
    pub(crate) fn frame_content_from_frame(frame: &pl::DataFrame) -> (Vec<Column>, Vec<Row>) {
        let columns: Vec<Column> = frame
            .columns()
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let name = {
                    let name = source.name().as_str();
                    if name.trim().is_empty() {
                        format!("Column {}", index + 1)
                    } else {
                        name.to_string()
                    }
                };
                Column {
                    id: column_id(&name),
                    name,
                    source_name: None,
                    data_type: framework_type_from_polars(source.dtype())
                        .unwrap_or(DataType::String),
                    scale: decimal_scale_from_polars(source.dtype()),
                    categories: declared_categories(source.dtype()),
                    format: None,
                    formula: None,
                }
            })
            .collect();
        let rows = (0..frame.height())
            .map(|row_index| Row {
                id: id(),
                cells: columns
                    .iter()
                    .enumerate()
                    .map(|(column_index, column)| {
                        let raw = frame
                            .columns()
                            .get(column_index)
                            .and_then(|series| {
                                polars_value_at(series.as_materialized_series(), row_index).ok()
                            })
                            .map(scalar_value_to_raw)
                            .unwrap_or_default();
                        (
                            column.id.clone(),
                            Cell {
                                raw,
                                ..Cell::default()
                            },
                        )
                    })
                    .collect(),
            })
            .collect();
        (columns, rows)
    }

    pub(crate) fn build_frame_with_types(
        name: String,
        grid: Vec<Vec<String>>,
        data_types: Vec<DataType>,
        x: f64,
        y: f64,
    ) -> (FrameObject, CanvasView) {
        let width = data_types.len().max(1);
        let headers = grid
            .first()
            .cloned()
            .unwrap_or_else(|| vec!["Column".into()]);
        let columns: Vec<Column> = (0..width)
            .map(|index| {
                let name = headers
                    .get(index)
                    .filter(|header| !header.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("Column {}", index + 1));
                Column {
                    id: column_id(&name),
                    name,
                    source_name: None,
                    data_type: data_types.get(index).copied().unwrap_or(DataType::String),
                    scale: None,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                }
            })
            .collect();
        let rows = grid
            .iter()
            .skip(1)
            .map(|values| Row {
                id: id(),
                cells: columns
                    .iter()
                    .enumerate()
                    .map(|(index, column)| {
                        (
                            column.id.clone(),
                            Cell {
                                raw: values.get(index).cloned().unwrap_or_default(),
                                ..Cell::default()
                            },
                        )
                    })
                    .collect(),
            })
            .collect();
        Self::frame_with_view(name, columns, rows, x, y)
    }

    /// The object and its card for freshly built columns and rows. The card
    /// is sized to the column count, the way every new literal frame's is.
    pub(crate) fn frame_with_view(
        name: String,
        columns: Vec<Column>,
        rows: Vec<Row>,
        x: f64,
        y: f64,
    ) -> (FrameObject, CanvasView) {
        let width = columns.len().max(1);
        let row_count = rows.len();
        let frame_id = id();
        let frame = FrameObject {
            id: frame_id.clone(),
            name,
            columns,
            rows,
            ..FrameObject::default()
        };
        let view = CanvasView {
            id: id(),
            object_id: frame_id,
            x,
            y,
            width: frame_card_width(width),
            height: frame_card_height(row_count),
            collapsed: false,
            tab_object_ids: Vec::new(),
        };
        (frame, view)
    }
}

impl FrameObject {
    /// A derived frame that reproduces this one column for column, with an
    /// empty wrangle chain and a display layer of its own.
    ///
    /// This is what a new tab is. Nothing is computed here: the child holds
    /// a projection, and the rows only exist when someone reads a page.
    ///
    /// The child's columns are its own, so it starts with an empty display
    /// layer rather than a copy of this frame's — a filter written against
    /// the parent's column ids would not resolve here.
    pub(crate) fn pass_through_child(&self, name: String) -> FrameObject {
        let mut columns = Vec::with_capacity(self.columns.len());
        let mut projections = Vec::with_capacity(self.columns.len());
        for source in &self.columns {
            let output_column_id = column_id(&source.name);
            columns.push(Column {
                id: output_column_id.clone(),
                name: source.name.clone(),
                source_name: None,
                data_type: source.data_type,
                scale: source.scale,
                categories: source.categories.clone(),
                format: source.format.clone(),
                formula: None,
            });
            projections.push(DerivedExpression {
                output_column_id,
                expression: Expr::Column {
                    column_id: source.id.clone(),
                },
            });
        }
        let column_ids = projections
            .iter()
            .map(|projection| projection.output_column_id.clone())
            .collect();
        FrameObject {
            comment: None,
            id: id(),
            name,
            columns,
            rows: Vec::new(),
            steps: Vec::new(),
            display: FrameDisplay {
                orientation: self.display.orientation,
                ..FrameDisplay::default()
            },
            base_columns: Vec::new(),
            source_file: None,
            file_origin: None,
            disconnected_read: None,
            artifact: None,
            connector: None,
            cell_overlay: Vec::new(),
            deleted_rows: Vec::new(),
            appended_rows: 0,
            prediction: None,
            derivation: Some(FrameDerivation {
                source_frame_id: self.id.clone(),
                join: None,
                steps: vec![
                    FrameStep::WithColumns {
                        columns: projections,
                    },
                    FrameStep::Select { column_ids },
                ],
            }),
            generator: None,
            entry_columns: Vec::new(),
            materialization: None,
            unique_keys: Vec::new(),
            period: None,
            summaries: Vec::new(),
        }
    }
}
