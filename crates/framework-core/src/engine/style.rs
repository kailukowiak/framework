//! Running a frame's conditional-formatting rules.
//!
//! A rule is an ordinary row-wise formula, so it compiles to exactly what a
//! calculated column compiles to — and it is run the same way, as one
//! `with_columns`. Every rule becomes a hidden column named for its id, all
//! of them in a single plan, so Polars optimizes them together and reads each
//! source column once.
//!
//! Hidden is the whole point: the columns are read off the collected frame
//! and dropped there. They never enter the frame's schema, never become
//! analytical columns, and nothing downstream can read them.
//!
//! **The columns belong in the plan that produces all the rows, not in the
//! page.** A rule may ask something of the whole column — `x > x.mean()`, or
//! the ends of a ramp — and an answer computed over a thousand visible rows
//! would be a different answer on every page. So the caller that pages adds
//! these expressions above its slice and lets Polars push the slice through
//! the elementwise ones; the callers that already hold every row run them
//! over that. See `get_frame_page`.
//!
//! What comes back is style rather than values. A rule's answer means
//! something different depending on what it returns — a flag picks rows, a
//! label sorts them into cases, a number places them along a ramp — and
//! resolving that here keeps one reading of a rule instead of one per
//! surface that draws it.

use crate::*;
use polars::prelude as pl;
use polars::prelude::IntoLazy;
use std::collections::{HashMap, HashSet};

/// Prefix for a rule's hidden column. Not a column id, so it cannot collide
/// with one — the same guarantee `ROW_INDEX` relies on.
const RULE_MASK: &str = "__framework_rule:";

fn mask_name(rule_id: &str) -> String {
    format!("{RULE_MASK}{rule_id}")
}

/// What the rules said about each row, and which rules could not be asked.
#[derive(Debug, Default, Clone)]
pub(crate) struct StyleRuleMatches {
    /// One entry per row, in the order the rows were given: what each rule
    /// with something to say about that row said, in the frame's own rule
    /// order. Later rules win when they are merged, so that order is
    /// load-bearing.
    pub(crate) rows: Vec<Vec<FrameStyleMatch>>,
    /// Rules that could not be run, by id. A rule that breaks reports itself
    /// and is skipped; every other rule, and the frame, still renders.
    pub(crate) errors: HashMap<Id, String>,
}

impl StyleRuleMatches {
    /// The answers for one page of the rows these were computed over.
    pub(crate) fn slice(mut self, offset: usize, limit: usize) -> Self {
        self.rows = self
            .rows
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        self
    }
}

impl FrameObject {
    /// This frame's rules as hidden columns, ready to add to a plan.
    ///
    /// Returned with the ids they belong to rather than parsed back out of
    /// column names, and with the rules that would not compile named in the
    /// error map instead of silently missing.
    pub(crate) fn style_rule_columns(
        &self,
        document: &Document,
    ) -> (Vec<pl::Expr>, HashMap<Id, String>) {
        let mut errors = HashMap::new();
        let mut columns = Vec::with_capacity(self.display.style_rules.len());
        for rule in &self.display.style_rules {
            match rule_column(rule, document) {
                Ok(column) => columns.push(column),
                Err(error) => {
                    errors.insert(rule.id.clone(), in_plain_words(error));
                }
            }
        }
        (columns, errors)
    }

    /// What this frame's rules make of each row of `data_frame`, computing
    /// the hidden columns over exactly those rows.
    ///
    /// For a caller holding every row — a frame that keeps its own, or a
    /// sorted read that had to materialize the lot anyway — that is the
    /// whole column, so an aggregate inside a rule means what it says.
    pub(crate) fn evaluate_style_rules(
        &self,
        document: &Document,
        data_frame: &pl::DataFrame,
    ) -> StyleRuleMatches {
        if self.display.style_rules.is_empty() {
            return StyleRuleMatches::default();
        }
        let (columns, mut errors) = self.style_rule_columns(document);
        let evaluated = match with_rule_columns(data_frame, &columns) {
            Ok(evaluated) => evaluated,
            // One broken rule fails the batch, and the batch cannot say
            // which. Running them one at a time is the only way to name it,
            // and only happens on a read that was already failing.
            Err(_) => {
                let mut kept = Vec::new();
                for (rule, column) in self.display.style_rules.iter().zip(columns) {
                    if errors.contains_key(&rule.id) {
                        continue;
                    }
                    match with_rule_columns(data_frame, std::slice::from_ref(&column)) {
                        Ok(_) => kept.push(column),
                        Err(error) => {
                            errors.insert(rule.id.clone(), error);
                        }
                    }
                }
                match with_rule_columns(data_frame, &kept) {
                    Ok(evaluated) => evaluated,
                    Err(_) => {
                        return StyleRuleMatches {
                            rows: vec![Vec::new(); data_frame.height()],
                            errors,
                        };
                    }
                }
            }
        };
        let mut matches = self.read_style_rules(&evaluated);
        matches.errors.extend(errors);
        matches
    }

