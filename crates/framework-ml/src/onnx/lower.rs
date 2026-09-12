use super::{ImportedLinear, Result, attributes as a, ensure, proto::Graph};
use crate::pipeline_types::{
    ClassLabel, Dtype, Imputation, Input, MissingValue, NumericPrecision, Recipe, Role, Scalar,
    ScaleOperation, ScaleParameters, Step, UnknownCategory,
};
use std::collections::HashSet;

fn constants(g: &Graph, categories: usize) -> Result<()> {
    let zero = &g.initializers[0];
    let shape = &g.initializers[1];
    ensure(
        zero.dtype == Some(7) && zero.dims == [1] && zero.ints == [0],
        "unsupported zero initializer",
    )?;
    ensure(
        shape.dtype == Some(7) && shape.dims == [2] && shape.ints == [-1, categories as i64],
        "unsupported reshape initializer",
    )
}

fn categorical(g: &Graph) -> Result<(String, String, Vec<String>)> {
    let n = &g.nodes;
    let categories = a::strings(&n[10], "cats_strings")?;
    ensure(
        !categories.is_empty() && categories.len() <= 32,
        "category count limit (1..32)",
    )?;
    ensure(
        categories.iter().collect::<HashSet<_>>().len() == categories.len(),
        "duplicate categories",
    )?;
    let fill = a::string(&n[1], "default_string")?;
    ensure(
        a::get(&n[1], "keys_int64s")?.ints == [0]
            && a::strings(&n[1], "values_strings")? == [fill.clone()],
        "unsupported fill-value motif",
    )?;
    a::integer(&n[4], "default_int64", 0)?;
    let sentinels = a::strings(&n[4], "keys_strings")?;
    ensure(
        sentinels.len() == 1 && a::get(&n[4], "values_int64s")?.ints == [1],
        "unsupported missing sentinel motif",
    )?;
    constants(g, categories.len())?;
    Ok((fill, sentinels[0].clone(), categories))
}

fn numeric_recipe(g: &Graph, steps: &mut Vec<Step>) -> Result<()> {
    let replacement = a::get(&g.nodes[3], "replaced_value_float")?.float;
    ensure(
        replacement.is_some_and(f32::is_nan),
        "only NaN numeric missing policy is supported",
    )?;
    let impute = a::floats(&g.nodes[3], "imputed_value_floats", 2)?;
    let offset = a::floats(&g.nodes[5], "offset", 2)?;
    let scale = a::floats(&g.nodes[5], "scale", 2)?;
    ensure(
        scale.iter().all(|x| *x > 0.0),
        "nonpositive scale multiplier",
    )?;
    for i in 0..2 {
        steps.push(Step::Impute {
            input: format!("input:{i}"),
            output: format!("imputed:{i}"),
            strategy: Imputation::Constant,
            missing: MissingValue::NaN,
            learned: Some(Scalar::Number(f64::from(impute[i]))),
        });
        steps.push(Step::Scale {
            input: format!("imputed:{i}"),
            output: format!("scaled:{i}"),
            learned: Some(ScaleParameters {
                offset: f64::from(offset[i]),
                scale: f64::from(scale[i]),
                operation: ScaleOperation::Multiply,
            }),
        });
    }
    Ok(())
}

pub fn lower(g: &Graph, classification: bool) -> Result<ImportedLinear> {
    let (fill, missing, categories) = categorical(g)?;
    let mut steps = Vec::new();
    numeric_recipe(g, &mut steps)?;
    steps.push(Step::Impute {
        input: "input:2".into(),
        output: "imputed:2".into(),
        strategy: Imputation::Constant,
        missing: MissingValue::String(missing),
        learned: Some(Scalar::String(fill)),
    });
    let outputs: Vec<_> = (0..categories.len())
        .map(|i| format!("encoded:{i}"))
        .collect();
    steps.push(Step::OneHot {
        input: "imputed:2".into(),
        outputs: outputs.clone(),
        unknown: UnknownCategory::AllZero,
        learned: Some(categories),
    });
    let features: Vec<_> = ["scaled:0".into(), "scaled:1".into()]
        .into_iter()
        .chain(outputs)
        .collect();
    let estimator = &g.nodes[13];
    let count = if classification { 2 } else { 1 };
    let coefficients = a::floats(estimator, "coefficients", count * features.len())?;
    let intercepts = a::floats(estimator, "intercepts", count)?;
    let labels = if classification {
        a::integer(estimator, "multi_class", 0)?;
        ensure(
            a::string(estimator, "post_transform")? == "LOGISTIC",
            "unsupported post-transform",
        )?;
        let labels = a::strings(estimator, "classlabels_strings")?;
        ensure(
            labels.len() == 2 && labels[0] != labels[1],
            "expected two distinct string classes",
        )?;
        // This narrow sklearn binary motif stores opposite coefficient rows.
        // Reject general OVR matrices rather than pretending their probabilities
        // obey the binary complement/decision semantics implemented here.
        ensure(
            intercepts[0] == -intercepts[1]
                && coefficients[..features.len()]
                    .iter()
                    .zip(&coefficients[features.len()..])
                    .all(|(a, b)| *a == -*b),
            "unsupported binary coefficient layout",
        )?;
        labels.into_iter().map(ClassLabel::String).collect()
    } else {
        Vec::new()
    };
    let inputs = g
        .inputs
        .iter()
        .enumerate()
        .map(|(i, value)| Input {
            slot: format!("input:{i}"),
            name: value.name.clone().unwrap_or_default(),
            dtype: if i == 2 { Dtype::String } else { Dtype::Number },
            role: Role::Predictor,
            binding: None,
        })
        .collect();
    Ok(ImportedLinear {
        inputs,
        recipe: Recipe {
            precision: NumericPrecision::Float32,
            steps,
            features,
        },
        coefficients,
        intercepts,
        labels,
    })
}
