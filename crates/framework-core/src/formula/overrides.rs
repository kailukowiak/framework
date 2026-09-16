//! Evaluate under overrides: `under` and `solve`.
//!
//! Everything a finance person asks for after the model works — a table of
//! scenarios side by side, a grid that varies two assumptions, a price that
//! hits a target — is one question asked repeatedly: *what would this
//! result be if that assumption held a different number*, without the
//! document holding it. The document can only be in one scenario at a
//! time, and switching it is an edit that moves every card. So the answer
//! is a private evaluation: a copy of the document with the overrides
//! written into it, computed through the ordinary compiler, and thrown
//! away. Nothing is materialized, nothing lands in history, no other reader
//! sees a temporary value, and `active_scenario` never changes. The
//! Calculation Matrix's sensitivity mode already does exactly this per
//! cell; `under` and `solve` are the same act with a formula's face on it.
//!
//! `under(scenario, expr)` answers `expr` as the named scenario would read
//! it. `solve(expr == target, by=value, within=[low, high])` searches the
//! bracket for the value that makes the two sides meet, one variable at a
//! time, and refuses when the bracket does not contain a crossing rather
//! than returning the nearest miss. Both compile to a literal: the copy is
//! evaluated at plan time and only the answer reaches Polars, which is what
//! lets `under(…)` sit inside any arithmetic, column or line.
//!
//! The written form of `under` takes a scenario. The map form the plan
//! describes — overriding named values inline — is what `solve` and the
//! matrix use internally through [`Overrides::Values`], so there is one
//! evaluation path; it waits for the formula language to have a map literal
//! before it gets a spelling.
//!
//! Cost is reported rather than hidden: a copy of the document per
//! evaluation, and one evaluation per bisection step for `solve`. The one
//! economy taken is the plan's "overriding a value that nothing reads costs
//! nothing": when the overridden values cannot reach the expression, the
//! copy is skipped and the expression is read as it stands.

use crate::*;
use polars::prelude as pl;
use std::collections::{BTreeMap, HashSet};

use super::ast::{BinaryOperator, keyword_argument};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the scenario `scenario_id` is called now — the only place a rename
/// has to reach, because formulas hold the id. `#REF` for an id no scenario
/// answers to, the way a lost column renders.
pub(crate) fn scenario_name<'a>(document: &'a Document, scenario_id: &str) -> &'a str {
    document
        .scenario(scenario_id)
        .map(|scenario| scenario.name.as_str())
        .unwrap_or("#REF")
}

/// The one shape every evaluation-under-overrides takes: a scenario the copy
/// activates, or a map of value ids to the raw text their cards would hold.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Overrides {
    Scenario(Id),
    Values(BTreeMap<Id, String>),
}

/// What a top-level `solve` line reports beside its answer, so the gutter
/// can say how the number was reached and offer to make it real.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SolveReport {
    /// The value object the search varied.
    pub target_id: Id,
    pub target_name: String,
    pub answer: f64,
    /// The answer as `SetValue` should receive it: rounded to twelve
    /// significant digits, so applying `138.75` writes `138.75` and not the
    /// last bit of bisection noise.
    pub answer_raw: String,
    pub iterations: usize,
    /// The difference between the two sides at the answer.
    pub residual: f64,
    pub low: f64,
    pub high: f64,
    /// Model evaluations spent, each a private copy of the document.
    pub evaluations: usize,
}

const MAX_BISECTIONS: usize = 200;

impl Document {
    /// `expression` as it reads with `overrides` in force: a private copy of
    /// the document, evaluated through the ordinary Scratchwork path, so
    /// live frames, sibling lines and results all follow the override.
    pub(crate) fn evaluate_under(
        &self,
        overrides: &Overrides,
        expression: &Expr,
    ) -> Result<(DataType, pl::Series), String> {
        if !self.overrides_reach(overrides, expression) {
            return self.evaluate_scratchwork_series(expression);
        }
        let evaluation = self.document_under(overrides)?;
        evaluation.evaluate_scratchwork_series(expression)
    }

