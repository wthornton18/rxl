#[derive(Debug, Clone)]
pub enum TableError {
    MismatchedColumns,
    EmptyTable,
    ErrorReadingFile,
    InvalidCell(String),
    ErrorConstructingAst(String),
    RuntimeError(String),
    RecursiveCellExpr((usize, usize)),
    EmptyCell,
    EmptyCellEvaluation,
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
