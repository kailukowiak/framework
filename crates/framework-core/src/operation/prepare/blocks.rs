//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to line IDs, and
//! every precondition checked before anything is applied.
//!
//! Formula blocks: the ordered scratchpad and the lines in it.
//!
//! A block is edited as one piece of text rather than a line at a time,
//! because that is what a scratchpad is — you type down a page, and the
//! lines are wherever the newlines fell. So there is one operation here that
//! takes the whole text and works out what changed, rather than four that
//! each know about one line.

use crate::formula::line::{ParsedLine, split_line};
use crate::*;

impl Document {
    pub(crate) fn prepare_add_block(
        &self,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Block(BlockObject {
                id: object_id.clone(),
                name,
                // Empty, and waiting. A scratchpad that opened with `line_1
                // = 0` in it would be asking to be cleared before it could
                // be used.
                lines: Vec::new(),
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 340.0,
                height: 220.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id: None,
        })
    }

    /// The block retyped: its whole text in, its whole list of lines out.
    ///
    /// Nothing here refuses a formula. A line that does not parse is stored
    /// with the complaint it earned, to be shown in its own gutter — see
    /// [`BlockLine`] for why a scratchpad is the one surface in this
    /// document model that stores text it could not read.
    ///
    /// The one refusal left is a line disappearing out from under something
    /// *outside* the block that reads it. Inside the block a broken
    /// reference is visible in the next gutter down and fixed by typing;
    /// from a frame column three cards away it is invisible, so it is worth
    /// stopping for.
    pub(crate) fn prepare_set_block_source(
        &self,
        block_id: Id,
        source: String,
        editing: Option<usize>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let block = self.block(&block_id)?;
        let lines = self.rebuild_lines(block, &source, editing);
        if let Some((line, reader)) = self.first_outside_reference_lost(block, &lines) {
            return Err(CoreError::Formula(format!(
                "‘{}’ reads ‘{}’, so this line cannot be taken away. \
                 Change the formula that reads it first.",
                reader, line
            )));
        }
        Ok(ReplicatedOperation::SetBlockLines { block_id, lines })
    }

