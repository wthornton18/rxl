use bigdecimal::{BigDecimal, FromPrimitive};

use crate::{
    error::{TableError, TableResult},
    eval::Evaluate,
    tokenizer::Token,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Grouping(Box<Expr>),
    Literal(Token),
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Call {
        calle: Box<Expr>,
        arguments: Vec<Box<Expr>>,
    },
}

impl Expr {
    pub fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Self::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }

    pub fn grouping(expr: Expr) -> Self {
        Self::Grouping(Box::new(expr))
    }

    pub fn literal(token: Token) -> Self {
        Self::Literal(token)
    }

    pub fn unary(operator: Token, right: Expr) -> Self {
        Self::Unary {
            operator,
            right: Box::new(right),
        }
    }

    pub fn call(calle: Expr, arguments: Vec<Expr>) -> Self {
        Self::Call {
            calle: Box::new(calle),
            arguments: arguments
                .into_iter()
                .map(|e| Box::new(e))
                .collect::<Vec<_>>(),
        }
    }
}

impl Evaluate for Expr {
    fn evaluate<P>(&self, get_cell_value: &mut P) -> Vec<TableResult<BigDecimal>>
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>,
    {
        use Expr::*;
        use Token::*;
        match self {
            Binary {
                left,
                operator,
                right,
            } => {
                let left = left.evaluate(get_cell_value);
                let right = right.evaluate(get_cell_value);
                if left.len() != 1 || right.len() != 1 {
                    return vec![Err(TableError::runtime_error(
                        "Cannot add cell ranges together",
                    ))];
                }
                let left = left[0].clone();
                let right = right[0].clone();
                if let (Ok(left), Ok(right)) = (left, right) {
                    let res = match operator {
                        Plus => Ok(left + right),
                        Slash => Ok(left / right),
                        Minus => Ok(left - right),
                        Star => Ok(left * right),
                        _ => Err(TableError::RuntimeError(format!(
                            "invalid token in binary expression: {operator:?}"
                        ))),
                    };
                    vec![res]
                } else {
                    vec![Err(TableError::runtime_error(
                        "Error performing binary operation on two cells",
                    ))]
                }
            }
            Grouping(expr) => expr.evaluate(get_cell_value),
            Literal(token) => match token {
                Number(d) => vec![Ok(d.clone())],
                CellRef((col, row)) => vec![get_cell_value(*col, *row)],
                CellRange((col_range, row_range)) => {
                    let mut cells = Vec::new();
                    for col in col_range.clone().into_iter() {
                        for row in row_range.clone().into_iter() {
                            cells.push(get_cell_value(col, row))
                        }
                    }
                    cells
                }
                _ => vec![Err(TableError::RuntimeError(format!(
                    "invalid token literal {token:?}"
                )))],
            },
            Unary { operator, right } => {
                let right = right.evaluate(get_cell_value);
                if right.len() != 1 {
                    return vec![Err(TableError::runtime_error(
                        "Error in unary expression - expected single cell value",
                    ))];
                }

                let right = right[0].clone();

                match (operator, right) {
                    (Minus, Ok(r)) => vec![Ok(-r)],
                    (_, Err(r)) => vec![Err(r)],
                    _ => vec![Err(TableError::RuntimeError(format!(
                        "invalid token for unary expression {operator:?}"
                    )))],
                }
            }
            Call { calle, arguments } => match *calle.clone() {
                Expr::Literal(t) => match t {
                    Sum => {
                        let counter = BigDecimal::from_i128(0).ok_or(TableError::RuntimeError(
                            format!("Error performing summation"),
                        ));
                        if let Err(c) = counter {
                            return vec![Err(c)];
                        }
                        let mut counter = counter.unwrap();
                        for arg in arguments {
                            let res = arg.evaluate(get_cell_value);
                            for r in res {
                                if let Ok(res) = r.clone() {
                                    counter += res;
                                } else {
                                    return vec![Err(TableError::runtime_error(
                                        "Error performing summation",
                                    ))];
                                }
                            }
                        }

                        vec![Ok(counter)]
                    }
                    _ => vec![Err(TableError::RuntimeError(format!(
                        "Invalid token encountered type for calle {t:?}"
                    )))],
                },
                _ => vec![Err(TableError::RuntimeError(format!(
                    "Invalid expr type for calle {calle:?}"
                )))],
            },
        }
    }
}
