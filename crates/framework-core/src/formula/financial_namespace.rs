//! A namespace changes how a formula is written, not how it calculates. Keep
//! the receiver binding here and feed the existing financial compiler exactly
//! the same arguments, so defaults, errors and live dependencies cannot drift.
use crate::{Document, Expr};
use polars::prelude as pl;

pub(super) fn receiver_parameter(name: &str) -> &'static str {
    match name {
        "pv" => "pmt",
        "npv" | "xnpv" | "irr" | "xirr" | "mirr" => "values",
        "effect" => "nominal_rate",
        "nominal" => "effect_rate",
        "sln" | "db" | "ddb" => "cost",
        "period_index" => "date",
        "prior" => "expr",
        "ytd" | "ttm" | "same_period_last_year" => "expr",
        "fiscal_year" | "fiscal_quarter" | "fiscal_period" | "period_start" | "period_end"
        | "add_periods" => "date",
        _ => "pv",
    }
}

pub(super) fn compile_receiver(
    input: &Expr,
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    let name = name.to_ascii_lowercase();
    if !super::financial::is_financial(&name) {
        return Err(format!("Unknown financial function ‘{name}’"));
    }
    let receiver = receiver_parameter(&name);
    let parameters = super::financial::parameter_names(&name);
    let remaining: Vec<_> = parameters.iter().filter(|p| **p != receiver).collect();
    if arguments.len() > remaining.len() {
        return Err(format!(
            ".finance.{name} expects at most {} arguments",
            remaining.len()
        ));
    }
    let mut bound = vec![(receiver.to_string(), input.clone())];
    bound.extend(
        remaining
            .into_iter()
            .zip(arguments)
            .map(|(name, value)| ((*name).to_string(), value.clone())),
    );
    bound.extend_from_slice(keywords);
    super::financial::compile(&name, &[], &bound, document)
}
