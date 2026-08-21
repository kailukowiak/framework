use crate::error::CoreError;
use crate::formula::ast::{BinaryOperator, Expr};
use crate::formula::lexer::{FormulaReference, ReferenceName, Token, tokenize};
use crate::model::document::{DataObject, Document};
use crate::model::frame::{Column, FrameObject};
use crate::model::value::{BlockLine, BlockObject};
use crate::reference_matches;

type FormulaArguments = (Vec<Expr>, Vec<(String, Expr)>);

pub(crate) struct Parser<'a> {
    tokens: Vec<Token>,
    position: usize,
    frame: &'a FrameObject,
    /// The block this formula sits in, when it sits in one. Its lines are
    /// the innermost scope: a bare name lands on a sibling line before it
    /// lands on anything on the canvas, the way a column shadows a value
    /// inside a frame.
    block: Option<&'a BlockObject>,
    /// Whether this formula produces a value rather than a column: a result,
    /// or a line of a block.
    ///
    /// It decides one thing — whether a frame with no snapshot may be named.
    /// A column formula may not, because a frame reading another frame live
    /// could lead back round to itself. A scalar surface may name one:
    /// Scratchwork evaluates it live, while a one-value result may still ask
    /// for an explicit snapshot or recorded answer at evaluation time.
    scalar: bool,
    /// Whether this formula may evaluate to a list rather than one value.
    ///
    /// True on a block line and nowhere else. A column formula has to
    /// produce a column, a result has to produce a result, and a scratchpad
    /// line holds whatever it works out to — `4 * `Rates`` is three numbers,
    /// and refusing that is refusing arithmetic somebody meant.
    lists: bool,
    document: &'a Document,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(
        source: &str,
        frame: &'a FrameObject,
        document: &'a Document,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            tokens: tokenize(source)?,
            position: 0,
            frame,
            block: None,
            scalar: false,
            lists: false,
            document,
        })
    }

    /// A parser for a formula that produces a value rather than a column: no
    /// frame to take bare column names from, and frames with no snapshot may
    /// be named.
    pub(crate) fn new_scalar(
        source: &str,
        frame: &'a FrameObject,
        document: &'a Document,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            tokens: tokenize(source)?,
            position: 0,
            frame,
            block: None,
            scalar: true,
            lists: false,
            document,
        })
    }

    /// The scalar scope with lists welcome: what a generated frame's rule is
    /// parsed with. Same resolution as `new_scalar` — no bare columns — but
    /// a list-shaped answer is the point rather than a mistake, exactly as
    /// on a block line.
    pub(crate) fn new_scalar_list(
        source: &str,
        frame: &'a FrameObject,
        document: &'a Document,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            tokens: tokenize(source)?,
            position: 0,
            frame,
            block: None,
            scalar: true,
            lists: true,
            document,
        })
    }

    /// A parser for a formula on one of `block`'s lines: the scalar scope —
    /// an empty frame, so no bare columns — with the block's own lines in
    /// front of it.
    pub(crate) fn new_in_block(
        source: &str,
        block: &'a BlockObject,
        frame: &'a FrameObject,
        document: &'a Document,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            tokens: tokenize(source)?,
            position: 0,
            frame,
            block: Some(block),
            scalar: true,
            lists: true,
            document,
        })
    }

    pub(crate) fn parse(mut self) -> Result<Expr, CoreError> {
        let expression = self.parse_expression(0)?;
        if self.peek() != &Token::End {
            return Err(CoreError::Formula(
                "Unexpected text at end of formula".into(),
            ));
        }
        expression.validate_list_placement(self.document, self.lists)?;
        expression.validate_comparison_types_among(self.document, &self.frame.columns)?;
        Ok(expression)
    }

    fn parse_expression(&mut self, minimum_precedence: u8) -> Result<Expr, CoreError> {
        let primary = match self.next() {
            Token::Integer(value) => Expr::Integer { value },
            Token::Number(value) => Expr::Number { value },
            Token::Percentage(value) => Expr::Percentage { value },
            Token::Money(value) => Expr::Money { value },
            Token::Date(value) => Expr::Date { value },
            Token::Duration(value) => Expr::Duration { value },
            Token::String(value) => Expr::String { value },
            Token::Identifier(reference) => match &reference {
                FormulaReference::Unqualified(name)
                    if !name.exact && self.peek() == &Token::LeftParen =>
                {
                    self.parse_function_call(name)?
                }
                FormulaReference::Unqualified(name)
                    if !name.exact
                        && name.value.eq_ignore_ascii_case("frame")
                        && self.peek() == &Token::Dot =>
                {
                    self.parse_current_frame_call()?
                }
                FormulaReference::Unqualified(name)
                    if !name.exact && name.value.eq_ignore_ascii_case("true") =>
                {
                    Expr::Boolean { value: true }
                }
                FormulaReference::Unqualified(name)
                    if !name.exact && name.value.eq_ignore_ascii_case("false") =>
                {
                    Expr::Boolean { value: false }
                }
                FormulaReference::Unqualified(name)
                    if !name.exact
                        && (name.value.eq_ignore_ascii_case("null")
                            || name.value.eq_ignore_ascii_case("none")) =>
                {
                    Expr::Null
                }
                FormulaReference::Unqualified(name) if self.block_member_follows(name) => {
                    self.parse_block_member(name)?
                }
                _ => self.resolve_identifier(&reference)?,
            },
            Token::Minus => Expr::Negate {
                expression: Box::new(self.parse_expression(6)?),
            },
            Token::Tilde => Expr::Not {
                expression: Box::new(self.parse_expression(6)?),
            },
            Token::LeftParen => {
                let expression = self.parse_expression(0)?;
                if self.next() != Token::RightParen {
                    return Err(CoreError::Formula("Missing closing parenthesis".into()));
                }
                expression
            }
            Token::LeftBracket => self.parse_expression_list()?,
            _ => {
                return Err(CoreError::Formula(
                    "Expected a literal, backtick column, value, or Polars function".into(),
                ));
            }
        };
        let mut left = self.parse_postfix(primary)?;

        loop {
            let (operator, precedence, right_associative) = match self.peek() {
                Token::Or => (BinaryOperator::Or, 1, false),
                Token::And => (BinaryOperator::And, 2, false),
                Token::EqualEqual => (BinaryOperator::Equal, 3, false),
                Token::NotEqual => (BinaryOperator::NotEqual, 3, false),
                Token::Less => (BinaryOperator::Less, 3, false),
                Token::LessEqual => (BinaryOperator::LessEqual, 3, false),
                Token::Greater => (BinaryOperator::Greater, 3, false),
                Token::GreaterEqual => (BinaryOperator::GreaterEqual, 3, false),
                Token::Plus => (BinaryOperator::Add, 4, false),
                Token::Minus => (BinaryOperator::Subtract, 4, false),
                Token::Star => (BinaryOperator::Multiply, 5, false),
                Token::Slash => (BinaryOperator::Divide, 5, false),
                Token::SlashSlash => (BinaryOperator::FloorDivide, 5, false),
                Token::Percent => (BinaryOperator::Modulo, 5, false),
                Token::StarStar => (BinaryOperator::Power, 7, true),
                _ => break,
            };
            if precedence < minimum_precedence {
                break;
            }
            self.next();
            let right = self.parse_expression(if right_associative {
                precedence
            } else {
                precedence + 1
            })?;
            left = Expr::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_postfix(&mut self, mut input: Expr) -> Result<Expr, CoreError> {
        while self.peek() == &Token::Dot {
            self.next();
            let mut path = Vec::new();
            loop {
                let Token::Identifier(FormulaReference::Unqualified(name)) = self.next() else {
                    return Err(CoreError::Formula(
                        "Expected a Polars method or namespace after ‘.’".into(),
                    ));
                };
                if name.exact {
                    return Err(CoreError::Formula(
                        "Polars method and namespace names cannot use backticks".into(),
                    ));
                }
                path.push(name.value);
                if self.peek() != &Token::Dot {
                    break;
                }
                self.next();
            }
            if self.next() != Token::LeftParen {
                return Err(CoreError::Formula(format!(
                    "Expected ‘(’ after .{}",
                    path.join(".")
                )));
            }
            let (arguments, keyword_arguments) = self.parse_arguments()?;
            input = Expr::Method {
                input: Box::new(input),
                path,
                arguments,
                keyword_arguments,
            };
        }
        Ok(input)
    }

    fn resolve_identifier(&self, reference: &FormulaReference) -> Result<Expr, CoreError> {
        if let FormulaReference::Qualified(path) = reference {
            // What the first name is decides which kind of path this is.
            // A container is walked through, however deep it goes; anything
            // else is a frame and a column of it, which is exactly two.
            let head = &path[0];
            if let Some(container) = self.document.objects.iter().find(|object| {
                matches!(object, DataObject::Container(_)) && reference_matches(object.name(), head)
            }) {
                return self.resolve_through_container(container, &path[1..]);
            }
            if let Some(block) = self
                .document
                .objects
                .iter()
                .find_map(|object| match object {
                    DataObject::Block(block) if reference_matches(&block.name, head) => Some(block),
                    _ => None,
                })
            {
                return Self::resolve_block_line(block, &path[1..]);
            }
            let [frame, column] = path.as_slice() else {
                return Err(CoreError::Formula(format!(
                    "‘{}’ is not a container, so ‘{}’ names one thing too many",
                    head.value,
                    path.iter()
                        .map(|name| name.value.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                )));
            };
            if !reference_matches(&self.frame.name, frame) {
                return self.resolve_foreign_column(frame, column);
            }
            let matches: Vec<&Column> = self
                .frame
                .columns
                .iter()
                .filter(|candidate| reference_matches(&candidate.name, column))
                .collect();
            return match matches.as_slice() {
                [column] => Ok(Expr::Column {
                    column_id: column.id.clone(),
                }),
                [] => Err(CoreError::Formula(format!(
                    "Unknown column ‘{}’ in ‘{}’",
                    column.value, self.frame.name
                ))),
                _ => Err(CoreError::Formula(format!(
                    "Ambiguous column name ‘{}’",
                    column.value
                ))),
            };
        }

        let FormulaReference::Unqualified(name) = reference else {
            unreachable!();
        };

        // Inside a block, sibling lines resolve bare — that is what makes a
        // scratchpad writable without qualifying every step of the working,
        // and it is why this is reached before the backtick rule below: a
        // line called `x` is named `x`, not `` `x` ``. Nothing is shadowed
        // by allowing it, because a block sits in no frame and a bare name
        // in that scope resolves to nothing else today.
        //
        // Whether the line named is *above* this one is settled when the
        // block is computed, which can say which line is below which; here
        // a later line still resolves, so the complaint can name it.
        if let Some(block) = self.block {
            let lines: Vec<_> = block
                .lines
                .iter()
                .filter(|line| !line.name.is_empty() && reference_matches(&line.name, name))
                .collect();
            match lines.as_slice() {
                [line] => {
                    return Ok(Expr::Value {
                        object_id: line.id.clone(),
                    });
                }
                [] => {}
                _ => {
                    return Err(CoreError::Formula(format!(
                        "Ambiguous line name ‘{}’",
                        name.value
                    )));
                }
            }
        }

        if !name.exact {
            return Err(CoreError::Formula(format!(
                "Unknown Polars name ‘{}’; wrap column and canvas-value names in backticks",
                name.value
            )));
        }
        let columns: Vec<&Column> = self
            .frame
            .columns
            .iter()
            .filter(|column| reference_matches(&column.name, name))
            .collect();
        match columns.as_slice() {
            [column] => {
                return Ok(Expr::Column {
                    column_id: column.id.clone(),
                });
            }
            [_, ..] => {
                return Err(CoreError::Formula(format!(
                    "Ambiguous column name ‘{}’",
                    name.value
                )));
            }
            [] => {}
        }

        let values: Vec<&DataObject> = self
            .document
            .objects
            .iter()
            .filter(|object| {
                matches!(
                    object,
                    DataObject::Value(_) | DataObject::Result(_) | DataObject::Series(_)
                ) && reference_matches(object.name(), name)
            })
            .collect();
        match values.as_slice() {
            [DataObject::Series(series)] => Ok(Expr::Series {
                object_id: series.id.clone(),
            }),
            [object] => Ok(Expr::Value {
                object_id: object.id().into(),
            }),
            [] => {
                // A block named on its own is its value, when it holds one:
                // the person who made a card called "Timesheet date" with
                // one date in it made *a named value*, and every formula
                // outside should get to say so in one name. A block of many
                // lines stays a place, and says which lines it offers.
                if let Some(block) = self
                    .document
                    .objects
                    .iter()
                    .find_map(|object| match object {
                        DataObject::Block(block) if reference_matches(&block.name, name) => {
                            Some(block)
                        }
                        _ => None,
                    })
                {
                    if let Some(line) = Self::single_line_of(block) {
                        return Ok(Expr::Value {
                            object_id: line.id.clone(),
                        });
                    }
                    return Err(CoreError::Formula(format!(
                        "‘{}’ is a formula block. Name one of its lines.",
                        block.name
                    )));
                }
                // A named line is addressable from anywhere its name is
                // unambiguous — requiring the block qualifier for a name
                // that exists exactly once was a toll with nobody to pay.
                let lines: Vec<(&str, &BlockLine)> = self
                    .document
                    .objects
                    .iter()
                    .filter_map(|object| match object {
                        DataObject::Block(block) => Some(block),
                        _ => None,
                    })
                    .flat_map(|block| {
                        block
                            .lines
                            .iter()
                            .filter(|line| {
                                !line.name.is_empty() && reference_matches(&line.name, name)
                            })
                            .map(move |line| (block.name.as_str(), line))
                    })
                    .collect();
                match lines.as_slice() {
                    [(_, line)] => {
                        return Ok(Expr::Value {
                            object_id: line.id.clone(),
                        });
                    }
                    [] => {}
                    several => {
                        return Err(CoreError::Formula(format!(
                            "‘{}’ names a line in more than one block ({}). Write \
                             the block's name in front.",
                            name.value,
                            several
                                .iter()
                                .map(|(block, _)| format!("‘{block}’"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )));
                    }
                }
                // A container named on its own is a place, not a value, and
                // saying so is more use than saying the name is unknown —
                // it is right there on the canvas.
                if let Some(container) = self.document.objects.iter().find(|object| {
                    matches!(object, DataObject::Container(_))
                        && reference_matches(object.name(), name)
                }) {
                    return Err(CoreError::Formula(format!(
                        "‘{}’ is a container. Name something in it.",
                        container.name()
                    )));
                }
                Err(CoreError::Formula(format!("Unknown name ‘{}’", name.value)))
            }
            _ => Err(CoreError::Formula(format!(
                "Ambiguous value name ‘{}’",
                name.value
            ))),
        }
    }

    /// Walks the rest of a dotted name down through a container.
    ///
    /// Each step looks only at what that container holds, which is the whole
    /// point of putting things in one: `` `Finance`.`Rate` `` and
    /// `` `Ops`.`Rate` `` are different numbers and neither is ambiguous.
    /// The last name has to land on something that is not itself a
    /// container, since a container is a place rather than a value.
    fn resolve_through_container(
        &self,
        container: &DataObject,
        rest: &[ReferenceName],
    ) -> Result<Expr, CoreError> {
        let Some((name, remaining)) = rest.split_first() else {
            return Err(CoreError::Formula(format!(
                "‘{}’ is a container. Name something in it.",
                container.name()
            )));
        };
        let DataObject::Container(container) = container else {
            unreachable!("only called with a container");
        };
        let matches: Vec<&DataObject> = container
            .member_ids
            .iter()
            .filter_map(|member_id| self.document.object(member_id).ok())
            .filter(|member| reference_matches(member.name(), name))
            .collect();
        let member = match matches.as_slice() {
            [member] => *member,
            [] => {
                return Err(CoreError::Formula(format!(
                    "‘{}’ holds nothing called ‘{}’",
                    container.name, name.value
                )));
            }
            _ => {
                return Err(CoreError::Formula(format!(
                    "‘{}’ holds more than one ‘{}’",
                    container.name, name.value
                )));
            }
        };
        if !remaining.is_empty() {
            return self.resolve_through_container(member, remaining);
        }
        match member {
            DataObject::Value(value) => Ok(Expr::Value {
                object_id: value.id.clone(),
            }),
            DataObject::Result(result) => Ok(Expr::Value {
                object_id: result.id.clone(),
            }),
            DataObject::Series(series) => Ok(Expr::Series {
                object_id: series.id.clone(),
            }),
            DataObject::Container(inner) => Err(CoreError::Formula(format!(
                "‘{}’ is a container. Name something in it.",
                inner.name
            ))),
            other => Err(CoreError::Formula(format!(
                "‘{}’ cannot be read from a formula",
                other.name()
            ))),
        }
    }

    /// The block this name would be naming, if it names one.
    ///
    /// Borrowed from the document rather than from `self`, so the caller can
    /// go on reading tokens while holding it.
    fn block_named(&self, name: &ReferenceName) -> Option<&'a BlockObject> {
        self.document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Block(block) if reference_matches(&block.name, name) => Some(block),
                _ => None,
            })
    }

    /// Whether what follows is `` `Assumptions`.rate `` — a block's name, a
    /// dot, and one of its lines written plainly.
    ///
    /// The lexer stops a dotted reference at the first name that is not
    /// backticked, because everywhere else in the language a bare name after
    /// a dot is a method or a namespace: `` `Amount`.round(2) ``,
    /// `` `Amount`.str.zfill(2) ``. That rule is right for a value and wrong
    /// for a block, because a block is not a value and has no methods at all
    /// — every dotted name on one is a line. So the parser takes this one
    /// case back off the lexer, which is also what lets the spelling in the
    /// spec, `` `general calculations`.account_balance ``, actually work.
    ///
    /// Two things still hold it back. A sibling line of the same name comes
    /// first, because inside a block a bare name has always meant a line and
    /// a block that happens to share the name should not change that. And a
    /// following `(` means a method call however the head resolves, which is
    /// the other half of the same collision.
    fn block_member_follows(&self, name: &ReferenceName) -> bool {
        self.peek() == &Token::Dot
            && !matches!(self.peek_n(2), Token::LeftParen)
            && !self.block.is_some_and(|block| {
                block
                    .lines
                    .iter()
                    .any(|line| !line.name.is_empty() && reference_matches(&line.name, name))
            })
            && self.block_named(name).is_some()
    }

    /// `` `Assumptions`.rate ``, once [`Self::block_member_follows`] has said
    /// that is what this is.
    fn parse_block_member(&mut self, head: &ReferenceName) -> Result<Expr, CoreError> {
        let block = self
            .block_named(head)
            .expect("block_member_follows found the block");
        // A block holding one value *is* that value to everyone outside it,
        // so a dotted name that matches none of its lines is not a line
        // lookup gone wrong — it is a method on the value: `` `Timesheet
        // date`.dt.month_start() ``. Nothing is consumed here; the ordinary
        // postfix loop reads the `.dt.month_start(...)` chain exactly as it
        // would after any other value. A block with several lines keeps the
        // strict reading, because there the dotted name has to say which
        // line is meant — and an explicit backticked segment is always a
        // line lookup, so a typo in one still says so.
        if let Token::Identifier(FormulaReference::Unqualified(segment)) = self.peek_n(1)
            && !segment.exact
            && !block
                .lines
                .iter()
                .any(|line| reference_matches(&line.name, segment))
            && let Some(line) = Self::single_line_of(block)
        {
            return Ok(Expr::Value {
                object_id: line.id.clone(),
            });
        }
        self.next();
        let Token::Identifier(FormulaReference::Unqualified(line)) = self.next() else {
            return Err(CoreError::Formula(format!(
                "Expected a line of ‘{}’ after ‘.’",
                block.name
            )));
        };
        Self::resolve_block_line(block, &[line])
    }

    /// The one line a block holds, when it holds exactly one that computes.
    /// Blank and comment lines do not count against it: a heading and a
    /// value still make "one named thing" to everyone outside the block.
    fn single_line_of(block: &BlockObject) -> Option<&BlockLine> {
        let mut computing = block
            .lines
            .iter()
            .filter(|line| line.expression().is_some());
        match (computing.next(), computing.next()) {
            (Some(line), None) => Some(line),
            _ => None,
        }
    }

    /// The rest of a dotted name landing on one of a block's lines.
    ///
    /// Exactly one name deep, always: a block holds lines and a line holds
    /// nothing further, so `` `Calcs`.`balance` `` is the whole shape. The
    /// reference travels as the line's id, exactly as a result's does, so
    /// renaming the line or the block breaks nothing.
    fn resolve_block_line(block: &BlockObject, rest: &[ReferenceName]) -> Result<Expr, CoreError> {
        let [name] = rest else {
            return Err(CoreError::Formula(if rest.is_empty() {
                format!(
                    "‘{}’ is a formula block. Name one of its lines.",
                    block.name
                )
            } else {
                format!(
                    "‘{}’ is a formula block, so ‘{}’ names one thing too many",
                    block.name,
                    rest.iter()
                        .map(|name| name.value.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                )
            }));
        };
        let matches: Vec<_> = block
            .lines
            .iter()
            .filter(|line| reference_matches(&line.name, name))
            .collect();
        match matches.as_slice() {
            [line] => Ok(Expr::Value {
                object_id: line.id.clone(),
            }),
            [] => Err(CoreError::Formula(format!(
                "‘{}’ has no line called ‘{}’",
                block.name, name.value
            ))),
            _ => Err(CoreError::Formula(format!(
                "‘{}’ has more than one line called ‘{}’",
                block.name, name.value
            ))),
        }
    }

    /// A column in some frame other than the one being written in.
    ///
    /// A frame formula may only use this when the other frame holds a
    /// snapshot. A scalar surface may read the current frame directly; the
    /// evaluator then applies the different Result and Scratchwork contracts.
    fn resolve_foreign_column(
        &self,
        frame: &ReferenceName,
        column: &ReferenceName,
    ) -> Result<Expr, CoreError> {
        let candidates: Vec<&FrameObject> = self
            .document
            .objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Frame(candidate) if reference_matches(&candidate.name, frame) => {
                    Some(candidate)
                }
                _ => None,
            })
            .collect();
        let other = match candidates.as_slice() {
            [other] => other,
            [] => {
                return Err(CoreError::Formula(format!(
                    "Unknown frame ‘{}’",
                    frame.value
                )));
            }
            _ => {
                return Err(CoreError::Formula(format!(
                    "Ambiguous frame name ‘{}’",
                    frame.value
                )));
            }
        };
        // A column formula may only read a frame that holds a snapshot: two
        // frames reading each other live is a loop with nothing to stop it,
        // and a snapshot is where a lineage ends rather than continues.
        // Scalar surfaces may name the live frame; Scratchwork evaluates that
        // semantic query directly, while Result keeps its one-value boundary.
        if !self.scalar && other.materialization.is_none() {
            return Err(CoreError::Formula(format!(
                "‘{}’ has to be materialized before another frame can read from it. \
                 Materialize it, and this reference will work.",
                other.name
            )));
        }
        let matches: Vec<&Column> = other
            .columns
            .iter()
            .filter(|candidate| reference_matches(&candidate.name, column))
            .collect();
        match matches.as_slice() {
            [found] => Ok(Expr::ForeignColumn {
                frame_id: other.id.clone(),
                column_id: found.id.clone(),
            }),
            [] => Err(CoreError::Formula(format!(
                "Unknown column ‘{}’ in ‘{}’",
                column.value, other.name
            ))),
            _ => Err(CoreError::Formula(format!(
                "Ambiguous column name ‘{}’ in ‘{}’",
                column.value, other.name
            ))),
        }
    }

    fn parse_function_call(&mut self, name: &ReferenceName) -> Result<Expr, CoreError> {
        self.next();
        let (arguments, keyword_arguments) = self.parse_arguments()?;
        Ok(Expr::PolarsCall {
            name: name.value.clone(),
            arguments,
            keyword_arguments,
        })
    }

    /// Parse the one virtual value owned by a frame formula: `frame.len()`.
    ///
    /// It is represented as a root call internally because there is no
    /// pretend frame object to compile or persist. Keeping the public spelling
    /// namespaced matters, though: `len()` beside a column formula looks like
    /// it might count a column, while `frame.len()` says exactly which rows
    /// determine a generated sequence.
    fn parse_current_frame_call(&mut self) -> Result<Expr, CoreError> {
        if self.scalar || self.frame.id.is_empty() {
            return Err(CoreError::Formula(
                "frame.len() is only available in a frame transformation".into(),
            ));
        }
        self.next();
        let Token::Identifier(FormulaReference::Unqualified(method)) = self.next() else {
            return Err(CoreError::Formula("Expected len after frame.".into()));
        };
        if method.exact
            || !(method.value.eq_ignore_ascii_case("len")
                || method.value.eq_ignore_ascii_case("n_rows"))
        {
            return Err(CoreError::Formula(
                "frame currently provides frame.len() (also written frame.n_rows())".into(),
            ));
        }
        if self.next() != Token::LeftParen || self.next() != Token::RightParen {
            return Err(CoreError::Formula(
                "frame.len() and frame.n_rows() take no arguments".into(),
            ));
        }
        Ok(Expr::PolarsCall {
            name: "frame_len".into(),
            arguments: Vec::new(),
            keyword_arguments: Vec::new(),
        })
    }

    fn parse_arguments(&mut self) -> Result<FormulaArguments, CoreError> {
        let mut arguments = Vec::new();
        let mut keyword_arguments = Vec::new();
        if self.peek() == &Token::RightParen {
            self.next();
            return Ok((arguments, keyword_arguments));
        }
        loop {
            let is_keyword = matches!(self.peek(), Token::Identifier(FormulaReference::Unqualified(name)) if !name.exact)
                && self.peek_n(1) == &Token::Equal;
            if is_keyword {
                let Token::Identifier(FormulaReference::Unqualified(name)) = self.next() else {
                    unreachable!()
                };
                self.next();
                keyword_arguments.push((name.value, self.parse_expression(0)?));
            } else {
                if !keyword_arguments.is_empty() {
                    return Err(CoreError::Formula(
                        "Positional arguments must come before keyword arguments".into(),
                    ));
                }
                arguments.push(self.parse_expression(0)?);
            }
            match self.peek() {
                Token::Comma => {
                    self.next();
                    if self.peek() == &Token::RightParen {
                        self.next();
                        break;
                    }
                }
                Token::RightParen => {
                    self.next();
                    break;
                }
                _ => {
                    return Err(CoreError::Formula(
                        "Expected ‘,’ or ‘)’ in argument list".into(),
                    ));
                }
            }
        }
        Ok((arguments, keyword_arguments))
    }

    fn parse_expression_list(&mut self) -> Result<Expr, CoreError> {
        let mut items = Vec::new();
        if self.peek() == &Token::RightBracket {
            return Err(CoreError::Formula(
                "Expression lists cannot be empty".into(),
            ));
        }
        loop {
            let item = self.parse_expression(0)?;
            if matches!(item, Expr::List { .. }) {
                return Err(CoreError::Formula(
                    "Nested expression lists are not supported".into(),
                ));
            }
            items.push(item);
            match self.peek() {
                Token::Comma => {
                    self.next();
                }
                Token::RightBracket => {
                    self.next();
                    break;
                }
                _ => {
                    return Err(CoreError::Formula(
                        "Expected ‘,’ or ‘]’ in expression list".into(),
                    ));
                }
            }
        }
        Ok(Expr::List { items })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::End)
    }

    fn next(&mut self) -> Token {
        let token = self.peek().clone();
        self.position += 1;
        token
    }

    fn peek_n(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.position + offset)
            .unwrap_or(&Token::End)
    }
}