    /// The copy: the same document with the overrides written in. A
    /// scenario is activated on the copy; a value map is written straight
    /// onto the cards, with any scenario override for those ids removed so
    /// the map wins whatever the copy is reading through.
    fn document_under(&self, overrides: &Overrides) -> Result<Document, String> {
        let mut evaluation = self.clone();
        match overrides {
            Overrides::Scenario(scenario_id) => {
                if self.scenario(scenario_id).is_none() {
                    return Err("That scenario is no longer in this document.".into());
                }
                evaluation.active_scenario = Some(scenario_id.clone());
            }
            Overrides::Values(values) => {
                for (value_id, raw) in values {
                    let DataObject::Value(value) =
                        evaluation.object_mut(value_id).map_err(|e| e.to_string())?
                    else {
                        return Err("Only a value can be overridden".into());
                    };
                    // A whole-number card takes a fractional trial on the
                    // copy: `solve` bisects a continuous input, and the card
                    // is retyped only here, never on the document. Applying
                    // the answer goes through `SetValue`, which re-infers
                    // the type from what is written, so `138.75` lands as a
                    // number there too.
                    if value.data_type == DataType::Integer
                        && parse_scalar_value(raw, DataType::Integer).is_err()
                    {
                        parse_scalar_value(raw, DataType::Number)?;
                        value.data_type = DataType::Number;
                    } else {
                        parse_scalar_value(raw, value.data_type)?;
                    }
                    value.raw = raw.clone();
                    for scenario in &mut evaluation.scenarios {
                        scenario.values.remove(value_id);
                    }
                }
            }
        }
        Ok(evaluation)
    }

    /// Whether any overridden value can reach `expression`. A value the
    /// expression never reads, directly or through the lines, results and
    /// frames it does read, cannot change the answer, so the copy is skipped.
    ///
    /// A frame read is taken as reaching everything: a frame's steps,
    /// filters and derivation can name a value anywhere down the chain, and
    /// answering that precisely is the dependency graph's job, not this
    /// shortcut's. Being wrong in the expensive direction is the safe way to
    /// be wrong here.
    pub(crate) fn overrides_reach(&self, overrides: &Overrides, expression: &Expr) -> bool {
        let targets: HashSet<&str> = match overrides {
            Overrides::Scenario(scenario_id) => match self.scenario(scenario_id) {
                Some(scenario) => scenario.values.keys().map(String::as_str).collect(),
                None => return true,
            },
            Overrides::Values(values) => values.keys().map(String::as_str).collect(),
        };
        if targets.is_empty() {
            return false;
        }
        let mut seen = HashSet::new();
        self.overrides_reach_through(expression, &targets, &mut seen)
    }

    fn overrides_reach_through(
        &self,
        expression: &Expr,
        targets: &HashSet<&str>,
        seen: &mut HashSet<Id>,
    ) -> bool {
        let mut found = false;
        expression.walk(&mut |node| {
            if found {
                return;
            }
            match node {
                Expr::ForeignColumn { .. } => found = true,
                Expr::Value { object_id } => {
                    if targets.contains(object_id.as_str()) {
                        found = true;
                        return;
                    }
                    if !seen.insert(object_id.clone()) {
                        return;
                    }
                    let dependency = match self.object(object_id) {
                        Ok(DataObject::Result(result)) => Some(result.formula.expression.clone()),
                        _ => self
                            .block_line(object_id)
                            .and_then(|(block, index)| block.lines[index].expression().cloned()),
                    };
                    if let Some(dependency) = dependency {
                        found = self.overrides_reach_through(&dependency, targets, seen);
                    }
                }
                _ => {}
            }
        });
        found
    }

