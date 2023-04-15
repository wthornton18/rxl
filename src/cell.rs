use super::ast::Expr;
use super::ast::Parser;
use super::error::*;
use super::eval::Evaluate;
use super::table::Table;
use super::tokenizer::Tokenizer;
use bigdecimal::BigDecimal;
use std::cell::RefCell;
use std::cell::RefMut;
use std::str::FromStr;

#[derive(Debug, Clone, Default)]
pub enum CellKind<T: Evaluate + Clone> {
    #[default]
    Empty,
    Expr {
        expr: T,
        result: Option<TableResult<BigDecimal>>,
    },
    Number(BigDecimal),
}

impl<T: Evaluate + Clone> CellKind<T> {
    fn new_expr(expr: T) -> Self {
        Self::Expr { expr, result: None }
    }

    fn new_number(d: BigDecimal) -> Self {
        Self::Number(d)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cell<'a, T>
where
    T: Evaluate,
{
    pub source: &'a str,
    pub kind: CellKind<T>,
}

impl<'a> Cell<'a, Expr> {
    pub fn new_expr(source: &'a str) -> TableResult<Self> {
        let token_stream = source.chars().collect::<Vec<_>>();
        let kind = if token_stream.len() == 0 {
            CellKind::Empty
        } else {
            match token_stream[0] {
                '=' => parse_expr(&token_stream[1..]),
                c if c.is_numeric() => parse_number(&source),
                _ => unimplemented!("Unimplemented cell kind"),
            }?
        };

        Ok(Self { source, kind })
    }
}

impl<'a, T: Evaluate> Cell<'a, T> {
    pub fn evaluate<P>(self, evaluate_other: P)
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>,
    {
        match self.kind.clone() {
            CellKind::Expr {
                mut expr,
                mut result,
            } if result.is_none() => {
                result = Some(expr.evaluate(evaluate_other));
            }

            _ => {}
        }
    }
}

// impl<'a> Cell<'a, Expr> {
//     pub fn evaluate<P>(mut self, evaluate_other: P) -> Self
//     where
//         P: FnMut((usize, usize)) -> TableResult<BigDecimal> + Clone,
//     {
//     }
// }

impl<'a, T: Evaluate + Clone> std::fmt::Display for Cell<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)
    }
}

fn parse_expr<'a>(token_stream: &'a [char]) -> TableResult<CellKind<Expr>> {
    let mut tokenizer = Tokenizer::new(&token_stream);
    let mut parser = Parser::new(&mut tokenizer);
    parser.ast().map(|ast| CellKind::new_expr(ast))
}

fn parse_number<'a>(num: &'a str) -> TableResult<CellKind<Expr>> {
    BigDecimal::from_str(num)
        .map_err(|_| TableError::InvalidCell(format!("Could not format {num} as a valid number")))
        .map(|d| CellKind::new_number(d))
}
