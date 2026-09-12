use super::*;

pub(super) fn compile_from_expression(
    input: &Expr,
    path: &[String],
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Option<Result<pl::Expr, String>> {
    let compiled = match path {
        [namespace, name] if namespace.eq_ignore_ascii_case("finance") => {
            crate::formula::financial_namespace::compile_receiver(
                input,
                name,
                arguments,
                keyword_arguments,
                document,
            )
        }
        [one] if one == "otherwise" => {
            compile_when_chain(input, arguments, keyword_arguments, document)
        }
        [one] if one == "format" => {
            compile_format_method(input, arguments, keyword_arguments, document)
        }
        [one] if one == "cast" => compile_cast(input, arguments, keyword_arguments, document),
        [one] if one == "show" => compile_show(input, arguments, keyword_arguments, document),
        [one] if one == "at" => compile_at(input, arguments, keyword_arguments, document),
        [namespace, one] if namespace == "str" && one == "to_date" => {
            compile_string_to_date(input, arguments, keyword_arguments, document)
        }
        _ => return None,
    };
    Some(compiled)
}
