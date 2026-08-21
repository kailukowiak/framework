use crate::error::CoreError;
use chrono::NaiveDate;

/// What a duration can be written as, and the Polars unit each stands for.
///
/// The spelling on the right is what gets handed to `offset_by`, so
/// whatever anyone writes means exactly what Polars means by it —
/// including the pair that catches people out, `m` for minutes against
/// `mo` for months. The long forms are here because `30days` is what a
/// person reaches for before they have learned there is a short one, and
/// being told that is not a unit would be a silly thing to be told.
const DURATION_UNITS: &[(&str, &str)] = &[
    ("ns", "ns"),
    ("nanosecond", "ns"),
    ("nanoseconds", "ns"),
    ("us", "us"),
    ("microsecond", "us"),
    ("microseconds", "us"),
    ("ms", "ms"),
    ("millisecond", "ms"),
    ("milliseconds", "ms"),
    ("s", "s"),
    ("sec", "s"),
    ("secs", "s"),
    ("second", "s"),
    ("seconds", "s"),
    ("m", "m"),
    ("min", "m"),
    ("mins", "m"),
    ("minute", "m"),
    ("minutes", "m"),
    ("h", "h"),
    ("hr", "h"),
    ("hrs", "h"),
    ("hour", "h"),
    ("hours", "h"),
    ("d", "d"),
    ("day", "d"),
    ("days", "d"),
    ("w", "w"),
    ("week", "w"),
    ("weeks", "w"),
    ("mo", "mo"),
    ("month", "mo"),
    ("months", "mo"),
    ("q", "q"),
    ("quarter", "q"),
    ("quarters", "q"),
    ("y", "y"),
    ("yr", "y"),
    ("yrs", "y"),
    ("year", "y"),
    ("years", "y"),
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferenceName {
    pub(crate) value: String,
    pub(crate) exact: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FormulaReference {
    Unqualified(ReferenceName),
    /// Two or more names joined by dots: `` `Orders`.`Quantity` ``,
    /// `` `Finance`.`Rates`.`Prime` ``. How deep it goes is not fixed,
    /// because a container can hold a container.
    Qualified(Vec<ReferenceName>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Integer(i64),
    Number(f64),
    /// `4.25%`, already divided: the value is 0.0425 and the token is what
    /// remembers it was written as a percentage.
    Percentage(f64),
    /// `$250000`: the plain number, and the fact that it is money.
    Money(f64),
    Date(NaiveDate),
    Duration(String),
    String(String),
    Identifier(FormulaReference),
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    SlashSlash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Tilde,
    Dot,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    End,
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<Token>, CoreError> {
    let mut tokens = Vec::new();
    let mut chars = source.trim().trim_start_matches('=').chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            character if character.is_whitespace() => {}
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                tokens.push(Token::StarStar);
            }
            '*' => tokens.push(Token::Star),
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                tokens.push(Token::SlashSlash);
            }
            '/' => tokens.push(Token::Slash),
            // `%%` is the remainder written unmistakably, and `%` still is
            // too everywhere it is not stuck to the end of a number. Only
            // that one spot changed meaning — see [`read_percent_tail`].
            '%' if chars.peek() == Some(&'%') => {
                chars.next();
                tokens.push(Token::Percent);
            }
            '%' => tokens.push(Token::Percent),
            '=' if chars.peek() == Some(&'=') => {
                chars.next();
                tokens.push(Token::EqualEqual);
            }
            '=' => tokens.push(Token::Equal),
            '!' if chars.peek() == Some(&'=') => {
                chars.next();
                tokens.push(Token::NotEqual);
            }
            '<' if chars.peek() == Some(&'=') => {
                chars.next();
                tokens.push(Token::LessEqual);
            }
            '<' => tokens.push(Token::Less),
            '>' if chars.peek() == Some(&'=') => {
                chars.next();
                tokens.push(Token::GreaterEqual);
            }
            '>' => tokens.push(Token::Greater),
            '&' => tokens.push(Token::And),
            '|' => tokens.push(Token::Or),
            '~' => tokens.push(Token::Tilde),
            ',' => tokens.push(Token::Comma),
            '(' => tokens.push(Token::LeftParen),
            ')' => tokens.push(Token::RightParen),
            '[' => tokens.push(Token::LeftBracket),
            ']' => tokens.push(Token::RightBracket),
            '.' if !chars.peek().is_some_and(|next| next.is_ascii_digit()) => {
                tokens.push(Token::Dot)
            }
            '\'' | '"' => {
                let quote = character;
                let mut value = String::new();
                let mut closed = false;
                while let Some(next) = chars.next() {
                    if next == quote {
                        closed = true;
                        break;
                    }
                    if next == '\\' {
                        let escaped = chars
                            .next()
                            .ok_or_else(|| CoreError::Formula("Unclosed string literal".into()))?;
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            other => other,
                        });
                    } else {
                        value.push(next);
                    }
                }
                if !closed {
                    return Err(CoreError::Formula("Unclosed string literal".into()));
                }
                tokens.push(Token::String(value));
            }
            // `$250000`, the other half of `4.25%`. Money and a rate are the
            // two numbers people write with a mark on them, and a formula
            // language that could read neither would make the block a worse
            // place to keep a figure than the cell it replaced.
            '$' => {
                let mut number = String::new();
                while chars
                    .peek()
                    .is_some_and(|next| next.is_ascii_digit() || *next == '.')
                {
                    number.push(chars.next().unwrap());
                }
                let value = number.parse().map_err(|_| {
                    CoreError::Formula(
                        "‘$’ is how money is written, so it needs a number after it: ‘$250000’."
                            .into(),
                    )
                })?;
                tokens.push(Token::Money(value));
            }
            character if character.is_ascii_digit() || character == '.' => {
                let mut number = character.to_string();
                while chars
                    .peek()
                    .is_some_and(|next| next.is_ascii_digit() || *next == '.')
                {
                    number.push(chars.next().unwrap());
                }
                if let Some(date) = read_date_tail(&number, &mut chars)? {
                    tokens.push(Token::Date(date));
                } else if let Some(duration) = read_duration_tail(&number, &mut chars)? {
                    tokens.push(Token::Duration(duration));
                } else {
                    tokens.push(numeric_token(&number, &mut chars)?);
                }
            }
            character if character.is_alphabetic() || character == '_' || character == '`' => {
                let reference = read_formula_reference(character, &mut chars)?;
                // The connectives written as words. Half the people writing
                // formulas here arrive from Python or from English, type
                // `a and b`, and got an error that named no fix. The words
                // mean exactly what `&` and `|` mean, so they lex to the
                // same tokens — bare words only: backticks still name a
                // column called ‘and’, because quoting is how a name opts
                // out of being vocabulary.
                match &reference {
                    FormulaReference::Unqualified(name)
                        if !name.exact && (name.value == "and" || name.value == "or") =>
                    {
                        tokens.push(if name.value == "and" {
                            Token::And
                        } else {
                            Token::Or
                        });
                    }
                    _ => tokens.push(Token::Identifier(reference)),
                }
            }
            other => {
                return Err(CoreError::Formula(format!(
                    "Unexpected character ‘{other}’"
                )));
            }
        }
    }
    tokens.push(Token::End);
    Ok(tokens)
}

