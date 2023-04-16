use std::collections::HashSet;

use bigdecimal::BigDecimal;

use crate::{
    ast::Expr,
    cell::{Cell, CellKind},
    error::{TableError, TableResult},
    eval::Evaluate,
};

#[derive(Debug, Clone)]
pub struct Table<'source, T>
where
    T: Evaluate,
{
    rows: usize,
    cols: usize,
    cells: Vec<TableResult<Cell<'source, T>>>,
}

impl<'source> Table<'source, Expr> {
    pub fn new_interpet(source: &'source str) -> TableResult<Self> {
        let mut cells = Vec::new();
        let mut rows = 0;

        let mut previous_cols = None;
        for row in source.lines() {
            let mut current_cols = 0;
            for col in row.split('|') {
                let cell = Cell::new_expr(col);
                cells.push(cell);
                current_cols += 1;
            }
            match (previous_cols, current_cols) {
                (None, c) => previous_cols = Some(c),
                (Some(p), c) if p != c => Err(TableError::MismatchedColumns)?,
                _ => {}
            }

            rows += 1;
        }
        match previous_cols {
            None => Err(TableError::EmptyTable),
            Some(cols) => Ok(Self { rows, cols, cells }),
        }
    }
}

impl<'source, T: Evaluate> Table<'source, T> {
    pub fn evaluate_cell(
        &mut self,
        col: usize,
        row: usize,
        mut call_chain: HashSet<(usize, usize)>,
    ) -> TableResult<BigDecimal> {
        if !call_chain.insert((col, row)) {
            return Err(TableError::RecursiveCellExpr((col, row)));
        }

        let cell = self.cells[(col * self.rows + row)].clone()?;
        match cell.kind.clone() {
            CellKind::Empty => Err(TableError::EmptyCellEvaluation),
            CellKind::Number(d) => Ok(d),
            CellKind::Expr { result, expr } => {
                if let None = result {
                    let res = expr.evaluate(&mut |new_col, new_row| {
                        Table::evaluate_cell(self, new_col, new_row, call_chain.clone())
                    });
                    let res = match res.len() {
                        1 => res[0].clone(),
                        _ => Err(TableError::MultipleCellReturn),
                    };
                    self.cells[(col * self.rows + row)] = Ok(Cell {
                        kind: CellKind::Expr {
                            expr,
                            result: Some(res.clone()),
                        },
                        source: cell.source,
                    });
                    return res;
                };
                result.unwrap()
            }
        }
    }

    pub fn run(&mut self) {
        for col in 0..self.cols {
            for row in 0..self.rows {
                if let Ok(c) = self.cells[(col * self.rows + row)].clone() {
                    match c.kind {
                        CellKind::Expr { expr, result } if result.is_none() => {
                            let res = expr.evaluate(&mut |new_col, new_row| {
                                Table::evaluate_cell(self, new_col, new_row, HashSet::new())
                            });
                            let res = match res.len() {
                                1 => res[0].clone(),
                                _ => Err(TableError::MultipleCellReturn),
                            };
                            self.cells[(col * self.rows + row)] = Ok(Cell {
                                kind: CellKind::Expr {
                                    expr,
                                    result: Some(res.clone()),
                                },
                                source: c.source,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl<'source, T: Evaluate> std::fmt::Display for Table<'source, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for col in 0..self.cols {
            for row in 0..self.rows {
                match self.cells[col * self.rows + row].clone() {
                    Ok(c) => write!(f, "{c}")?,
                    Err(e) => write!(f, "{e}")?,
                };
                write!(f, "|")?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}