    /// The report for a line whose whole formula is one `solve` call, or
    /// `None` when the line is something else. The block computes such a
    /// line here rather than through Polars so the gutter gets the search's
    /// facts and not only its number.
    pub(crate) fn solve_report(&self, expression: &Expr) -> Option<Result<SolveReport, String>> {
        let Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } = expression
        else {
            return None;
        };
        if name != "solve" {
            return None;
        }
        Some(self.solve(arguments, keyword_arguments))
    }

    fn solve(
        &self,
        arguments: &[Expr],
        keywords: &[(String, Expr)],
    ) -> Result<SolveReport, String> {
        const SHAPE: &str = "solve(expression == target, by=`Value`, within=[low, high])";
        let [comparison] = arguments else {
            return Err(format!("solve takes one comparison: {SHAPE}"));
        };
        let Expr::Binary {
            operator: BinaryOperator::Equal,
            left,
            right,
        } = comparison
        else {
            return Err(format!(
                "solve's first argument says what should equal what: {SHAPE}"
            ));
        };
        for (keyword, _) in keywords {
            if !matches!(keyword.as_str(), "by" | "within" | "tolerance") {
                return Err(format!(
                    "solve does not take a {keyword}= argument. {SHAPE}"
                ));
            }
        }
        let Some(Expr::Value { object_id }) = keyword_argument(keywords, "by") else {
            return Err(format!("by= names the value to vary: {SHAPE}"));
        };
        let target = match self.object(object_id) {
            Ok(DataObject::Value(value)) => value,
            _ => return Err("by= must name a value card, not a result or a line.".into()),
        };
        if self.frozen_values.contains_key(object_id) {
            return Err(format!(
                "‘{}’ is frozen, so solve cannot vary it.",
                target.name
            ));
        }
        match target.data_type {
            DataType::Number
            | DataType::Integer
            | DataType::Currency
            | DataType::Percentage
            | DataType::Accounting => {}
            _ => {
                return Err(format!(
                    "‘{}’ is not a number, so solve cannot vary it.",
                    target.name
                ));
            }
        }
        let Some(within) = keyword_argument(keywords, "within") else {
            return Err(format!("within= brackets the search: {SHAPE}"));
        };
        let (low, high) = self.bracket(within)?;
        let tolerance = match keyword_argument(keywords, "tolerance") {
            None => 1e-12,
            Some(expression) => {
                let value = self.scalar_number(expression)?;
                if !(value > 0.0) {
                    return Err("tolerance= must be a positive number".into());
                }
                value
            }
        };
        let difference = Expr::Binary {
            operator: BinaryOperator::Subtract,
            left: left.clone(),
            right: right.clone(),
        };
        if !self.overrides_reach(
            &Overrides::Values(BTreeMap::from([(object_id.clone(), String::new())])),
            &difference,
        ) {
            return Err(format!(
                "Nothing in the comparison reads ‘{}’, so changing it cannot make the sides meet.",
                target.name
            ));
        }

        let mut evaluations = 0usize;
        let mut objective = |input: f64| -> Result<f64, String> {
            evaluations += 1;
            let raw = scalar_value_to_raw(ScalarValue::Number(input));
            let overrides = Overrides::Values(BTreeMap::from([(object_id.clone(), raw)]));
            let (_, series) = self.evaluate_under(&overrides, &difference)?;
            if series.len() != 1 {
                return Err("solve needs a comparison of two single values".into());
            }
            match polars_value_at(&series, 0)? {
                ScalarValue::Number(value) if value.is_finite() => Ok(value),
                ScalarValue::Null => Err(format!(
                    "The comparison has no value when ‘{}’ is {}.",
                    target.name,
                    format_input(input)
                )),
                _ => Err("solve needs both sides of the comparison to be numbers".into()),
            }
        };
        let side = |lhs: &Expr, rhs: &Expr| {
            format!(
                "{} and {}",
                lhs.render(&FrameObject::default(), self, 0),
                rhs.render(&FrameObject::default(), self, 0)
            )
        };
        let mut left_input = low;
        let mut right_input = high;
        let mut left_value = objective(left_input)?;
        let right_value = objective(right_input)?;
        let done = |input: f64, value: f64, iterations: usize, evaluations: usize| SolveReport {
            target_id: object_id.clone(),
            target_name: target.name.clone(),
            answer: input,
            answer_raw: rounded_raw(input),
            iterations,
            residual: value,
            low,
            high,
            evaluations,
        };
        if left_value == 0.0 {
            return Ok(done(left_input, left_value, 0, evaluations));
        }
        if right_value == 0.0 {
            return Ok(done(right_input, right_value, 0, evaluations));
        }
        if left_value.signum() == right_value.signum() {
            return Err(format!(
                "Nothing between {} and {} makes {} meet: the difference is {} at {} and {} at {}, on the same side of zero. Widen within=.",
                format_input(low),
                format_input(high),
                side(left, right),
                format_input(left_value),
                format_input(low),
                format_input(right_value),
                format_input(high)
            ));
        }
        for iteration in 1..=MAX_BISECTIONS {
            let middle = left_input + (right_input - left_input) / 2.0;
            let value = objective(middle)?;
            if value == 0.0 || (right_input - left_input).abs() <= tolerance * (1.0 + middle.abs())
            {
                // The bisected answer is a long decimal that is almost the
                // round number the model actually wants. Try the short
                // spellings first, shortest first, and keep the first one
                // that meets the target at least as well: `138.75` beats
                // `138.749999972` when both make EBITDA 300,000.
                let (answer, residual) = (6..=11)
                    .map(|digits| {
                        let candidate: f64 =
                            format!("{middle:.digits$e}").parse().unwrap_or(middle);
                        candidate
                    })
                    .filter(|candidate| candidate != &middle)
                    .find_map(|candidate| match objective(candidate) {
                        Ok(residual) if residual.abs() <= value.abs() => {
                            Some((candidate, residual))
                        }
                        _ => None,
                    })
                    .unwrap_or((middle, value));
                return Ok(done(answer, residual, iteration, evaluations));
            }
            if value.signum() == left_value.signum() {
                left_input = middle;
                left_value = value;
            } else {
                right_input = middle;
            }
        }
        Err(format!(
            "solve did not settle within {MAX_BISECTIONS} steps. Narrow within= or loosen tolerance=."
        ))
    }

    /// `within=[low, high]`: two numbers, written or computed, low first.
    fn bracket(&self, within: &Expr) -> Result<(f64, f64), String> {
        let (data_type, series) = self.evaluate_scratchwork_series(within)?;
        if series.len() != 2 {
            return Err("within= takes exactly two numbers: within=[low, high]".into());
        }
        let mut bounds = [0.0; 2];
        for (slot, index) in bounds.iter_mut().zip(0..2) {
            *slot = match expression_value_at(&series, data_type, index)? {
                Expr::Integer { value } => value as f64,
                Expr::Number { value } | Expr::Money { value } | Expr::Percentage { value } => {
                    value
                }
                _ => return Err("within= takes two numbers: within=[low, high]".into()),
            };
        }
        let [low, high] = bounds;
        if !(low.is_finite() && high.is_finite()) || low >= high {
            return Err("within= needs low before high: within=[100, 200]".into());
        }
        Ok((low, high))
    }

    fn scalar_number(&self, expression: &Expr) -> Result<f64, String> {
        let (_, series) = self.evaluate_scratchwork_series(expression)?;
        match (series.len(), polars_value_at(&series, 0)?) {
            (1, ScalarValue::Number(value)) => Ok(value),
            _ => Err("Expected one number".into()),
        }
    }
}

