use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Object not found")]
    ObjectNotFound,
    #[error("Frame not found")]
    FrameNotFound,
    #[error("Column not found")]
    ColumnNotFound,
    #[error("Row not found")]
    RowNotFound,
    #[error("View not found")]
    ViewNotFound,
    #[error("A frame must keep at least one column")]
    CannotDeleteLastColumn,
    #[error("Derived frame results are read-only; edit their transformation instead")]
    DerivedFrameReadOnly,
    /// Named, not counted. "Another formula references it" leaves somebody
    /// to search their own document for a formula they cannot see, and the
    /// site that refuses always knows which one it found — it had to find
    /// one to refuse at all.
    #[error("{0}")]
    ReferencedByFormula(String),
    #[error("Formula error: {0}")]
    Formula(String),
    #[error("That formula creates a circular dependency")]
    CircularDependency,
    #[error("Could not load document: {0}")]
    Load(String),
    #[error("Could not save document: {0}")]
    Persistence(String),
    #[error("Could not import file: {0}")]
    Import(String),
    /// A wrangle step the plan refused. Carried without a prefix of its
    /// own: the editor shows it against the step that failed, which has
    /// already said which step that is and what it was meant to do.
    #[error("{0}")]
    Transform(String),
    #[error("Could not export frame: {0}")]
    Export(String),
    #[error("Invalid replicated operation: {0}")]
    InvalidOperation(String),
    #[error("Invalid operation event: {0}")]
    InvalidEvent(String),
}