    /// The same reading, for a frame whose hidden columns are already there
    /// because the plan that produced it carried them.
    pub(crate) fn read_style_rules(&self, data_frame: &pl::DataFrame) -> StyleRuleMatches {
        let mut errors = HashMap::new();
        let mut rows = vec![Vec::new(); data_frame.height()];
        for rule in &self.display.style_rules {
            let Ok(column) = data_frame.column(&mask_name(&rule.id)) else {
                continue;
            };
            match read_rule(rule, column.as_materialized_series()) {
                Ok(styles) => {
                    for (row_index, style) in styles.into_iter().enumerate() {
                        let Some(style) = style else { continue };
                        rows[row_index].push(FrameStyleMatch {
                            rule_id: rule.id.clone(),
                            style,
                        });
                    }
                }
                Err(error) => {
                    errors.insert(rule.id.clone(), error);
                }
            }
        }
        StyleRuleMatches { rows, errors }
    }
}

/// One rule as the hidden column it computes.
///
/// All three readings want the formula's own answer, which is why there is
/// almost nothing here: a flag, a label, or a position between 0 and 1. A
/// scale used to do arithmetic at this point, working out where each row sat
/// between ends the rule carried; the ends live in the formula now, so the
/// column it needs is the column it was given.
///
/// The aggregates that arithmetic needed have not gone anywhere — they are
/// inside `.normalize()`, and they still work for the same reason `x >
/// x.mean()` does: this column goes into the plan above the page slice, so
/// `min` and `max` see every row rather than the visible thousand.
fn rule_column(rule: &FrameStyleRule, document: &Document) -> Result<pl::Expr, String> {
    let expression = rule.formula.expression.to_polars(document)?;
    let column = match &rule.output {
        FrameStyleOutput::Scale { .. } => {
            // The formula is the position. Nothing is computed here because
            // there is nothing left to compute: where a row sits between the
            // ends is what the formula was written to answer, which is what
            // makes pinning, clipping, log scales and substituting a value
            // from another column edits to a formula rather than settings
            // that would each have needed a control.
            //
            // Float because a ramp over a whole-numbered column is a
            // fraction of the way along it, and integer arithmetic would
            // floor every row to one end or the other.
            expression.cast(pl::DataType::Float64)
        }
        _ => expression,
    };
    Ok(column.alias(mask_name(&rule.id)))
}

/// The hidden columns added to rows already in hand.
fn with_rule_columns(
    data_frame: &pl::DataFrame,
    columns: &[pl::Expr],
) -> Result<pl::DataFrame, String> {
    data_frame
        .clone()
        .lazy()
        .with_columns(columns)
        .collect()
        .map_err(|error| in_plain_words(error.to_string()))
}

/// One rule's hidden column, read as one style per row — `None` wherever the
/// rule has nothing to say about that row.
///
/// A rule is typed when it is set, so a column that no longer fits its
/// reading is a schema that moved under it rather than a rule that was ever
/// allowed. It says so and styles nothing.
fn read_rule(
    rule: &FrameStyleRule,
    series: &pl::Series,
) -> Result<Vec<Option<FrameCellStyle>>, String> {
    match &rule.output {
        FrameStyleOutput::Condition { style } => {
            let flags = series
                .bool()
                .map_err(|_| "This rule needs a formula that produces true or false".to_string())?;
            Ok(flags
                .iter()
                // Null is not a match. A rule over a column with gaps in it
                // styles the rows it can answer for and leaves the rest.
                .map(|matched| (matched == Some(true)).then(|| style.clone()))
                .collect())
        }
        FrameStyleOutput::Category { cases, other } => Ok(read_labels(series)?
            .into_iter()
            .map(|label| {
                let label = label?;
                cases
                    .iter()
                    .find(|case| case.value == label)
                    .map(|case| case.style.clone())
                    .or_else(|| other.clone())
            })
            .collect()),
        FrameStyleOutput::Scale { scale } => Ok(read_numbers(series)?
            .into_iter()
            .map(|position| {
                // The formula's own answer, clamped: a formula that maps
                // its column onto a narrower range than the data covers is
                // a deliberate choice to flatten the ends. `.normalize(0,
                // 100)` over a column reaching 140 means "everything past a
                // hundred is the top", not an error.
                let position = position?.clamp(0.0, 1.0);
                // A middle color makes the ramp two ramps meeting at 0.5,
                // and the formula is what puts a number there.
                let color = |scale: &FrameStyleColorScale| match &scale.mid {
                    Some(mid) if position <= 0.5 => mix_colors(&scale.low, mid, position * 2.0),
                    Some(mid) => mix_colors(mid, &scale.high, (position - 0.5) * 2.0),
                    None => mix_colors(&scale.low, &scale.high, position),
                };
                Some(FrameCellStyle {
                    text_color: scale.text.as_ref().and_then(color),
                    fill_color: scale.fill.as_ref().and_then(color),
                    ..FrameCellStyle::default()
                })
            })
            .collect()),
    }
}

