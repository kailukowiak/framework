use super::{
    Error, Result, ensure,
    proto::{Attribute, Node},
};

pub fn check(node: &Node, wanted: &[(&str, i32)]) -> Result<()> {
    ensure(
        node.attributes.len() == wanted.len(),
        "unsupported attribute set",
    )?;
    for (name, kind) in wanted {
        let matches: Vec<_> = node
            .attributes
            .iter()
            .filter(|a| a.name.as_deref() == Some(name))
            .collect();
        ensure(
            matches.len() == 1,
            &format!("missing/duplicate attribute {name}"),
        )?;
        let attr = matches[0];
        ensure(attr.kind == Some(*kind), "incorrect attribute type")?;
        let populated = [
            attr.float.is_some(),
            attr.int.is_some(),
            attr.string.is_some(),
            !attr.floats.is_empty(),
            !attr.ints.is_empty(),
            !attr.strings.is_empty(),
        ];
        let expected = match kind {
            1 => 0,
            2 => 1,
            3 => 2,
            6 => 3,
            7 => 4,
            8 => 5,
            _ => return Err(Error("unsupported attribute type".into())),
        };
        ensure(
            populated
                .iter()
                .enumerate()
                .all(|(i, present)| *present == (i == expected)),
            "attribute must contain exactly its declared value",
        )?;
    }
    Ok(())
}

pub fn get<'a>(node: &'a Node, name: &str) -> Result<&'a Attribute> {
    node.attributes
        .iter()
        .find(|a| a.name.as_deref() == Some(name))
        .ok_or_else(|| Error(format!("missing attribute {name}")))
}

pub fn integer(node: &Node, name: &str, expected: i64) -> Result<()> {
    ensure(
        get(node, name)?.int == Some(expected),
        &format!("unsupported {name}"),
    )
}

pub fn floats(node: &Node, name: &str, count: usize) -> Result<Vec<f32>> {
    let values = &get(node, name)?.floats;
    ensure(
        values.len() == count && values.iter().all(|x| x.is_finite()),
        &format!("invalid {name} shape or nonfinite parameter"),
    )?;
    Ok(values.clone())
}

pub fn strings(node: &Node, name: &str) -> Result<Vec<String>> {
    get(node, name)?
        .strings
        .iter()
        .map(|bytes| {
            String::from_utf8(bytes.clone()).map_err(|_| Error(format!("invalid UTF-8 in {name}")))
        })
        .collect()
}

pub fn string(node: &Node, name: &str) -> Result<String> {
    String::from_utf8(
        get(node, name)?
            .string
            .clone()
            .ok_or_else(|| Error("missing string".into()))?,
    )
    .map_err(|_| Error(format!("invalid UTF-8 in {name}")))
}
