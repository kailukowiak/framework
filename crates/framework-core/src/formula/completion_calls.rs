/// The innermost call whose closing parenthesis is still to the right of the
/// cursor. This deliberately scans incomplete text rather than asking the
/// parser: parameter help is most useful between the opening parenthesis and
/// the moment the expression becomes valid.
pub(super) fn active_call_at_cursor(chars: &[char], cursor: usize) -> Option<(String, usize)> {
    #[derive(Debug)]
    enum Open {
        Call { spelling: String, argument: usize },
        Group,
        Bracket,
    }

    let mut open = Vec::new();
    let mut index = 0;
    while index < cursor {
        match chars[index] {
            '"' => {
                index += 1;
                while index < cursor {
                    if chars[index] == '\\' {
                        index = (index + 2).min(cursor);
                    } else if chars[index] == '"' {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            '`' => {
                index += 1;
                while index < cursor {
                    if chars[index] == '`' && chars.get(index + 1) == Some(&'`') {
                        index += 2;
                    } else if chars[index] == '`' {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            '(' => {
                let spelling = callable_before(chars, index);
                open.push(if spelling.is_empty() {
                    Open::Group
                } else {
                    Open::Call {
                        spelling,
                        argument: 0,
                    }
                });
            }
            '[' => open.push(Open::Bracket),
            ')' | ']' => {
                open.pop();
            }
            ',' => {
                if let Some(Open::Call { argument, .. }) = open.last_mut() {
                    *argument += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    open.into_iter().rev().find_map(|item| match item {
        Open::Call { spelling, argument } => Some((spelling, argument)),
        Open::Group | Open::Bracket => None,
    })
}

fn callable_before(chars: &[char], opening: usize) -> String {
    let mut end = opening;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == '.')
    {
        start -= 1;
    }
    let spelling: String = chars[start..end].iter().collect();
    if spelling.eq_ignore_ascii_case("frame.len")
        || spelling.to_ascii_lowercase().starts_with("finance.")
    {
        spelling
    } else if let Some(dot) = spelling.find('.') {
        spelling[dot..].to_string()
    } else {
        spelling
    }
}

pub(super) fn function_id_for_spelling(spelling: &str) -> Option<String> {
    let method = spelling.starts_with('.');
    let normalized = spelling.trim_start_matches('.').to_lowercase();
    crate::formula_function_catalog()
        .into_iter()
        .find(|function| {
            function.name.starts_with('.') == method
                && (function.name.trim_start_matches('.').to_lowercase() == normalized
                    || function
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase() == normalized))
        })
        .map(|function| function.id)
}
