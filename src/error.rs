use std::result::Result;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TableError {
    #[error("Mismatched column")]
    MismatchedColumns,
    #[error("Empty table")]
    EmptyTable,
    #[error("Error reading file")]
    ErrorReadingFile,
    #[error("Invalid cell: {0}")]
    InvalidCell(String),
    #[error("Error parsing AST: {0}")]
    ErrorConstructingAst(String),
    #[error("Runtime Error: {0}")]
    RuntimeError(String),
    #[error("Recursive Cell at: {0:?}")]
    RecursiveCellExpr((usize, usize)),
    #[error("Error attempting to evaluate empty cell")]
    EmptyCellEvaluation,
    #[error("Multiple cell values returned where a single was expected")]
    MultipleCellReturn,
}

impl TableError {
    pub fn runtime_error<T>(err: T) -> Self
    where
        T: ToString,
    {
        Self::RuntimeError(err.to_string())
    }
}

pub type TableResult<T> = Result<T, TableError>;
