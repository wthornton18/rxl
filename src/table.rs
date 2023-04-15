use std::{cell::RefCell, collections::HashSet};

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
    fn new_interpet(source: &'source str) -> TableResult<Self> {
        let mut cells = Vec::new();
        let mut rows = 0;

        let mut previous_cols = None;
        for row in source.lines() {
            let mut current_cols = 0;
            for col in row.split(',') {
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
        call_chain: HashSet<(usize, usize)>,
    ) -> TableResult<BigDecimal> {
        if call_chain.contains(&(col, row)) {
            return Err(TableError::RecursiveCellExpr((col, row)));
        };

        let cell = self.cells[(col * self.rows + row)].clone()?;
        match cell.kind.clone() {
            CellKind::Empty => Err(TableError::EmptyCellEvaluation),
            CellKind::Number(d) => Ok(d),
            CellKind::Expr { result, expr } => {
                if let None = result {
                    let res = expr.evaluate(|new_col, new_row| {
                        Table::evaluate_cell(self, new_col, new_row, call_chain.clone())
                    });
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

    fn run(&mut self) {
        for col in 0..self.cols {
            for row in 0..self.rows {
                let mut call_chain = HashSet::new();
                self.evaluate_cell(col, row, call_chain);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table() {
        let mut table = Table::new_interpet("1,2\n=a1+a2,=b1+3").unwrap();
        table.run();
        println!("{:?}", table);
        assert_eq!(1, 2)
    }
}
