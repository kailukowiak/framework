//! Deliberately restricted wire projection of ONNX 1.18.0's Apache-2.0 schema.
//! Field numbers/types come from the vendored, hash-recorded upstream schema.
//! `wire` rejects every field outside this projection *before* prost decoding;
//! omitted graphs, external tensor data and functions cannot disappear silently.
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct Model {
    #[prost(int64, optional, tag = "1")]
    pub ir_version: Option<i64>,
    #[prost(string, optional, tag = "2")]
    pub producer_name: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub producer_version: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub domain: Option<String>,
    #[prost(int64, optional, tag = "5")]
    pub model_version: Option<i64>,
    #[prost(string, optional, tag = "6")]
    pub doc_string: Option<String>,
    #[prost(message, optional, tag = "7")]
    pub graph: Option<Graph>,
    #[prost(message, repeated, tag = "8")]
    pub opsets: Vec<Opset>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Opset {
    #[prost(string, optional, tag = "1")]
    pub domain: Option<String>,
    #[prost(int64, optional, tag = "2")]
    pub version: Option<i64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Graph {
    #[prost(message, repeated, tag = "1")]
    pub nodes: Vec<Node>,
    #[prost(string, optional, tag = "2")]
    pub name: Option<String>,
    #[prost(message, repeated, tag = "5")]
    pub initializers: Vec<Tensor>,
    #[prost(message, repeated, tag = "11")]
    pub inputs: Vec<Value>,
    #[prost(message, repeated, tag = "12")]
    pub outputs: Vec<Value>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Node {
    #[prost(string, repeated, tag = "1")]
    pub inputs: Vec<String>,
    #[prost(string, repeated, tag = "2")]
    pub outputs: Vec<String>,
    #[prost(string, optional, tag = "3")]
    pub name: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub op: Option<String>,
    #[prost(message, repeated, tag = "5")]
    pub attributes: Vec<Attribute>,
    #[prost(string, optional, tag = "7")]
    pub domain: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Attribute {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(float, optional, tag = "2")]
    pub float: Option<f32>,
    #[prost(int64, optional, tag = "3")]
    pub int: Option<i64>,
    #[prost(bytes = "vec", optional, tag = "4")]
    pub string: Option<Vec<u8>>,
    #[prost(float, repeated, packed = "false", tag = "7")]
    pub floats: Vec<f32>,
    #[prost(int64, repeated, packed = "false", tag = "8")]
    pub ints: Vec<i64>,
    #[prost(bytes = "vec", repeated, tag = "9")]
    pub strings: Vec<Vec<u8>>,
    #[prost(int32, optional, tag = "20")]
    pub kind: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Tensor {
    #[prost(int64, repeated, packed = "false", tag = "1")]
    pub dims: Vec<i64>,
    #[prost(int32, optional, tag = "2")]
    pub dtype: Option<i32>,
    #[prost(int64, repeated, tag = "7")]
    pub ints: Vec<i64>,
    #[prost(string, optional, tag = "8")]
    pub name: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Value {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(message, optional, tag = "2")]
    pub r#type: Option<Type>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Type {
    #[prost(message, optional, tag = "1")]
    pub tensor: Option<TensorType>,
}

#[derive(Clone, PartialEq, Message)]
pub struct TensorType {
    #[prost(int32, optional, tag = "1")]
    pub elem_type: Option<i32>,
    #[prost(message, optional, tag = "2")]
    pub shape: Option<Shape>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Shape {
    #[prost(message, repeated, tag = "1")]
    pub dims: Vec<Dimension>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Dimension {
    #[prost(int64, optional, tag = "1")]
    pub value: Option<i64>,
}