    /// Splits the text into lines and gives each one an identity.
    ///
    /// Identity is the whole difficulty. References travel by id, so a line
    /// that survives an edit has to come back with the id it had, or every
    /// formula reading it breaks. Three passes, each a weaker claim than the
    /// last: a line keeps its id if its name is unchanged, else if its
    /// expression is unchanged (which is what a rename looks like), else if
    /// nothing else has claimed the line that was in its place.
    fn rebuild_lines(
        &self,
        block: &BlockObject,
        source: &str,
        editing: Option<usize>,
    ) -> Vec<BlockLine> {
        let logical = crate::formula::line::logical_lines(source);
        let split: Vec<ParsedLine> = logical.iter().map(|line| split_line(line)).collect();
        let mut claimed = vec![false; block.lines.len()];
        let mut taken: Vec<Option<usize>> = vec![None; split.len()];

        let claim = |index: usize,
                     taken: &mut Vec<Option<usize>>,
                     claimed: &mut Vec<bool>,
                     matches: &dyn Fn(&BlockLine) -> bool| {
            if taken[index].is_some() {
                return;
            }
            if let Some(found) = block
                .lines
                .iter()
                .position(|line| !claimed[block_index(block, line)] && matches(line))
            {
                claimed[found] = true;
                taken[index] = Some(found);
            }
        };

        for (index, parsed) in split.iter().enumerate() {
            let Some(name) = parsed.name.as_deref() else {
                continue;
            };
            claim(index, &mut taken, &mut claimed, &|line| {
                line.named
                    && crate::engine::values::normalize_name(&line.name)
                        == crate::engine::values::normalize_name(name)
            });
        }
        for (index, parsed) in split.iter().enumerate() {
            if parsed.source.is_empty() {
                continue;
            }
            claim(index, &mut taken, &mut claimed, &|line| {
                line.source == parsed.source
            });
        }
        for index in 0..split.len() {
            claim(index, &mut taken, &mut claimed, &|line| {
                block_index(block, line) == index
            });
        }

        // Names, before any parsing: a line resolving `x` has to find the
        // `x` this same edit is establishing.
        let mut drafted: Vec<BlockLine> = Vec::with_capacity(split.len());
        for (index, parsed) in split.iter().enumerate() {
            let previous = taken[index].map(|found| &block.lines[found]);
            let expression = !parsed.source.is_empty() && !parsed.source.starts_with('#');
            // The cursor is on this line, so what is to the left of its `=`
            // is halfway to somewhere. The name it already answers to is
            // held until the cursor leaves: `revenue` on the way to
            // `revenue10` is a line called `r`, then `re`, then `rev`, and
            // renaming it three times renames it three times everywhere it
            // is read. What the author has typed is on their screen, which
            // is the only place it needs to be until they have finished.
            let held = (editing == Some(index))
                .then_some(previous)
                .flatten()
                .filter(|line| !line.name.is_empty());
            drafted.push(draft_block_line(parsed, previous, held, expression));
        }
        let mut ordinal = 1;
        for index in 0..drafted.len() {
            if drafted[index].name.is_empty() && drafted[index].is_expression() {
                let name = loop {
                    let candidate = format!("line_{ordinal}");
                    ordinal += 1;
                    if !drafted.iter().any(|line| line.name == candidate) {
                        break candidate;
                    }
                };
                drafted[index].name = name;
            }
        }

        // Now the formulas, each against the finished shape of the block.
        let scope = BlockObject {
            id: block.id.clone(),
            name: block.name.clone(),
            lines: drafted.clone(),
        };

        // Renaming a line is free everywhere else in this document, because
        // references hold ids and are written back out under whatever name
        // the thing has now. A block is the one place a reference is also
        // *text somebody typed*, so it has to be rewritten to stay true:
        // rename `x` to `price` and the line below that reads it says
        // `price` from then on.
        //
        // Only lines the author left alone are rewritten. A line they are
        // in the middle of typing is theirs, and having it change under the
        // cursor would be worse than a name that has gone stale. A name still
        // being typed is not in this list either, having been held above.
        let renamed: Vec<&Id> = taken
            .iter()
            .enumerate()
            .filter_map(|(index, found)| {
                let previous = &block.lines[(*found)?];
                (previous.name != drafted[index].name).then_some(&previous.id)
            })
            .collect();
        if !renamed.is_empty() {
            for (index, found) in taken.iter().enumerate() {
                let Some(previous) = found.map(|found| &block.lines[found]) else {
                    continue;
                };
                if previous.source != drafted[index].source {
                    continue;
                }
                let Some(expression) = previous.expression() else {
                    continue;
                };
                let mut reads_renamed = false;
                expression.walk_values(&mut |object_id| {
                    reads_renamed = reads_renamed || renamed.iter().any(|id| *id == object_id);
                });
                if reads_renamed {
                    drafted[index].source =
                        expression.render_in_scope(&FrameObject::default(), self, Some(&scope), 0);
                }
            }
        }
        let scope = BlockObject {
            lines: drafted.clone(),
            ..scope
        };
        for line in &mut drafted {
            if !line.is_expression() {
                continue;
            }
            match self.parse_formula_in_draft_block(&scope, &line.source) {
                Ok(expression) => line.formula = Some(Formula { expression }),
                Err(error) => line.error = Some(error.to_string()),
            }
        }

        // The order rule, applied rather than refused. A line reading one
        // below it is kept — it is on the screen, and the person typing can
        // see what they meant — but it does not run, so the block stays a
        // calculation that reads top to bottom.
        let below: Vec<(usize, String)> = drafted
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let mut found = None;
                line.expression()?.walk_values(&mut |object_id| {
                    if found.is_none()
                        && let Some(other) = drafted.iter().position(|line| line.id == *object_id)
                        && other >= index
                    {
                        found = Some(drafted[other].name.clone());
                    }
                });
                found.map(|name| (index, name))
            })
            .collect();
        for (index, name) in below {
            drafted[index].formula = None;
            drafted[index].error = Some(if name == drafted[index].name {
                format!("‘{name}’ would be defined in terms of itself.")
            } else {
                format!(
                    "‘{name}’ is written below this line, and a line may only read the \
                     lines above it."
                )
            });
        }

        // And the loop the order rule cannot see: out through a result or
        // another block and back again. Also kept and not run — this is what
        // lets compilation inline a line's formula without watching its own
        // feet.
        let circular: Vec<usize> = drafted
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                line.expression().is_some_and(|expression| {
                    self.draft_formula_reaches_object(expression, &line.id, (&block.id, &drafted))
                })
            })
            .map(|(index, _)| index)
            .collect();
        for index in circular {
            let name = drafted[index].name.clone();
            drafted[index].formula = None;
            drafted[index].error = Some(format!("‘{name}’ would be defined in terms of itself."));
        }
        drafted
    }

    /// The first line this edit takes away that something outside the block
    /// still reads, and the name of what reads it.
    fn first_outside_reference_lost(
        &self,
        block: &BlockObject,
        lines: &[BlockLine],
    ) -> Option<(String, String)> {
        for gone in block
            .lines
            .iter()
            .filter(|line| !lines.iter().any(|kept| kept.id == line.id))
        {
            for object in &self.objects {
                let reads = match object {
                    DataObject::Frame(frame) => {
                        frame.references_object(&gone.id)
                            || frame.display.references_object(&gone.id)
                    }
                    DataObject::Result(result) => {
                        result.formula.expression.references_object(&gone.id)
                    }
                    DataObject::Block(other) => {
                        other.id != block.id
                            && other.lines.iter().any(|line| {
                                line.formula.as_ref().is_some_and(|formula| {
                                    formula.expression.references_object(&gone.id)
                                })
                            })
                    }
                    _ => false,
                };
                if reads {
                    return Some((gone.name.clone(), object.name().to_string()));
                }
            }
        }
        None
    }
}

