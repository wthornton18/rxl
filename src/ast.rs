use bigdecimal::BigDecimal;

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
}

impl Expr {
    fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Self::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }

    fn grouping(expr: Expr) -> Self {
        Self::Grouping(Box::new(expr))
    }

    fn literal(token: Token) -> Self {
        Self::Literal(token)
    }

    fn unary(operator: Token, right: Expr) -> Self {
        Self::Unary {
            operator,
            right: Box::new(right),
        }
    }

    fn eval<P>(&self, ast: &Expr, eval_other: &mut P) -> TableResult<BigDecimal>
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>,
    {
        use Expr::*;
        use Token::*;
        match ast {
            Binary {
                left,
                operator,
                right,
            } => {
                let left = self.eval(left, eval_other)?;
                let right = self.eval(right, eval_other)?;
                Ok(match operator {
                    Plus => left + right,
                    Slash => left / right,
                    Minus => left - right,
                    Star => left * right,
                    _ => Err(TableError::RuntimeError(format!(
                        "invalid token in binary expression: {operator:?}"
                    )))?,
                })
            }
            Grouping(expr) => expr.eval(expr, eval_other),
            Literal(token) => match token {
                Number(d) => Ok(d.clone()),
                CellRef((col, row)) => eval_other(*col, *row),
                _ => Err(TableError::RuntimeError(format!(
                    "invalid token literal {token:?}"
                ))),
            },
            Unary { operator, right } => {
                let right = self.eval(right, eval_other)?;

                match operator {
                    Minus => Ok(-right),
                    _ => Err(TableError::RuntimeError(format!(
                        "invalid token for unary expression {operator:?}"
                    ))),
                }
            }
        }
    }
}

impl Evaluate for Expr {
    fn evaluate<P>(&self, mut evaluate_other: P) -> TableResult<BigDecimal>
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>,
    {
        self.eval(self, &mut evaluate_other)
    }
}

pub struct Parser<'source, I: Iterator<Item = TableResult<Token>>> {
    iterator: &'source mut I,
    current_token: Option<Token>,
    previous_token: Option<Token>,
}

impl<'source, I: Iterator<Item = TableResult<Token>>> Parser<'source, I> {
    pub fn new(iterator: &'source mut I) -> Self {
        Self {
            iterator,
            current_token: None,
            previous_token: None,
        }
    }

    fn get_previous_token(&mut self) -> TableResult<Token> {
        self.previous_token
            .clone()
            .ok_or(TableError::ErrorConstructingAst(format!(
                "Error returning previous token"
            )))
    }

    pub fn ast(&mut self) -> TableResult<Expr> {
        self.advance()?;
        self.expression()
    }

    fn expression(&mut self) -> TableResult<Expr> {
        self.term()
    }

    fn term(&mut self) -> TableResult<Expr> {
        use Token::{Minus, Plus};
        let mut expr = self.factor()?;
        loop {
            if !self.advance_match(|t| t == Minus || t == Plus)? {
                return Ok(expr);
            }

            let operator = self.get_previous_token()?;

            let right = self.factor()?;
            expr = Expr::binary(expr, operator, right);
        }
    }

    fn factor(&mut self) -> TableResult<Expr> {
        use Token::{Slash, Star};
        let mut expr = self.unary()?;

        loop {
            if !self.advance_match(|t| t == Slash || t == Star)? {
                return Ok(expr);
            }
            let operator = self.get_previous_token()?;
            let right = self.unary()?;
            expr = Expr::binary(expr, operator, right);
        }
    }

    fn unary(&mut self) -> TableResult<Expr> {
        use Token::Minus;
        if self.advance_match(|t| t == Minus)? {
            let operator = self.get_previous_token()?;
            let right = self.unary()?;
            Ok(Expr::unary(operator, right))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> TableResult<Expr> {
        use Token::{LeftParen, RightParen};
        if self.advance_match(|t| t.is_number() || t.is_cell_ref())? {
            let token = self.get_previous_token()?;

            Ok(Expr::literal(token))
        } else if self.advance_match(|t| t == LeftParen)? {
            let expr = self.expression()?;
            self.consume_or(
                |t| t == RightParen,
                TableError::ErrorConstructingAst(format!("Expected ')' after expression")),
            )?;

            Ok(Expr::grouping(expr))
        } else {
            Err(TableError::ErrorConstructingAst(format!(
                "Invalid primary expression token: {:?}",
                self.current_token.clone()
            )))
        }
    }

    fn advance_match<P>(&mut self, predicate: P) -> TableResult<bool>
    where
        P: FnOnce(Token) -> bool,
    {
        match self.current_token.clone() {
            Some(t) if predicate(t.clone()) => {
                self.advance()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_or<P>(&mut self, predicate: P, err: TableError) -> TableResult<()>
    where
        P: FnOnce(Token) -> bool,
    {
        match self.current_token.clone() {
            Some(t) if predicate(t.clone()) => {
                self.advance()?;
                Ok(())
            }
            _ => Err(err),
        }
    }

    fn advance(&mut self) -> TableResult<()> {
        self.previous_token = self.current_token.clone();
        self.current_token = match self.iterator.next() {
            None => None,
            Some(t) => Some(t?),
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bigdecimal::{BigDecimal, FromPrimitive};

    struct DummyTokenizer {
        tokens: Vec<Token>,
        pointer: usize,
    }

    impl DummyTokenizer {
        fn new(tokens: Vec<Token>) -> Self {
            Self { tokens, pointer: 0 }
        }
    }

    impl Iterator for DummyTokenizer {
        type Item = TableResult<Token>;
        fn next(&mut self) -> Option<Self::Item> {
            if self.pointer < self.tokens.len() {
                let token = self.tokens[self.pointer].clone();
                self.pointer += 1;
                Some(Ok(token))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_simple_op() {
        use Token::{CellRef, Minus, Number, Plus, Slash, Star};

        for op in [Minus, Plus, Slash, Star] {
            for left_token in [CellRef((0, 0)), Number(BigDecimal::from_f64(1.2).unwrap())] {
                for right_token in [CellRef((1, 1)), Number(BigDecimal::from_f64(1.4).unwrap())] {
                    let tokens = vec![left_token.clone(), op.clone(), right_token.clone()];
                    let mut tokenizer = DummyTokenizer::new(tokens);
                    let mut parser = Parser::new(&mut tokenizer);
                    let ast = parser.ast();
                    assert!(ast.is_ok());
                    let expectec_ast = Expr::binary(
                        Expr::literal(left_token.clone()),
                        op.clone(),
                        Expr::literal(right_token),
                    );

                    let ast = ast.unwrap();

                    assert_eq!(ast, expectec_ast);
                }
            }
        }
    }
}
