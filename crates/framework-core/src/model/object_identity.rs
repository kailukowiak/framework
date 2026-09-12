use crate::DataObject;

impl DataObject {
    pub fn id(&self) -> &str {
        match self {
            Self::Value(value) => &value.id,
            Self::Result(result) => &result.id,
            Self::Block(block) => &block.id,
            Self::Series(series) => &series.id,
            Self::Container(container) => &container.id,
            Self::Frame(frame) => &frame.id,
            Self::Text(text) => &text.id,
            Self::Plot(plot) => &plot.id,
            Self::CalculationMatrix(matrix) => &matrix.id,
            Self::Model(model) => &model.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Value(value) => &value.name,
            Self::Result(result) => &result.name,
            Self::Block(block) => &block.name,
            Self::Series(series) => &series.name,
            Self::Container(container) => &container.name,
            Self::Frame(frame) => &frame.name,
            Self::Text(text) => &text.name,
            Self::Plot(plot) => &plot.name,
            Self::CalculationMatrix(matrix) => &matrix.name,
            Self::Model(model) => &model.name,
        }
    }
}
