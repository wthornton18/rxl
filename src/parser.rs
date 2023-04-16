use crate::{
    ast::Expr,
    error::{TableError, TableResult},
    tokenizer::Token,
};

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
            self.call()
        }
    }

    fn call(&mut self) -> TableResult<Expr> {
        use Token::LeftParen;
        let mut expr = self.primary()?;
        while self.advance_match(|t| t == LeftParen)? {
            expr = self.generate_call(expr)?;
        }
        Ok(expr)
    }

    fn generate_call(&mut self, calle: Expr) -> TableResult<Expr> {
        use Token::RightParen;
        let mut arguments = Vec::new();

        loop {
            arguments.push(self.expression()?);
            if !self.advance_match(|t| t == Token::Comma)? {
                break;
            }
        }
        self.consume_or(
            |t| t == RightParen,
            TableError::ErrorConstructingAst(format!("Expect ')' after arguments")),
        )?;

        Ok(Expr::call(calle, arguments))
    }

    fn primary(&mut self) -> TableResult<Expr> {
        use Token::{LeftParen, RightParen};
        if self.advance_match(|t| {
            t.is_number() || t.is_cell_ref() || t.is_builtin_fn() || t.is_cell_range()
        })? {
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
    use std::ops::Range;

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

    #[test]
    fn test_sum_of_sums() {
        use Token::{CellRange, CellRef, Comma, LeftParen, RightParen, Sum};

        let tokens = vec![
            Sum,
            LeftParen,
            CellRef((0, 0)),
            Comma,
            Sum,
            LeftParen,
            CellRange((Range { start: 0, end: 5 }, Range { start: 1, end: 3 })),
            RightParen,
            RightParen,
        ]; // =sum(a1, sum(a1:e2))

        let mut tokenizer = DummyTokenizer::new(tokens);
        let mut parser = Parser::new(&mut tokenizer);
        let ast = parser.ast();
        assert!(ast.is_ok());
        assert_eq!(
            ast.unwrap(),
            Expr::call(
                Expr::literal(Sum),
                vec![
                    Expr::literal(CellRef((0, 0)),),
                    Expr::call(
                        Expr::literal(Sum),
                        vec![Expr::literal(CellRange((
                            Range { start: 0, end: 5 },
                            Range { start: 1, end: 3 }
                        )),)]
                    )
                ]
            )
        );
    }
}
