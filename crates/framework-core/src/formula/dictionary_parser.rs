//! A keyed lookup is an explicit live cross-frame operation. Permit its
//! references while parsing, then check the new dependency before compiling
//! a plan, when the draft is not yet part of the document graph.
use super::*;
impl Parser<'_> {
    pub(super) fn parse_function_call(&mut self, name: &ReferenceName) -> Result<Expr, CoreError> {
        self.next();
        let previous = self.mapping_arguments;
        let mapping = matches!(name.value.as_str(), "lookup" | "map_values");
        self.mapping_arguments |= mapping;
        let result = self.parse_arguments();
        self.mapping_arguments = previous;
        let (arguments, keyword_arguments) = result?;
        if mapping && !self.frame.id.is_empty() {
            for argument in &arguments {
                let mut frames = Vec::new();
                argument.foreign_frames(&mut frames);
                for frame in frames {
                    crate::formula::dictionary::check_dictionary_cycle(
                        self.document,
                        frame,
                        &self.frame.id,
                    )?;
                }
            }
        }
        Ok(Expr::PolarsCall {
            name: name.value.clone(),
            arguments,
            keyword_arguments,
        })
    }
}
