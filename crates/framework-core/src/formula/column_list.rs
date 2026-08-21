//! A written list of columns: `` `Jan`, `Feb`, starts_with("Q") ``.
//!
//! The notation a step reaches for when what it wants is *columns* rather
//! than values — an unpivot's melt list is the first taker. Written out
//! rather than clicked because the frame this operation exists for is the
//! wide one, and forty checkboxes is not a way to say "the quarter
//! columns". The same backtick rule as the formula language: a column is
//! named exactly, in backticks, and a bare word gets the same nudge a
//! formula would give it.
//!
//! Besides plain references there are selectors, for saying a family of
//! columns at once:
//!
//! - `starts_with("Q")`, `ends_with("total")`, `contains("2024")` — the
//!   columns whose display name matches, in frame order.
//! - `except(`Region`)` — every column *but* those named, which is the
//!   shortest honest spelling of "melt the whole wide frame".
//!
//! A selector is resolved right here, when the step is written, into the
//! concrete columns it matched — the list that comes back is ids and
//! nothing else. That is the same decision the pivot makes about its
//! outputs and the union about its mapping: a step keeps meaning what it
//! meant when it was saved, and a column arriving upstream next week does
//! not quietly join a melt somebody wrote today. Re-saving the step is
//! what re-runs the selector.
//!
//! Matching is exact, case included. A pattern is not a name: names get
//! the normalized matching the parser gives bare words, but a pattern
//! swept over forty headers has to be predictable above all, and the
//! zero-match error says so rather than letting a lowercase `q` quietly
//! match nothing anyone can see.
//!
//! On duplicates the two piece kinds part ways. A reference named twice is
//! refused — the second mention is more likely a typo for a neighbouring
//! column than a thing anyone meant — while a selector sweeping in a
//! column the list already holds skips it silently, because overlap is
//! what sweeps do and erroring on it would make two selectors nearly
//! impossible to combine.

use crate::Id;
use crate::error::CoreError;
use crate::formula::lexer::{FormulaReference, ReferenceName, Token, tokenize};
use crate::model::frame::{Column, FrameObject};
use crate::reference_matches;

/// Parses `source` against `frame`'s columns — the schema visible where
/// the list is written — and answers the matched column ids in written
/// order. An all-whitespace source is an empty list, not an error; what an
/// empty list *means* is the caller's sentence to say.
pub(crate) fn parse_column_list(source: &str, frame: &FrameObject) -> Result<Vec<Id>, CoreError> {
    let tokens = tokenize(source)?;
    let mut list = ColumnList {
        tokens,
        position: 0,
        frame,
        collected: Vec::new(),
    };
    list.parse()?;
    Ok(list.collected)
}

const NAME_SELECTORS: &[&str] = &["starts_with", "ends_with", "contains"];

struct ColumnList<'a> {
    tokens: Vec<Token>,
    position: usize,
    frame: &'a FrameObject,
    collected: Vec<Id>,
}

