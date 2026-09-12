//! Compact document inventory. Keeping presentation here lets adding a model
//! remain a small adapter change rather than growing the tool router again.
use crate::{DocumentSummary, ObjectSummary, column_summaries, data_type_name};
use framework_core::{DataObject, DocumentView, FrameObject};
use std::path::Path;

pub(crate) fn document_summary(view: &DocumentView, path: &Path) -> DocumentSummary {
    DocumentSummary {
        id: view.document.id.clone(),
        name: view.document.name.clone(),
        revision: view.document.revision,
        document_path: path.display().to_string(),
        can_undo: view.can_undo,
        can_redo: view.can_redo,
        objects: view
            .document
            .objects
            .iter()
            .map(|object| object_summary(view, object))
            .collect(),
        formula_reference: "Search formula functions with the search_functions tool — \
                            aliases cover Excel and Polars vocabulary."
            .into(),
    }
}

fn object_summary(view: &DocumentView, object: &DataObject) -> ObjectSummary {
    let mut summary = ObjectSummary {
        id: object.id().into(),
        name: object.name().into(),
        kind: String::new(),
        value: None,
        data_type: None,
        row_count: None,
        columns: Vec::new(),
    };
    let kind = match object {
        DataObject::Value(value) => {
            summary.value = Some(value.raw.clone());
            summary.data_type = Some(data_type_name(value.data_type));
            "value"
        }
        DataObject::Result(result) => {
            let computed = view.computed_results.get(&result.id);
            summary.value = Some(
                computed
                    .map(|value| format!("= {} → {}", value.formula, value.cell.display))
                    .unwrap_or_default(),
            );
            summary.data_type = computed.map(|value| data_type_name(value.data_type));
            "result"
        }
        DataObject::Block(block) => {
            summary.value = Some(block_text(view, &block.id));
            summary.row_count = Some(block.lines.len());
            "block"
        }
        DataObject::Series(series) => {
            summary.value = Some(series.values.join(", "));
            summary.data_type = Some(data_type_name(series.data_type));
            summary.row_count = Some(series.values.len());
            "series"
        }
        DataObject::Container(container) => {
            summary.value = Some(
                container
                    .member_ids
                    .iter()
                    .filter_map(|id| view.document.object(id).ok())
                    .map(|member| member.name())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            summary.row_count = Some(container.member_ids.len());
            "container"
        }
        DataObject::Frame(frame) => {
            summary.row_count = frame_row_count(view, frame);
            summary.columns = column_summaries(view, frame);
            "frame"
        }
        DataObject::Text(text) => {
            summary.value = Some(text.text.clone());
            summary.data_type = Some("text".into());
            "text"
        }
        DataObject::Plot(_) => {
            summary.data_type = Some("vega-lite".into());
            "plot"
        }
        DataObject::CalculationMatrix(matrix) => {
            let computed = view.computed_calculation_matrices.get(&matrix.id);
            summary.value = computed.and_then(|value| value.error.clone());
            summary.row_count = computed
                .and_then(|value| value.output.as_ref())
                .map(|output| output.rows.len());
            "calculationMatrix"
        }
        DataObject::Model(model) => {
            summary.value = Some(
                if model.fitted.is_some() {
                    "Fitted"
                } else {
                    "Not fitted"
                }
                .into(),
            );
            "model"
        }
    };
    summary.kind = kind.into();
    summary
}

fn frame_row_count(view: &DocumentView, frame: &FrameObject) -> Option<usize> {
    // Literal rows are a count only for an untransformed owned frame.
    // Imported/computed frames otherwise look falsely empty to agents.
    if frame.owns_its_rows() && frame.steps.is_empty() {
        Some(frame.rows.len())
    } else {
        view.computed_frames
            .get(&frame.id)
            .and_then(|frame| frame.total_rows)
    }
}

fn block_text(view: &DocumentView, id: &str) -> String {
    view.computed_blocks
        .get(id)
        .map(|block| {
            block
                .lines
                .iter()
                .filter(|line| !line.blank)
                .map(|line| match (&line.cell.error, line.comment) {
                    (_, true) => line.text.clone(),
                    (Some(error), _) => format!("{} → {error}", line.text),
                    (None, _) => format!("{} → {}", line.text, line.cell.display),
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}
