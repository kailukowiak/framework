//! Allocation-free preflight. A size limit alone does not stop prost creating
//! thousands of empty messages; field/message budgets bound that amplification.
//! The whitelist rejects unknown fields instead of relying on prost's skipping.
use crate::{Error, Result};

pub const MAX_BYTES: usize = 64 * 1024;
const MAX_FIELDS: usize = 4096;
const MAX_DEPTH: usize = 8;

#[derive(Clone, Copy, Debug)]
enum Schema {
    Model,
    Graph,
    Node,
    Attribute,
    Tensor,
    Value,
    Type,
    TensorType,
    Shape,
    Dimension,
    Opset,
}

// Wire kind, whether repeated, and nested schema. Packed numeric fields are
// accepted only where the pinned projection explicitly permits them.
fn field(schema: Schema, tag: u64) -> Option<(u64, bool, Option<Schema>)> {
    use Schema as S;
    let spec = match (schema, tag) {
        (S::Model, 1 | 5)
        | (S::Opset, 2)
        | (S::Attribute, 3 | 20)
        | (S::Tensor, 2)
        | (S::TensorType, 1)
        | (S::Dimension, 1) => (0, false, None),
        (S::Model, 2..=4 | 6)
        | (S::Graph, 2)
        | (S::Node, 3 | 4 | 7)
        | (S::Attribute, 1 | 4)
        | (S::Tensor, 8)
        | (S::Value, 1)
        | (S::Opset, 1) => (2, false, None),
        (S::Model, 7) => (2, false, Some(S::Graph)),
        (S::Model, 8) => (2, true, Some(S::Opset)),
        (S::Graph, 1) => (2, true, Some(S::Node)),
        (S::Graph, 5) => (2, true, Some(S::Tensor)),
        (S::Graph, 11 | 12) => (2, true, Some(S::Value)),
        (S::Node, 1 | 2) | (S::Attribute, 9) => (2, true, None),
        (S::Node, 5) => (2, true, Some(S::Attribute)),
        (S::Attribute, 2) => (5, false, None),
        (S::Attribute, 7) => (5, true, None),
        (S::Attribute, 8) | (S::Tensor, 1 | 7) => (0, true, None),
        (S::Value, 2) => (2, false, Some(S::Type)),
        (S::Type, 1) => (2, false, Some(S::TensorType)),
        (S::TensorType, 2) => (2, false, Some(S::Shape)),
        (S::Shape, 1) => (2, true, Some(S::Dimension)),
        _ => return None,
    };
    Some(spec)
}

fn varint(bytes: &mut &[u8]) -> Result<u64> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let (&byte, rest) = bytes
            .split_first()
            .ok_or_else(|| Error("truncated varint".into()))?;
        *bytes = rest;
        if shift == 63 && byte > 1 {
            return Err(Error("varint overflow".into()));
        }
        value |= u64::from(byte & 127) << shift;
        if byte < 128 {
            return Ok(value);
        }
    }
    Err(Error("varint overflow".into()))
}

fn take<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    let (value, rest) = bytes
        .split_at_checked(len)
        .ok_or_else(|| Error("truncated field".into()))?;
    *bytes = rest;
    Ok(value)
}

fn message(mut bytes: &[u8], schema: Schema, depth: usize, budget: &mut usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Error("protobuf depth limit".into()));
    }
    let mut seen = 0_u64;
    while !bytes.is_empty() {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| Error("protobuf field limit".into()))?;
        let key = varint(&mut bytes)?;
        let (tag, kind) = (key >> 3, key & 7);
        let (expected, repeated, child) = field(schema, tag)
            .ok_or_else(|| Error(format!("unsupported protobuf field {schema:?}.{tag}")))?;
        if !repeated && seen & (1 << tag) != 0 {
            return Err(Error(format!("duplicate protobuf field {schema:?}.{tag}")));
        }
        seen |= 1 << tag;
        let packed = repeated && matches!(expected, 0 | 5) && kind == 2;
        if kind != expected && !packed {
            return Err(Error("incorrect protobuf wire type".into()));
        }
        match kind {
            0 => {
                let value = varint(&mut bytes)?;
                if matches!(
                    (schema, tag),
                    (Schema::Attribute, 20) | (Schema::Tensor, 2) | (Schema::TensorType, 1)
                ) && value > i32::MAX as u64
                {
                    return Err(Error("invalid int32 discriminator".into()));
                }
            }
            5 => {
                take(&mut bytes, 4)?;
            }
            2 => {
                let len = usize::try_from(varint(&mut bytes)?)
                    .map_err(|_| Error("field size overflow".into()))?;
                let mut payload = take(&mut bytes, len)?;
                if let Some(child) = child {
                    message(payload, child, depth + 1, budget)?;
                } else if packed {
                    while !payload.is_empty() {
                        *budget = budget
                            .checked_sub(1)
                            .ok_or_else(|| Error("packed element limit".into()))?;
                        if expected == 0 {
                            varint(&mut payload)?;
                        } else {
                            take(&mut payload, 4)?;
                        }
                    }
                } else if len > 1024 {
                    return Err(Error("string/bytes length limit".into()));
                }
            }
            _ => return Err(Error("unsupported protobuf wire type".into())),
        }
    }
    Ok(())
}

pub fn check(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_BYTES {
        return Err(Error("ONNX byte limit (64 KiB)".into()));
    }
    let mut budget = MAX_FIELDS;
    message(bytes, Schema::Model, 0, &mut budget)
}

#[cfg(test)]
mod tests {
    #[test]
    fn discriminator_cannot_wrap_to_supported_int32() {
        // A decoder's int32 cast could otherwise turn 2^32 + 1 into FLOAT (1).
        let bytes = [0x08, 0x81, 0x80, 0x80, 0x80, 0x10];
        let mut budget = super::MAX_FIELDS;
        let result = super::message(&bytes, super::Schema::TensorType, 0, &mut budget);
        assert!(result.unwrap_err().0.contains("int32 discriminator"));
    }
}