impl ColumnList<'_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn next(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if token != Token::End {
            self.position += 1;
        }
        token
    }

    fn parse(&mut self) -> Result<(), CoreError> {
        loop {
            if self.peek() == &Token::End {
                return Ok(());
            }
            self.piece()?;
            match self.next() {
                Token::Comma => {}
                Token::End => return Ok(()),
                _ => {
                    return Err(CoreError::Formula(
                        "Separate the columns in this list with commas".into(),
                    ));
                }
            }
        }
    }

    fn piece(&mut self) -> Result<(), CoreError> {
        let Token::Identifier(reference) = self.next() else {
            return Err(CoreError::Formula(
                "This list holds column names, each in backticks: `Jan`, `Feb`".into(),
            ));
        };
        match &reference {
            FormulaReference::Unqualified(name)
                if !name.exact && self.peek() == &Token::LeftParen =>
            {
                self.selector(name)
            }
            FormulaReference::Unqualified(name) => {
                let column_id = self.resolve(name)?;
                if self.collected.contains(&column_id) {
                    return Err(CoreError::Formula(format!(
                        "‘{}’ is already in the list",
                        name.value
                    )));
                }
                self.collected.push(column_id);
                Ok(())
            }
            // The list can only name this frame's own columns, so a dotted
            // path has nowhere to point that a plain name does not.
            FormulaReference::Qualified(path) => Err(CoreError::Formula(format!(
                "Write just the column's name: ‘{}’",
                path.last().map(|name| name.value.as_str()).unwrap_or("")
            ))),
        }
    }

    fn selector(&mut self, name: &ReferenceName) -> Result<(), CoreError> {
        let selector = name.value.to_ascii_lowercase();
        if selector != "except" && !NAME_SELECTORS.contains(&selector.as_str()) {
            return Err(CoreError::Formula(format!(
                "‘{}’ is not a selector here. The selectors are starts_with(…), \
                 ends_with(…), contains(…), and except(…).",
                name.value
            )));
        }
        self.next(); // the opening paren the caller peeked
        let matched = if selector == "except" {
            self.except()?
        } else {
            self.pattern(&selector)?
        };
        // A sweep overlapping what the list already holds is normal — that
        // is what sweeps do — so the overlap is skipped rather than refused.
        for column_id in matched {
            if !self.collected.contains(&column_id) {
                self.collected.push(column_id);
            }
        }
        Ok(())
    }

    /// `starts_with("Q")` and its siblings: one written pattern, matched
    /// exactly against every visible column's display name.
    fn pattern(&mut self, selector: &str) -> Result<Vec<Id>, CoreError> {
        let Token::String(pattern) = self.next() else {
            return Err(CoreError::Formula(format!(
                "{selector} takes the text to match, in quotes: {selector}(\"Q\")"
            )));
        };
        if self.next() != Token::RightParen {
            return Err(CoreError::Formula(format!(
                "{selector} takes one pattern and nothing else"
            )));
        }
        // An empty pattern matches every column, which is never what an
        // empty string in a half-edited box means. except(…) is the honest
        // spelling of "everything but", and everything else is a list.
        if pattern.is_empty() {
            return Err(CoreError::Formula(format!(
                "{selector}(\"\") would match every column. Name the columns, \
                 or say which to leave out with except(…)."
            )));
        }
        let verb = match selector {
            "starts_with" => "starts with",
            "ends_with" => "ends with",
            _ => "contains",
        };
        let matched: Vec<Id> = self
            .frame
            .columns
            .iter()
            .filter(|column| match selector {
                "starts_with" => column.name.starts_with(pattern.as_str()),
                "ends_with" => column.name.ends_with(pattern.as_str()),
                _ => column.name.contains(pattern.as_str()),
            })
            .map(|column| column.id.clone())
            .collect();
        if matched.is_empty() {
            return Err(CoreError::Formula(format!(
                "No column's name {verb} ‘{pattern}’. Matching is exact, \
                 capitals included."
            )));
        }
        Ok(matched)
    }

    /// `except(`Region`, `Notes`)`: every visible column but those named.
    /// The named columns still have to exist — a typo here would otherwise
    /// come back as that column quietly joining the melt.
    fn except(&mut self) -> Result<Vec<Id>, CoreError> {
        let mut excepted: Vec<Id> = Vec::new();
        loop {
            match self.next() {
                Token::Identifier(FormulaReference::Unqualified(name)) => {
                    excepted.push(self.resolve(&name)?);
                }
                Token::RightParen if excepted.is_empty() => {
                    return Err(CoreError::Formula(
                        "except(…) needs the columns to leave out: except(`Region`)".into(),
                    ));
                }
                _ => {
                    return Err(CoreError::Formula(
                        "except(…) takes column names in backticks: except(`Region`)".into(),
                    ));
                }
            }
            match self.next() {
                Token::Comma => {}
                Token::RightParen => break,
                _ => {
                    return Err(CoreError::Formula(
                        "Separate except(…)'s columns with commas".into(),
                    ));
                }
            }
        }
        Ok(self
            .frame
            .columns
            .iter()
            .filter(|column| !excepted.contains(&column.id))
            .map(|column| column.id.clone())
            .collect())
    }

    /// The column a written name lands on. The same rule as the formula
    /// language: exact names in backticks resolve, a bare word is told to
    /// put them on, and a name two columns share resolves to neither.
    fn resolve(&self, name: &ReferenceName) -> Result<Id, CoreError> {
        if !name.exact {
            return Err(CoreError::Formula(format!(
                "Wrap column names in backticks: `{}`",
                name.value
            )));
        }
        let matches: Vec<&Column> = self
            .frame
            .columns
            .iter()
            .filter(|column| reference_matches(&column.name, name))
            .collect();
        match matches.as_slice() {
            [column] => Ok(column.id.clone()),
            [] => Err(CoreError::Formula(format!(
                "Unknown column ‘{}’",
                name.value
            ))),
            _ => Err(CoreError::Formula(format!(
                "Ambiguous column name ‘{}’",
                name.value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::frame::FrameDisplay;

    fn scope(names: &[&str]) -> FrameObject {
        FrameObject {
            comment: None,
            id: "frame".into(),
            name: "Wide".into(),
            columns: names
                .iter()
                .enumerate()
                .map(|(index, name)| Column {
                    id: format!("c{index}"),
                    name: (*name).into(),
                    source_name: None,
                    data_type: crate::model::value::DataType::Number,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                })
                .collect(),
            rows: Vec::new(),
            steps: Vec::new(),
            display: FrameDisplay::default(),
            base_columns: Vec::new(),
            source_file: None,
            artifact: None,
            connector: None,
            derivation: None,
            generator: None,
            entry_columns: Vec::new(),
            materialization: None,
            unique_keys: Vec::new(),
            summaries: Vec::new(),
        }
    }

    fn ids(source: &str, names: &[&str]) -> Result<Vec<Id>, CoreError> {
        parse_column_list(source, &scope(names))
    }

    #[test]
    fn references_resolve_in_written_order_and_tolerate_a_trailing_comma() {
        let list = ids("`Feb`, `Jan`,", &["Region", "Jan", "Feb"]).unwrap();
        assert_eq!(list, vec!["c2", "c1"]);
    }

    #[test]
    fn an_empty_source_is_an_empty_list() {
        assert_eq!(ids("   ", &["Region"]).unwrap(), Vec::<Id>::new());
    }

    #[test]
    fn a_bare_name_is_told_to_wear_backticks() {
        let error = ids("Jan", &["Region", "Jan"]).unwrap_err();
        assert!(error.to_string().contains("backticks"), "{error}");
    }

    #[test]
    fn an_unknown_column_is_named_in_the_refusal() {
        let error = ids("`Mar`", &["Region", "Jan"]).unwrap_err();
        assert!(error.to_string().contains("‘Mar’"), "{error}");
    }

    #[test]
    fn a_column_written_twice_is_refused() {
        let error = ids("`Jan`, `Jan`", &["Region", "Jan"]).unwrap_err();
        assert!(error.to_string().contains("already in the list"), "{error}");
    }

    #[test]
    fn a_backticked_name_holding_a_backtick_still_resolves() {
        let list = ids("`Q1 `` actual`", &["Region", "Q1 ` actual"]).unwrap();
        assert_eq!(list, vec!["c1"]);
    }

    #[test]
    fn starts_with_sweeps_matching_columns_in_frame_order() {
        let list = ids(
            "starts_with(\"Q\")",
            &["Region", "Q1", "Notes", "Q2", "quota"],
        )
        .unwrap();
        assert_eq!(list, vec!["c1", "c3"]);
    }

    #[test]
    fn a_selector_overlapping_the_list_skips_what_is_already_there() {
        let list = ids("`Q2`, starts_with(\"Q\")", &["Region", "Q1", "Q2"]).unwrap();
        assert_eq!(list, vec!["c2", "c1"]);
    }

    #[test]
    fn a_pattern_matching_nothing_says_matching_is_exact() {
        let error = ids("starts_with(\"q\")", &["Region", "Q1"]).unwrap_err();
        assert!(error.to_string().contains("capitals included"), "{error}");
    }

    #[test]
    fn except_answers_everything_but_the_named_columns() {
        let list = ids("except(`Region`)", &["Region", "Jan", "Feb"]).unwrap();
        assert_eq!(list, vec!["c1", "c2"]);
    }

    #[test]
    fn except_refuses_a_column_it_does_not_know() {
        let error = ids("except(`Regoin`)", &["Region", "Jan"]).unwrap_err();
        assert!(error.to_string().contains("‘Regoin’"), "{error}");
    }

    #[test]
    fn a_misspelled_selector_lists_the_real_ones() {
        let error = ids("startswith(\"Q\")", &["Region", "Q1"]).unwrap_err();
        assert!(error.to_string().contains("starts_with"), "{error}");
    }

    #[test]
    fn anything_other_than_a_name_is_refused_as_a_piece() {
        let error = ids("`Jan` + 1", &["Region", "Jan"]).unwrap_err();
        assert!(error.to_string().contains("commas"), "{error}");
    }
}