fn numeric_token(
    number: &str,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<Token, CoreError> {
    if read_percent_tail(number, chars)? {
        return number
            .parse::<f64>()
            .map(|value| Token::Percentage(value / 100.0))
            .map_err(|_| CoreError::Formula(format!("Invalid number ‘{number}’")));
    }
    if !number.contains('.') {
        return number
            .parse::<i64>()
            .map(Token::Integer)
            .map_err(|_| CoreError::Formula(format!("Invalid integer ‘{number}’")));
    }
    number
        .parse::<f64>()
        .map(Token::Number)
        .map_err(|_| CoreError::Formula(format!("Invalid number ‘{number}’")))
}

/// Whether the number just read wears a `%`, making it a percentage.
///
/// `4.25%` is how a rate gets written down, on paper and in every
/// spreadsheet, and a language that made somebody type `0.0425` instead
/// would be asking them to do the conversion the machine is for. So the
/// sign binds to the number the way a duration's `d` does: `4.25%` is one
/// token worth 0.0425, and it carries the fact that it is a percentage, so
/// it reads back out as `4.25%` rather than as a bare decimal.
///
/// The sign is only claimed where it is *stuck to* a number, which is what
/// keeps the remainder operator: `total % 3` and `10 % 3` are unchanged,
/// because a space is not a percentage sign, and `10 %% 3` says remainder
/// with no space needed. The one spelling that lost its old meaning is
/// `10%3` — no space, no doubling — and rather than pick for the author,
/// that is refused by name.
fn read_percent_tail(
    number: &str,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<bool, CoreError> {
    let mut lookahead = chars.clone();
    if lookahead.next() != Some('%') {
        return Ok(false);
    }
    // `10%%3` is the remainder, so the doubled sign is left for the operator
    // branch to read rather than eaten half here.
    if lookahead.peek() == Some(&'%') {
        return Ok(false);
    }
    if lookahead
        .peek()
        .is_some_and(|next| next.is_ascii_digit() || *next == '.')
    {
        return Err(CoreError::Formula(format!(
            "‘{number}%’ is a percentage, so ‘{number}%…’ reads as one number \
             followed by another. Write ‘{number} % …’ or ‘{number} %% …’ for \
             the remainder."
        )));
    }
    *chars = lookahead;
    Ok(true)
}

/// Reads the rest of `YYYY-MM-DD` once the year has been taken as digits.
///
/// The shape is the whole test, and it is a strict one: exactly four
/// digits, then `-` and two digits, twice, and nothing numeric after. That
/// is narrow enough that no arithmetic anybody writes can fall into it —
/// `2026 - 08 - 12` with spaces is still subtraction, and so is `2026-8-12`
/// — while `2026-08-12` stops meaning 2006, which is what it used to
/// quietly evaluate to.
///
/// Only a string that *is* this shape but names an impossible day gets an
/// error. Anything else is handed back for the number branch to keep.
fn read_date_tail(
    digits: &str,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<Option<NaiveDate>, CoreError> {
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    let mut lookahead = chars.clone();
    let mut tail = String::new();
    for _ in 0..2 {
        if lookahead.next() != Some('-') {
            return Ok(None);
        }
        tail.push('-');
        for _ in 0..2 {
            match lookahead.next() {
                Some(digit) if digit.is_ascii_digit() => tail.push(digit),
                _ => return Ok(None),
            }
        }
    }
    if lookahead.peek().is_some_and(char::is_ascii_digit) {
        return Ok(None);
    }
    let text = format!("{digits}{tail}");
    let date = NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .map_err(|_| CoreError::Formula(format!("‘{text}’ is not a real date")))?;
    *chars = lookahead;
    Ok(Some(date))
}

/// Reads the unit off a duration once the count has been taken as digits.
///
/// The unit runs to the end of the word rather than to the end of the
/// longest match, so `30days` is one unit this either knows or does not —
/// never `30d` with `ays` left lying around. A word it does not know is
/// handed back untouched, which leaves `2 x` reading exactly as it did.
fn read_duration_tail(
    digits: &str,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<Option<String>, CoreError> {
    if !chars.peek().is_some_and(char::is_ascii_alphabetic) {
        return Ok(None);
    }
    let mut lookahead = chars.clone();
    let mut written = String::new();
    while lookahead
        .peek()
        .is_some_and(|next| next.is_alphanumeric() || *next == '_')
    {
        written.push(lookahead.next().unwrap());
    }
    let Some((_, unit)) = DURATION_UNITS
        .iter()
        .find(|(spelling, _)| spelling.eq_ignore_ascii_case(&written))
    else {
        return Ok(None);
    };
    if digits.contains('.') {
        return Err(CoreError::Formula(format!(
            "‘{digits}{written}’ is not a whole number of them"
        )));
    }
    *chars = lookahead;
    Ok(Some(format!("{digits}{unit}")))
}

pub(crate) fn read_formula_reference(
    first: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<FormulaReference, CoreError> {
    let mut path = vec![read_reference_name(first, chars)?];
    // A dot followed by a backtick continues the name; a dot followed by
    // anything else is a method call and belongs to the parser. That test is
    // the same at every depth, which is what lets this loop rather than
    // stopping at two.
    loop {
        let mut lookahead = chars.clone();
        if lookahead.next() != Some('.') || lookahead.next() != Some('`') {
            break;
        }
        chars.next();
        let next = chars
            .next()
            .ok_or_else(|| CoreError::Formula("Expected a name after ‘.’".into()))?;
        if !(next.is_alphabetic() || next == '_' || next == '`') {
            return Err(CoreError::Formula("Expected a name after ‘.’".into()));
        }
        path.push(read_reference_name(next, chars)?);
    }
    if path.len() == 1 {
        return Ok(FormulaReference::Unqualified(path.remove(0)));
    }
    Ok(FormulaReference::Qualified(path))
}

pub(crate) fn read_reference_name(
    first: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<ReferenceName, CoreError> {
    if first != '`' {
        let mut value = first.to_string();
        while chars
            .peek()
            .is_some_and(|next| next.is_alphanumeric() || *next == '_')
        {
            value.push(chars.next().unwrap());
        }
        return Ok(ReferenceName {
            value,
            exact: false,
        });
    }

    let mut value = String::new();
    loop {
        match chars.next() {
            Some('`') if chars.peek() == Some(&'`') => {
                chars.next();
                value.push('`');
            }
            Some('`') => break,
            Some(character) => value.push(character),
            None => {
                return Err(CoreError::Formula("Unclosed backtick reference".into()));
            }
        }
    }
    if value.is_empty() {
        return Err(CoreError::Formula(
            "Backtick references cannot be empty".into(),
        ));
    }
    Ok(ReferenceName { value, exact: true })
}
