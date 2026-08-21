//! Reading one line of a scratchpad: what it is called, and what it says.
//!
//! A block is edited as text, so the split between a line's name and its
//! expression is lexical rather than structural — `x = 10` is a name and an
//! expression in the same breath, which is the whole point of typing into a
//! scratchpad instead of filling in a field beside one.

use std::borrow::Cow;

/// What a single line of block text turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine<'a> {
    /// The name written in front of the `=`, if one was.
    pub name: Option<Cow<'a, str>>,
    /// Whether the author used formula-style backticks around the name.
    /// Scratchwork accepts the plain-language spelling too, but preserves
    /// the quoted spelling when it was chosen so Wrangle and Scratchwork do
    /// not appear to speak subtly different languages.
    pub name_quoted: bool,
    /// Everything else, trimmed — the expression to parse, the comment to
    /// keep, or nothing at all on a blank line.
    pub source: &'a str,
}

/// Physical lines grouped into the logical ones a scratchpad computes.
///
/// A physical line that begins with whitespace and says something continues
/// the line above it. An open delimiter is the stronger, visible boundary:
/// every physical line through its matching close is one calculation, even
/// when the close returns to the margin. The editor writes that explicit
/// shape when Alt+Return first expands a formula:
///
/// ```text
/// revenue = (
///   `Amount`
///     .filter(`Region` == "East")
/// ).sum()
/// ```
///
/// A continuation never joins onto a *blank* line: the blank would vanish
/// into the joined text's trim and the block would stop round-tripping what
/// the author typed. An indented line after a blank is simply its own line,
/// which is also what it was before this rule existed.
///
/// The joined text keeps its newlines and indentation verbatim, so a
/// block's source reconstructs byte for byte and the author's layout is
/// theirs. The formula lexer treats a newline as any other whitespace, so
/// the parse neither knows nor cares where the line broke.
pub(crate) fn logical_lines(source: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut delimiter_depth = 0;
    for physical in source.split('\n') {
        let continues = !lines.is_empty()
            && (delimiter_depth > 0
                || (physical.starts_with([' ', '\t'])
                    && !physical.trim().is_empty()
                    && lines
                        .last()
                        .is_some_and(|previous| !previous.trim().is_empty())));
        match (continues, lines.last_mut()) {
            (true, Some(previous)) => {
                previous.push('\n');
                previous.push_str(physical);
            }
            _ => {
                lines.push(physical.to_string());
                delimiter_depth = 0;
            }
        }
        delimiter_depth = delimiter_depth_after(physical, delimiter_depth);
    }
    lines
}

/// The small, error-tolerant piece of scanning line grouping needs.
///
/// It cannot ask the formula parser because this runs while the author is in
/// the middle of making an incomplete formula. It consequently promises only
/// what layout needs: brackets inside quoted strings and backticked names are
/// inert, unmatched closes do not make depth negative, and an unmatched open
/// holds the following physical row in the same calculation until it closes.
fn delimiter_depth_after(source: &str, initial: usize) -> usize {
    if initial == 0 && source.trim_start().starts_with('#') {
        return 0;
    }
    let mut depth = initial;
    let mut quote = None;
    let mut escaped = false;
    for character in source.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(open) = quote {
            if open != '`' && character == '\\' {
                escaped = true;
            } else if character == open {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' | '`' => quote = Some(character),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

/// Splits `name = expression` into its two halves.
///
/// The `=` has to be a lone one: `a == b`, `a >= b`, `a <= b` and `a != b`
/// are comparisons, and a scratchpad that read the first of those as naming
/// a line `a` would be worse than useless. A name is a plain word, or
/// several — spaces are allowed, because `down payment = 40000` is how
/// somebody actually writes that down — and anything less tidy than that is
/// taken to be part of the expression rather than a name for it.
///
/// A leading `=` with nothing before it is dropped: it is the spreadsheet
/// habit of starting a formula with one, and honouring it costs nothing.
pub fn split_line<'a>(line: &'a str) -> ParsedLine<'a> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return ParsedLine {
            name: None,
            name_quoted: false,
            source: trimmed,
        };
    }
    let Some(position) = assignment_position(trimmed) else {
        return ParsedLine {
            name: None,
            name_quoted: false,
            source: trimmed,
        };
    };
    let (head, tail) = trimmed.split_at(position);
    let source = tail[1..].trim();
    let head = head.trim();
    if head.is_empty() {
        // `= 10`, typed out of spreadsheet habit.
        return ParsedLine {
            name: None,
            name_quoted: false,
            source,
        };
    }
    if let Some(name) = backticked_line_name(head) {
        return ParsedLine {
            name: Some(name),
            name_quoted: true,
            source,
        };
    }
    if !is_line_name(head) {
        return ParsedLine {
            name: None,
            name_quoted: false,
            source: trimmed,
        };
    }
    ParsedLine {
        name: Some(Cow::Borrowed(head)),
        name_quoted: false,
        source,
    }
}

/// The exact name inside a formula-style backtick pair.
///
/// A doubled backtick is one literal backtick, matching the formula lexer.
/// Anything else that closes early is rejected so a malformed declaration
/// remains visible as an expression and receives the ordinary parse error.
fn backticked_line_name(candidate: &str) -> Option<Cow<'_, str>> {
    let inner = candidate.strip_prefix('`')?.strip_suffix('`')?;
    if inner.is_empty() {
        return None;
    }
    if !inner.contains('`') {
        return Some(Cow::Borrowed(inner));
    }
    let mut unescaped = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '`' {
            unescaped.push(character);
            continue;
        }
        if characters.next() != Some('`') {
            return None;
        }
        unescaped.push('`');
    }
    Some(Cow::Owned(unescaped))
}