impl Document {
    /// `expression` with every `under(…)` and `solve(…)` in it replaced by
    /// the literal it answers, evaluated against this document as it
    /// stands.
    ///
    /// This runs before Scratchwork prepares live upstream lines, and has
    /// to: preparation inlines a live line's *current* answer as a literal,
    /// which is the right thing for every ordinary read and exactly the
    /// wrong thing for a line asking what that answer would be under a
    /// different scenario. Folding first means the copy `under` makes is a
    /// copy of the pristine document, whose lines still say what they
    /// compute rather than what they last computed to.
    pub(crate) fn fold_overrides(&self, expression: &Expr) -> Result<Expr, String> {
        let fold = |items: &[Expr]| {
            items
                .iter()
                .map(|item| self.fold_overrides(item))
                .collect::<Result<Vec<_>, _>>()
        };
        let fold_keywords = |items: &[(String, Expr)]| {
            items
                .iter()
                .map(|(keyword, item)| Ok((keyword.clone(), self.fold_overrides(item)?)))
                .collect::<Result<Vec<_>, String>>()
        };
        Ok(match expression {
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } if name == "under" => self.under_literal(arguments, keyword_arguments)?,
            Expr::PolarsCall { name, .. } if name == "solve" => {
                let report = self
                    .solve_report(expression)
                    .expect("a solve call reports")?;
                Expr::Number {
                    value: report.answer,
                }
            }
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } => Expr::PolarsCall {
                name: name.clone(),
                arguments: fold(arguments)?,
                keyword_arguments: fold_keywords(keyword_arguments)?,
            },
            Expr::Method {
                input,
                path,
                arguments,
                keyword_arguments,
            } => Expr::Method {
                input: Box::new(self.fold_overrides(input)?),
                path: path.clone(),
                arguments: fold(arguments)?,
                keyword_arguments: fold_keywords(keyword_arguments)?,
            },
            Expr::List { items } => Expr::List {
                items: fold(items)?,
            },
            Expr::Negate { expression } => Expr::Negate {
                expression: Box::new(self.fold_overrides(expression)?),
            },
            Expr::Not { expression } => Expr::Not {
                expression: Box::new(self.fold_overrides(expression)?),
            },
            Expr::Binary {
                operator,
                left,
                right,
            } => Expr::Binary {
                operator: *operator,
                left: Box::new(self.fold_overrides(left)?),
                right: Box::new(self.fold_overrides(right)?),
            },
            other => other.clone(),
        })
    }

    /// The literal an `under(scenario, expression)` call answers.
    fn under_literal(
        &self,
        arguments: &[Expr],
        keywords: &[(String, Expr)],
    ) -> Result<Expr, String> {
        if !keywords.is_empty() {
            return Err("under takes no keyword arguments: under(`Upside`, expression)".into());
        }
        let [scenario, expression] = arguments else {
            return Err(
                "under takes a scenario and an expression: under(`Upside`, expression)".into(),
            );
        };
        let scenario_id = match scenario {
            Expr::Scenario { scenario_id } => scenario_id.clone(),
            Expr::String { value } => self
                .scenarios
                .iter()
                .find(|candidate| candidate.name.eq_ignore_ascii_case(value.trim()))
                .map(|candidate| candidate.id.clone())
                .ok_or_else(|| no_such_scenario(self, value))?,
            other => {
                return Err(format!(
                    "under's first argument names a scenario, not {}. Write under(`Upside`, expression).",
                    other.render(&FrameObject::default(), self, 0)
                ));
            }
        };
        let (data_type, series) =
            self.evaluate_under(&Overrides::Scenario(scenario_id), expression)?;
        let mut items = (0..series.len())
            .map(|index| expression_value_at(&series, data_type, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(match items.len() {
            1 => items.remove(0),
            _ => Expr::List { items },
        })
    }
}

/// `under` and `solve` as root calls reached by the frame compiler, each
/// folded to the literal it answers. Scratchwork folds earlier, through
/// [`Document::fold_overrides`]; this arm is what a calculated column or a
/// matrix body compiles through.
pub(super) fn compile(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    let call = Expr::PolarsCall {
        name: name.into(),
        arguments: arguments.to_vec(),
        keyword_arguments: keywords.to_vec(),
    };
    document.fold_overrides(&call)?.to_polars(document)
}

fn no_such_scenario(document: &Document, name: &str) -> String {
    if document.scenarios.is_empty() {
        return format!(
            "There is no scenario called ‘{name}’ — this document has no scenarios yet. Add one from the Scenario menu in the corner of the canvas."
        );
    }
    format!(
        "There is no scenario called ‘{name}’. This document has {}.",
        document
            .scenarios
            .iter()
            .map(|scenario| format!("‘{}’", scenario.name))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn format_input(value: f64) -> String {
    crate::engine::values::format_number(value)
}

/// Twelve significant digits: enough to hand back exactly what was typed
/// for any number a card holds, and few enough to shed the bisection's last
/// bit so `138.75` does not arrive as `138.75000000000003`.
fn rounded_raw(value: f64) -> String {
    let rounded: f64 = format!("{value:.11e}").parse().unwrap_or(value);
    scalar_value_to_raw(ScalarValue::Number(rounded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(store: &Store, name: &str) -> Id {
        store
            .document()
            .objects
            .iter()
            .find(|object| object.name() == name)
            .unwrap()
            .id()
            .to_string()
    }

    fn parse(document: &Document, source: &str) -> Expr {
        crate::formula::parser::Parser::new(source, &FrameObject::default(), document)
            .unwrap()
            .parse()
            .unwrap()
    }

    fn overriding(id: &str) -> Overrides {
        Overrides::Values(BTreeMap::from([(id.to_string(), "1".to_string())]))
    }

    /// The plan's "overriding a value that nothing reads costs nothing",
    /// measured by whether the copy is taken rather than by timing.
    #[test]
    fn overriding_a_value_nothing_reads_skips_the_copy() {
        let mut store = Store::new(Document::blank("Reach"));
        store
            .apply(Operation::AddContainer {
                name: "Holder".into(),
                x: 0.0,
                y: 0.0,
                container_id: None,
            })
            .unwrap();
        let holder = value(&store, "Holder");
        for (name, raw) in [("Price", "120"), ("Fixed costs", "250000")] {
            store
                .apply(Operation::AddValue {
                    name: name.into(),
                    raw: raw.into(),
                    x: 0.0,
                    y: 0.0,
                    container_id: Some(holder.clone()),
                })
                .unwrap();
        }
        store
            .apply(Operation::AddVariable {
                name: "Twice fixed".into(),
                formula: "`Fixed costs` * 2".into(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        store
            .apply(Operation::AddFrame {
                name: "Plan".into(),
                grid: vec![vec!["Weight".into()], vec!["6".into()]],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let price = value(&store, "Price");
        let fixed = value(&store, "Fixed costs");
        let document = store.document().clone();
        let direct = parse(&document, "`Fixed costs` * 2");
        let through_result = parse(&document, "`Twice fixed` + 1");
        let plan = document.frame(&value(&store, "Plan")).unwrap();
        let through_frame = Expr::Method {
            input: Box::new(Expr::ForeignColumn {
                frame_id: plan.id.clone(),
                column_id: plan.columns[0].id.clone(),
            }),
            path: vec!["sum".into()],
            arguments: Vec::new(),
            keyword_arguments: Vec::new(),
        };
        assert!(document.overrides_reach(&overriding(&fixed), &direct));
        assert!(!document.overrides_reach(&overriding(&price), &direct));
        assert!(document.overrides_reach(&overriding(&fixed), &through_result));
        assert!(!document.overrides_reach(&overriding(&price), &through_result));
        // A frame may read anything down its chain, so it always reaches.
        assert!(document.overrides_reach(&overriding(&price), &through_frame));
        // A scenario that overrides nothing reaches nothing.
        store
            .apply(Operation::AddScenario {
                scenario_id: None,
                name: "Empty".into(),
                copy_from: None,
            })
            .unwrap();
        let empty = store.document().scenarios[0].id.clone();
        assert!(
            !store
                .document()
                .overrides_reach(&Overrides::Scenario(empty), &through_frame)
        );
    }
}
