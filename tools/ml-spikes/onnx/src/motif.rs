//! Only the observed two-numeric/one-string branched motif is accepted. Node
//! names may change; edges, types and attribute semantics must still agree.
use crate::{
    Result, attributes as a, ensure,
    proto::{Graph, Model, Value},
};
use std::collections::HashSet;

const OPS: [(&str, &str); 13] = [
    ("", "Concat"),
    ("ai.onnx.ml", "LabelEncoder"),
    ("ai.onnx.ml", "ArrayFeatureExtractor"),
    ("ai.onnx.ml", "Imputer"),
    ("ai.onnx.ml", "LabelEncoder"),
    ("ai.onnx.ml", "Scaler"),
    ("", "Cast"),
    ("", "Where"),
    ("", "Concat"),
    ("", "Gather"),
    ("ai.onnx.ml", "OneHotEncoder"),
    ("", "Reshape"),
    ("", "Concat"),
];

fn tensor(value: &Value, dtype: i32, width: Option<i64>) -> Result<()> {
    let tensor = value
        .r#type
        .as_ref()
        .and_then(|t| t.tensor.as_ref())
        .ok_or_else(|| crate::Error("missing tensor type".into()))?;
    ensure(tensor.elem_type == Some(dtype), "unsupported tensor dtype")?;
    let dims = &tensor
        .shape
        .as_ref()
        .ok_or_else(|| crate::Error("missing shape".into()))?
        .dims;
    let expected: Vec<_> = std::iter::once(None).chain(width.map(Some)).collect();
    ensure(
        dims.iter().map(|d| d.value).collect::<Vec<_>>() == expected,
        "unsupported tensor shape (expected dynamic rows)",
    )
}

fn names(graph: &Graph) -> Result<()> {
    let mut defined = HashSet::new();
    for name in graph
        .inputs
        .iter()
        .map(|v| v.name.as_deref())
        .chain(graph.initializers.iter().map(|v| v.name.as_deref()))
    {
        let name = name.ok_or_else(|| crate::Error("missing input/initializer name".into()))?;
        ensure(
            !name.is_empty() && defined.insert(name),
            "duplicate/empty graph name",
        )?;
    }
    for node in &graph.nodes {
        ensure(
            node.inputs.iter().all(|v| defined.contains(v.as_str())),
            "undefined or non-topological input",
        )?;
        for name in &node.outputs {
            ensure(
                !name.is_empty() && defined.insert(name),
                "duplicate/empty node output",
            )?;
        }
    }
    Ok(())
}

fn wiring(g: &Graph, classification: bool) -> Result<()> {
    let n = &g.nodes;
    let raw: Vec<_> = g
        .inputs
        .iter()
        .map(|v| v.name.as_deref().unwrap_or_default())
        .collect();
    let zero = g.initializers[0].name.as_deref().unwrap_or_default();
    let shape = g.initializers[1].name.as_deref().unwrap_or_default();
    let out = |i: usize| n[i].outputs[0].as_str();
    let expected = [
        vec![raw[0], raw[1]],
        vec![zero],
        vec![raw[2], zero],
        vec![out(0)],
        vec![out(2)],
        vec![out(3)],
        vec![out(4)],
        vec![out(6), out(1), out(2)],
        vec![out(7)],
        vec![out(8), zero],
        vec![out(9)],
        vec![out(10), shape],
        vec![out(5), out(11)],
        vec![out(12)],
    ];
    for (node, inputs) in n.iter().zip(expected) {
        ensure(
            node.inputs.iter().map(String::as_str).collect::<Vec<_>>() == inputs,
            "unsupported motif wiring",
        )?;
    }
    ensure(
        g.outputs.len() == if classification { 2 } else { 1 },
        "unsupported output count",
    )?;
    for (value, name) in g.outputs.iter().zip(&n[13].outputs) {
        ensure(
            value.name.as_ref() == Some(name),
            "incorrect graph output binding",
        )?;
    }
    if classification {
        tensor(&g.outputs[0], 8, None)?;
        tensor(&g.outputs[1], 1, Some(2))
    } else {
        tensor(&g.outputs[0], 1, Some(1))
    }
}

fn attributes(g: &Graph, classification: bool) -> Result<()> {
    let specs: [&[(&str, i32)]; 13] = [
        &[("axis", 2)],
        &[
            ("default_string", 3),
            ("keys_int64s", 7),
            ("values_strings", 8),
        ],
        &[],
        &[("imputed_value_floats", 6), ("replaced_value_float", 1)],
        &[
            ("default_int64", 2),
            ("keys_strings", 8),
            ("values_int64s", 7),
        ],
        &[("offset", 6), ("scale", 6)],
        &[("to", 2)],
        &[],
        &[("axis", 2)],
        &[("axis", 2)],
        &[("cats_strings", 8), ("zeros", 2)],
        &[],
        &[("axis", 2)],
    ];
    for (node, spec) in g.nodes.iter().zip(specs) {
        a::check(node, spec)?;
    }
    a::check(
        &g.nodes[13],
        if classification {
            &[
                ("classlabels_strings", 8),
                ("coefficients", 6),
                ("intercepts", 6),
                ("multi_class", 2),
                ("post_transform", 3),
            ]
        } else {
            &[("coefficients", 6), ("intercepts", 6)]
        },
    )?;
    for index in [0, 8, 9, 12] {
        a::integer(&g.nodes[index], "axis", 1)?;
    }
    a::integer(&g.nodes[6], "to", 9)?;
    a::integer(&g.nodes[10], "zeros", 1)
}

pub fn check(model: &Model) -> Result<(&Graph, bool)> {
    ensure(
        model.ir_version == Some(8),
        "unsupported ONNX IR version (expected 8)",
    )?;
    ensure(model.opsets.len() == 2, "unsupported opset imports")?;
    let ops: HashSet<_> = model
        .opsets
        .iter()
        .map(|x| (x.domain.as_deref().unwrap_or(""), x.version))
        .collect();
    ensure(
        ops == HashSet::from([("", Some(18)), ("ai.onnx.ml", Some(2))]),
        "unsupported opset versions",
    )?;
    let g = model
        .graph
        .as_ref()
        .ok_or_else(|| crate::Error("missing graph".into()))?;
    ensure(
        g.nodes.len() == 14 && g.inputs.len() == 3 && g.initializers.len() == 2,
        "unsupported branched motif size",
    )?;
    let classification = g.nodes[13].op.as_deref() == Some("LinearClassifier");
    for (index, node) in g.nodes.iter().enumerate() {
        let expected = if index < 13 {
            OPS[index]
        } else {
            (
                "ai.onnx.ml",
                if classification {
                    "LinearClassifier"
                } else {
                    "LinearRegressor"
                },
            )
        };
        ensure(
            (
                node.domain.as_deref().unwrap_or(""),
                node.op.as_deref().unwrap_or(""),
            ) == expected,
            &format!("unsupported operator/motif at node {index}: {:?}", node.op),
        )?;
        ensure(
            node.outputs.len() == if classification && index == 13 { 2 } else { 1 },
            "unsupported node output count",
        )?;
    }
    tensor(&g.inputs[0], 1, Some(1))?;
    tensor(&g.inputs[1], 1, Some(1))?;
    tensor(&g.inputs[2], 8, Some(1))?;
    names(g)?;
    wiring(g, classification)?;
    attributes(g, classification)?;
    Ok((g, classification))
}