fn draft_block_line(
    parsed: &ParsedLine<'_>,
    previous: Option<&BlockLine>,
    held: Option<&BlockLine>,
    expression: bool,
) -> BlockLine {
    BlockLine {
        id: previous.map_or_else(id, |line| line.id.clone()),
        name: match (expression, parsed.name.as_deref(), held) {
            (false, _, _) => String::new(),
            (true, _, Some(line)) => line.name.clone(),
            (true, Some(name), None) => name.to_string(),
            // An unnamed line keeps the automatic name it already answered
            // to, so a reference from elsewhere survives the line above it
            // being deleted.
            (true, None, None) => previous
                .filter(|line| !line.named && !line.name.is_empty())
                .map(|line| line.name.clone())
                .unwrap_or_default(),
        },
        named: match held {
            Some(line) => expression && line.named,
            None => expression && parsed.name.is_some(),
        },
        // Preserve whether the declaration itself used backticks. The
        // stored name is deliberately unescaped for lookup, while this bit
        // keeps the author's source valid when the block is rendered again.
        name_quoted: match held {
            Some(line) => expression && line.named && line.name_quoted,
            None => expression && parsed.name_quoted,
        },
        source: parsed.source.to_string(),
        formula: None,
        error: None,
    }
}

/// Where a line sits in its block. The rebuild works in indices, and this
/// keeps the borrow checker out of the way of asking for one.
fn block_index(block: &BlockObject, line: &BlockLine) -> usize {
    block
        .lines
        .iter()
        .position(|candidate| candidate.id == line.id)
        .expect("the line came from this block")
}
