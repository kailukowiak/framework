use super::*;

impl Parser<'_> {
    /// Keywords and static namespaces are resolved before document names;
    /// quoting a name keeps the document lookup path available.
    pub(super) fn parse_identifier(
        &mut self,
        reference: &FormulaReference,
    ) -> Result<Expr, CoreError> {
        Ok(match reference {
            FormulaReference::Unqualified(name)
                if !name.exact && self.peek() == &Token::LeftParen =>
            {
                self.parse_function_call(name)?
            }
            FormulaReference::Unqualified(name)
                if !name.exact
                    && name.value.eq_ignore_ascii_case("finance")
                    && self.peek() == &Token::Dot =>
            {
                self.parse_finance_call()?
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
            _ => self.resolve_identifier(reference)?,
        })
    }

    pub(super) fn parse_finance_call(&mut self) -> Result<Expr, CoreError> {
        self.next();
        let Token::Identifier(FormulaReference::Unqualified(method)) = self.next() else {
            return Err(CoreError::Formula(
                "Expected a financial function after finance.".into(),
            ));
        };
        if method.exact || !crate::formula::financial::is_financial(&method.value) {
            return Err(CoreError::Formula(format!(
                "Unknown financial function ‘{}’",
                method.value
            )));
        }
        if self.next() != Token::LeftParen {
            return Err(CoreError::Formula(format!(
                "Expected ‘(’ after finance.{}",
                method.value
            )));
        }
        let (arguments, keyword_arguments) = self.parse_arguments()?;
        Ok(Expr::PolarsCall {
            name: format!("finance.{}", method.value.to_ascii_lowercase()),
            arguments,
            keyword_arguments,
        })
    }

    pub(super) fn parse_current_frame_call(&mut self) -> Result<Expr, CoreError> {
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
}
