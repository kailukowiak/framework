//! Read-only dependency checks that keep object deletion from leaving dangling references.

use crate::*;

pub(super) fn deletion_refusal(
    document: &Document,
    object_index: usize,
    object_id: &Id,
) -> Option<String> {
    let going = as_named(document.objects[object_index].name());
    let referenced_by = formula_refusal(document, object_index, object_id, &going);
    let derived_from = document.objects.iter().find_map(|object| match object {
        DataObject::Model(model)
            if model
                .spec
                .as_ref()
                .is_some_and(|spec| spec.source_frame_id == *object_id)
                || model
                    .imported_input
                    .as_ref()
                    .is_some_and(|input| input.source_frame_id == *object_id) =>
        {
            Some(format!(
                "{} uses {going}. Change its input before deleting this frame.",
                as_named(&model.name)
            ))
        }
        // A union in a source frame's own chain reads the stacked frame
        // without any derivation existing, so both step lists answer
        // for "built from", not just the derivation.
        DataObject::Frame(frame) => (frame
            .prediction
            .as_ref()
            .is_some_and(|prediction| prediction.model_id == *object_id)
            || frame
                .derivation
                .as_ref()
                .is_some_and(|derivation| derivation.references_frame(object_id))
            || frame
                .steps
                .iter()
                .filter_map(FrameStep::lookup_frame_id)
                .any(|lookup_id| lookup_id == object_id))
        .then(|| {
            format!(
                "{} is built from {going}, so it cannot be deleted. Delete that frame first.",
                as_named(&frame.name)
            )
        }),
        _ => None,
    });
    let drawn_from = document.objects.iter().find_map(|object| match object {
        DataObject::Plot(plot) if plot.source_frame_id == *object_id => Some(format!(
            "{} is drawn from {going}, so it cannot be deleted. Delete that plot first.",
            as_named(&plot.name)
        )),
        _ => None,
    });
    let read_across = matches!(&document.objects[object_index], DataObject::Frame(_))
        .then(|| document.frame_read_by(object_id))
        .flatten()
        .map(|reader| {
            format!("{reader} reads {going}, so it cannot be deleted. Change the formula that reads it first.")
        });
    referenced_by
        .or(derived_from)
        .or(drawn_from)
        .or(read_across)
}

fn formula_refusal(
    document: &Document,
    object_index: usize,
    object_id: &Id,
    going: &str,
) -> Option<String> {
    // A value, a result, a list, and a block's lines are all read by id
    // from a formula, so all are held in place by one being written —
    // whether the formula sits in a frame, a result, or a block. A block
    // brings every line id it holds: deleting the card is deleting the
    // lines, and a formula holds a line, never the block itself.
    let referenced_ids: Vec<&str> = match &document.objects[object_index] {
        DataObject::Value(_) | DataObject::Result(_) | DataObject::Series(_) => {
            vec![object_id.as_str()]
        }
        DataObject::Block(block) => block.lines.iter().map(|line| line.id.as_str()).collect(),
        _ => Vec::new(),
    };
    referenced_ids
        .iter()
        .find_map(|target| {
            document.objects.iter().find_map(|object| {
                // The object being deleted does not hold itself in place:
                // a block's lines reading each other go out together.
                if object.id() == object_id {
                    return None;
                }
                match object {
                    DataObject::Frame(frame) => (frame.references_object(target)
                        || frame.display.references_object(target))
                    .then(|| as_named(&frame.name)),
                    DataObject::Result(result) => result
                        .formula
                        .expression
                        .references_object(target)
                        .then(|| as_named(&result.name)),
                    DataObject::Block(block) => block.lines.iter().find_map(|line| {
                        line.expression()?
                            .references_object(target)
                            .then(|| as_line_named(block, line))
                    }),
                    _ => None,
                }
            })
        })
        .map(|reader| {
            format!("{reader} reads {going}, so it cannot be deleted. Change the formula that reads it first.")
        })
}