/// The byte offset of the `=` that assigns, if the line has one.
fn assignment_position(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_backticks = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'`' {
            if in_backticks && bytes.get(index + 1) == Some(&b'`') {
                index += 2;
                continue;
            }
            in_backticks = !in_backticks;
            index += 1;
            continue;
        }
        if in_backticks {
            index += 1;
            continue;
        }
        if byte != b'=' {
            index += 1;
            continue;
        }
        // `==` is a comparison, and so is the `=` of `<=`, `>=` and `!=`.
        if bytes.get(index + 1) == Some(&b'=') {
            return None;
        }
        if index > 0 && matches!(bytes[index - 1], b'<' | b'>' | b'!' | b'=') {
            return None;
        }
        return Some(index);
    }
    None
}

/// Whether `candidate` is something a person would accept as a name.
///
/// Letters, digits, underscores and single spaces between words, opening on
/// something that is not a digit. Deliberately narrow: everything rejected
/// here stays part of the expression, so the cost of being wrong is a line
/// that computes rather than a line that vanishes into a name.
pub fn is_line_name(candidate: &str) -> bool {
    let mut characters = candidate.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    if candidate.contains("  ") {
        return false;
    }
    characters.all(|character| character.is_alphanumeric() || character == '_' || character == ' ')
}

/// Whether a name can be written into a formula without backticks around it.
///
/// The bare spelling is the one a scratchpad wants — `x + y`, not
/// `` `x` + `y` `` — and it is available exactly when the name could not be
/// mistaken for anything else the lexer reads.
pub fn is_bare_name(candidate: &str) -> bool {
    if matches!(candidate, "True" | "False" | "None" | "and" | "or" | "not") {
        return false;
    }
    let mut characters = candidate.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_')
        && characters.all(|character| character.is_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> (Option<String>, &str) {
        let ParsedLine { name, source, .. } = split_line(line);
        (name.map(Cow::into_owned), source)
    }

    #[test]
    fn a_name_in_front_of_an_equals_is_a_name() {
        assert_eq!(parsed("x = 10"), (Some("x".into()), "10"));
        assert_eq!(parsed("x=10"), (Some("x".into()), "10"));
        assert_eq!(
            parsed("  down payment = 40000  "),
            (Some("down payment".into()), "40000")
        );
        assert_eq!(parsed("rate_2 = 0.08"), (Some("rate_2".into()), "0.08"));
    }

    #[test]
    fn a_backticked_formula_name_can_name_a_line() {
        let quoted = split_line("`down payment` = 40000");
        assert_eq!(quoted.name.as_deref(), Some("down payment"));
        assert!(quoted.name_quoted);
        assert_eq!(quoted.source, "40000");

        let escaped = split_line("`rate `` special` = 0.08");
        assert_eq!(escaped.name.as_deref(), Some("rate ` special"));
        assert!(escaped.name_quoted);

        let with_equals = split_line("`gross=net` = 10");
        assert_eq!(with_equals.name.as_deref(), Some("gross=net"));
        assert_eq!(with_equals.source, "10");
    }

    #[test]
    fn a_comparison_is_not_an_assignment() {
        assert_eq!(parsed("x == 10"), (None, "x == 10"));
        assert_eq!(parsed("x >= 10"), (None, "x >= 10"));
        assert_eq!(parsed("x <= 10"), (None, "x <= 10"));
        assert_eq!(parsed("x != 10"), (None, "x != 10"));
    }

    #[test]
    fn an_expression_that_is_not_a_name_stays_an_expression() {
        assert_eq!(parsed("x + y"), (None, "x + y"));
        assert_eq!(
            parsed("`Ledger`.`Amount`.sum()"),
            (None, "`Ledger`.`Amount`.sum()")
        );
        // The head is not a plain name, so the whole line is the expression.
        assert_eq!(parsed("x + y = 10"), (None, "x + y = 10"));
        assert_eq!(parsed("2x = 10"), (None, "2x = 10"));
    }

    #[test]
    fn the_spreadsheet_habit_of_a_leading_equals_is_forgiven() {
        assert_eq!(parsed("= 10 * 2"), (None, "10 * 2"));
    }

    #[test]
    fn blanks_and_comments_carry_no_name() {
        assert_eq!(parsed(""), (None, ""));
        assert_eq!(parsed("   "), (None, ""));
        assert_eq!(
            parsed("# read off the bank site"),
            (None, "# read off the bank site")
        );
        // Even one shaped like an assignment.
        assert_eq!(parsed("# x = 10"), (None, "# x = 10"));
    }

    #[test]
    fn bare_names_are_the_ones_that_cannot_be_mistaken() {
        assert!(is_bare_name("x"));
        assert!(is_bare_name("account_balance"));
        assert!(!is_bare_name("down payment"));
        assert!(!is_bare_name("True"));
        assert!(!is_bare_name("2x"));
        assert!(!is_bare_name(""));
    }

    #[test]
    fn a_delimiter_keeps_its_margin_close_and_terminal_method_together() {
        let source = "total = (\n  [1, 2, 3]\n).sum()\ntotal / 2";
        assert_eq!(
            logical_lines(source),
            ["total = (\n  [1, 2, 3]\n).sum()", "total / 2"]
        );
    }

    #[test]
    fn delimiters_printed_in_values_do_not_group_lines() {
        assert_eq!(
            logical_lines("text = \"(\"\nname = `a[b`\n# note (\n3"),
            ["text = \"(\"", "name = `a[b`", "# note (", "3"]
        );
    }
}
