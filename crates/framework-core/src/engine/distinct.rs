//! The distinct values of one column, with counts -- what the column-header
//! value checklist is filled from.
//!
//! Only the engine can say this honestly. A paged import's rows on screen are
//! a window onto data the interface has never seen, so a list built from them
//! would offer a fraction of the values and let the person believe it was all
//! of them.
//!
//! **Which rows are counted is the delicate part.** The checklist writes a
//! trailing Filter step into the Wrangle chain, so the next time it opens the
//! chain already *excludes* every value the person unchecked. Listing only
//! what survives that filter would make an unchecked value vanish from the
//! list and become impossible to check again. So the caller hands over the
//! chain it wants evaluated -- the saved chain with the picker's own predicate
//! taken off -- exactly the way a step sample evaluates a chain prefix, and
//! the frame's display filter and sort (which sit above the data layer) are
//! never applied: the checklist describes what the frame holds, not what a
//! tab happens to be showing.

use crate::*;
use polars::prelude as pl;
use serde::{Deserialize, Serialize};

/// One distinct value, in the shape a formula literal can be written from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DistinctScalar {
    Null,
    Boolean(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistinctValue {
    pub value: DistinctScalar,
    /// Rows holding it.
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistinctValues {
    /// Commonest first, ties broken on the value, the blank group kept.
    pub values: Vec<DistinctValue>,
    /// Distinct values in the whole column — or, under a query, the ones
    /// that matched it — so a truncated list can say so.
    pub total: usize,
}

impl Document {
    /// Distinct values of `column_id` over the rows `steps` leaves, which
    /// replaces the frame's saved chain for this read only.
    ///
    /// Dates are refused rather than listed as text: a date is not a string
    /// literal in the formula language, and a list that cannot be written
    /// back as a predicate is a list that would silently filter nothing.
    /// The caller falls back to the typed filter for them.
    ///
    /// `query` narrows the list to values whose text contains it, ignoring
    /// case, *before* the limit is taken. The checklist shows the commonest
    /// few hundred, and a name further down is exactly what a person types
    /// to find; filtering only the page already sent would answer "no such
    /// value" about one that is there. The blank group has no text, so a
    /// query never matches it.
    pub(crate) fn distinct_values(
        &self,
        frame_id: &str,
        column_id: &str,
        steps: Vec<FrameStepInput>,
        limit: usize,
        query: Option<&str>,
    ) -> Result<DistinctValues, CoreError> {
        let (walk, failure) = self.walk_pipeline(frame_id, steps, None)?;
        if let Some((_, error)) = failure {
            return Err(error);
        }
        let schema = walk
            .plan
            .clone()
            .collect_schema()
            .map_err(|error| CoreError::Import(in_plain_words(error.to_string())))?;
        let dtype = schema.get(column_id).ok_or_else(|| {
            CoreError::InvalidOperation("That column is not in the frame's current data".into())
        })?;
        let value_expr = match dtype {
            pl::DataType::Boolean => pl::col(column_id),
            pl::DataType::String => pl::col(column_id),
            // Labels, not category numbers: compared and written as text.
            pl::DataType::Categorical(..) | pl::DataType::Enum(..) => {
                pl::col(column_id).cast(pl::DataType::String)
            }
            other if other.is_primitive_numeric() || other.is_decimal() => pl::col(column_id),
            _ => {
                return Err(CoreError::InvalidOperation(
                    "Only text, number and true/false columns have a value checklist".into(),
                ));
            }
        };
        const VALUE: &str = "value";
        const COUNT: &str = "rows";
        let grouped = walk
            .plan
            .select([value_expr.alias(VALUE)])
            .group_by([pl::col(VALUE)])
            .agg([pl::len().alias(COUNT)])
            .sort_by_exprs(
                [pl::col(COUNT), pl::col(VALUE)],
                pl::SortMultipleOptions::default()
                    .with_order_descending_multi([true, false])
                    .with_nulls_last(true),
            )
            .collect()
            .map_err(|error| CoreError::Import(in_plain_words(error.to_string())))?;
        let values = grouped
            .column(VALUE)
            .map_err(|error| CoreError::Import(error.to_string()))?
            .as_materialized_series()
            .clone();
        let counts = grouped
            .column(COUNT)
            .map_err(|error| CoreError::Import(error.to_string()))?
            .as_materialized_series()
            .clone();
        let entry_at = |index: usize| -> Result<DistinctValue, CoreError> {
            let value = match polars_value_at(&values, index).map_err(CoreError::Import)? {
                ScalarValue::Null => DistinctScalar::Null,
                ScalarValue::Boolean(value) => DistinctScalar::Boolean(value),
                ScalarValue::Number(value) => DistinctScalar::Number(value),
                ScalarValue::String(value) => DistinctScalar::Text(value),
                ScalarValue::Date(_) => unreachable!("dates were refused above"),
            };
            let count = match polars_value_at(&counts, index).map_err(CoreError::Import)? {
                ScalarValue::Number(count) => count as usize,
                _ => 0,
            };
            Ok(DistinctValue { value, count })
        };
        let needle = query
            .map(|query| query.trim().to_lowercase())
            .filter(|needle| !needle.is_empty());
        let Some(needle) = needle else {
            let total = grouped.height();
            let values = (0..total.min(limit))
                .map(entry_at)
                .collect::<Result<Vec<_>, CoreError>>()?;
            return Ok(DistinctValues { values, total });
        };
        // Every group is read to count the matches; the grouped frame is one
        // row per distinct value, so this is the size of the answer to "how
        // many match", not of the column.
        let mut matched = Vec::new();
        let mut total = 0;
        for index in 0..grouped.height() {
            let entry = entry_at(index)?;
            if entry
                .value
                .text()
                .is_some_and(|text| text.to_lowercase().contains(&needle))
            {
                total += 1;
                if matched.len() < limit {
                    matched.push(entry);
                }
            }
        }
        Ok(DistinctValues {
            values: matched,
            total,
        })
    }
}

impl DistinctScalar {
    /// The text a query is matched against: what the checklist shows for
    /// the value (`String(value)` in `useValueFilterPicker`), which for a
    /// whole number is written without a fraction. The blank group has none.
    fn text(&self) -> Option<String> {
        match self {
            DistinctScalar::Null => None,
            DistinctScalar::Boolean(value) => Some(value.to_string()),
            DistinctScalar::Number(value) => Some(value.to_string()),
            DistinctScalar::Text(value) => Some(value.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use crate::*;

    /// Every spelling `valueFilterPredicate` (src/lib/valueFilter.ts) emits.
    /// A client-authored chain formula that the engine would write back
    /// differently makes the save re-seed the chain and sever sync sessions,
    /// so each must parse and re-render to the identical string. Keep this
    /// list in step with `src/lib/valueFilter.test.ts`.
    #[test]
    fn the_pickers_predicate_spellings_are_the_engines_own() {
        let store = demo_store();
        let document = &store.document;
        let frame = document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "Products" => Some(frame),
                _ => None,
            })
            .unwrap();
        for spelling in [
            r#"`Category`.is_in(["Accessories", "Home office"])"#,
            r#"`Category`.is_in(["say \"hi\"", "back\\slash", "café"])"#,
            "`Category`.is_in([1, -2, 30])",
            "`Category`.is_in([1.5, -0.25, 19.99])",
            "`Category`.is_in([True, False])",
            "`Category`.is_null()",
            r#"`Category`.is_in(["a", "b"]) | `Category`.is_null()"#,
            "`Category`.is_in([1, 2.5]) | `Category`.is_null()",
            "`Category`.is_in([True]) | `Category`.is_null()",
        ] {
            let expression = document
                .parse_formula_for_frame(&frame.id, spelling)
                .unwrap_or_else(|error| panic!("{spelling} did not parse: {error}"));
            assert_eq!(
                expression.render(frame, document, 0),
                spelling,
                "the engine writes {spelling} back differently"
            );
        }
    }
}