fn read_labels(series: &pl::Series) -> Result<Vec<Option<String>>, String> {
    (0..series.len())
        .map(|index| match polars_value_at(series, index)? {
            ScalarValue::Null => Ok(None),
            ScalarValue::String(value) => Ok(Some(value)),
            _ => Err("This rule needs a formula that produces text".into()),
        })
        .collect()
}

fn read_numbers(series: &pl::Series) -> Result<Vec<Option<f64>>, String> {
    (0..series.len())
        .map(|index| match polars_value_at(series, index)? {
            ScalarValue::Null => Ok(None),
            // Infinities and NaN have no place on a ramp; they are a gap in
            // it rather than an end of it.
            ScalarValue::Number(value) => Ok(value.is_finite().then_some(value)),
            _ => Err("This rule needs a formula that produces a number".into()),
        })
        .collect()
}

/// `position` of the way from one color to the other, channel by channel.
/// Both ends were checked when the rule was set, so failing to read one is a
/// document somebody edited by hand.
fn mix_colors(low: &str, high: &str, position: f64) -> Option<String> {
    let channels = |color: &str| {
        let color = color.strip_prefix('#')?;
        (color.len() == 6).then_some(())?;
        Some([
            u8::from_str_radix(&color[0..2], 16).ok()?,
            u8::from_str_radix(&color[2..4], 16).ok()?,
            u8::from_str_radix(&color[4..6], 16).ok()?,
        ])
    };
    let (low, high) = (channels(low)?, channels(high)?);
    let mixed = low
        .iter()
        .zip(high)
        .map(|(low, high)| {
            let value = *low as f64 + (high as f64 - *low as f64) * position;
            format!("{:02x}", value.round().clamp(0.0, 255.0) as u8)
        })
        .collect::<Vec<_>>();
    Some(format!("#{}", mixed.concat()))
}

impl Document {
    /// The distinct labels a rule's formula produces over this frame,
    /// commonest first — what a case list is filled from.
    ///
    /// Only the engine can answer this, which is the whole reason it is a
    /// call rather than something the panel works out: the formula may be an
    /// expression over several columns, the rows may live in a Parquet file
    /// the interface has never seen, and the display filter decides which of
    /// them count. So the panel asks, and dresses the answer.
    ///
    /// Commonest first because a cap has to drop something, and the values
    /// worth a color are the ones with rows behind them. Ties break on the
    /// label so two people filling the same rule get the same list in the
    /// same order — a case list is document state, and document state settled
    /// by hash iteration order is a merge conflict waiting to happen. Nulls
    /// are not a value: a row with no label is what "anything else" is for.
    pub(crate) fn frame_formula_values(
        &self,
        frame_id: &str,
        formula: &str,
        limit: usize,
    ) -> Result<Vec<String>, CoreError> {
        let expression = self.prepare_formula_for_frame(frame_id, formula)?;
        let frame = self.frame(frame_id)?;
        let data_type = frame
            .infer_polars_expression_type(self, &expression)
            .map_err(CoreError::Formula)?;
        if !matches!(data_type, DataType::String | DataType::Categorical) {
            return Err(CoreError::InvalidOperation(
                "Only a formula that produces text sorts rows into named values".into(),
            ));
        }
        const VALUE: &str = "value";
        const COUNT: &str = "rows";
        let counted = self
            .materialize_frame_lazy(frame_id, Layer::Display, &mut HashSet::new())
            .map_err(CoreError::Import)?
            .select([expression
                .to_polars(self)
                .map_err(CoreError::Formula)?
                // Cast rather than trust: a categorical column's labels are
                // text everywhere this list is used -- compared against
                // `FrameStyleCase::value`, typed back into the panel -- and
                // reading them out as anything else would compare unequal to
                // the rule that was written from them.
                .cast(pl::DataType::String)
                .alias(VALUE)])
            .group_by([pl::col(VALUE)])
            .agg([pl::len().alias(COUNT)])
            .sort_by_exprs(
                [pl::col(COUNT), pl::col(VALUE)],
                pl::SortMultipleOptions::default()
                    .with_order_descending_multi([true, false])
                    .with_nulls_last(true),
            )
            .limit(limit as u32)
            .collect()
            .map_err(|error| CoreError::Import(in_plain_words(error.to_string())))?;
        let values = counted
            .column(VALUE)
            .map_err(|error| CoreError::Import(error.to_string()))?;
        let values = values.as_materialized_series();
        Ok((0..values.len())
            .filter_map(|index| match polars_value_at(values, index) {
                Ok(ScalarValue::String(value)) => Some(value),
                _ => None,
            })
            .collect())
    }
}
