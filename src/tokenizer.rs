use super::error::*;
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Number(BigDecimal),
    CellRef((usize, usize)),
    Plus,
    Slash,
    Minus,
    Star,
    LeftParen,
    RightParen,
}

impl Token {
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    pub fn get_number(&self) -> Option<BigDecimal> {
        match self {
            Self::Number(d) => Some(d.clone()),
            _ => None,
        }
    }

    pub fn is_cell_ref(&self) -> bool {
        matches!(self, Self::CellRef(..))
    }

    pub fn is_binary(&self) -> bool {
        use Token::*;
        matches!(self, Plus | Slash | Star | Minus)
    }
}

impl TryFrom<char> for Token {
    type Error = TableError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        use Token::*;
        match value {
            '+' => Ok(Plus),
            '-' => Ok(Minus),
            '/' => Ok(Slash),
            '*' => Ok(Star),
            '(' => Ok(LeftParen),
            ')' => Ok(RightParen),
            _ => Err(TableError::InvalidCell(format!(
                "Unknown character encountered: {value}"
            ))),
        }
    }
}

pub struct Tokenizer<'a> {
    source: &'a [char],
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a [char]) -> Self {
        Self { source }
    }

    fn at_end(&mut self) -> bool {
        self.source.is_empty()
    }

    fn chop_while_idx<P>(&mut self, mut predicate: P) -> usize
    where
        P: FnMut(char) -> bool,
    {
        let mut n = 0;
        while n < self.source.len() && predicate(self.source[n]) {
            n += 1;
        }
        n
    }

    fn chop_while<P>(&mut self, predicate: P) -> &'a [char]
    where
        P: FnMut(char) -> bool,
    {
        let n = self.chop_while_idx(predicate);
        self.chop(n)
    }

    fn chop_while_or_else<P>(&mut self, predicate: P, err: TableError) -> TableResult<&'a [char]>
    where
        P: FnMut(char) -> bool,
    {
        let n = self.chop_while_idx(predicate);
        if n == 0 {
            return Err(err);
        };
        Ok(self.chop(n))
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let result = &self.source[..n];
        self.source = &self.source[n..];
        return result;
    }

    fn strip_left(&mut self) {
        self.chop_while(|c| c.is_whitespace());
    }

    fn number(&mut self) -> TableResult<Token> {
        let source = self.chop_while(|c| c.is_numeric());
        let mut string_num = source.iter().collect::<String>();

        if !self.at_end() && self.source[0] == '.' {
            self.chop(1);
            let chars = self.chop_while(|c| c.is_numeric());
            string_num.push('.');
            string_num.extend(chars);
        }

        let decimal = BigDecimal::from_str(&string_num).map_err(|_| {
            TableError::InvalidCell(format!("Could not format {string_num} as a valid number"))
        })?;
        Ok(Token::Number(decimal))
    }

    fn cell_reference(&mut self) -> TableResult<Token> {
        let column_slice = self.chop_while_or_else(
            |c| c.is_ascii_alphabetic(),
            TableError::InvalidCell(format!("Could not parse cell reference")),
        )?;

        let mut col = 0;
        let base: usize = 26;
        for (i, c) in column_slice.iter().rev().enumerate() {
            if c.is_ascii_alphabetic() {
                let col_ref = c.to_ascii_lowercase() as usize - 96;
                col += col_ref * base.pow(i as u32);
            }
        }

        let row_slice = self.chop_while_or_else(
            |c| c.is_numeric(),
            TableError::InvalidCell(format!("Could not parse cell reference")),
        )?;

        let row = row_slice
            .into_iter()
            .collect::<String>()
            .parse::<usize>()
            .map_err(|_| TableError::InvalidCell(format!("Could not parse cell reference")))?;

        if row == 0 {
            return Err(TableError::InvalidCell(format!(
                "Could not parse cell reference"
            )));
        }

        Ok(Token::CellRef((col - 1, row - 1)))
    }

    fn next_token(&mut self) -> Option<TableResult<Token>> {
        self.strip_left();
        if self.at_end() {
            return None;
        }

        let token = match self.source[0] {
            c if c.is_ascii_alphabetic() => self.cell_reference(),
            c if c.is_numeric() => self.number(),
            _ => {
                let token = Token::try_from(self.source[0]);
                self.source = &self.source[1..];
                token
            }
        };

        Some(token)
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = TableResult<Token>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::FromPrimitive;

    use super::*;

    #[test]
    fn test_parse_integer() {
        let tokenizer = Tokenizer::new(&[' ', '1', '.', '2']);
        let tokens = tokenizer.collect::<Vec<TableResult<Token>>>();
        assert_eq!(tokens.len(), 1);
        let token = tokens[0].clone().unwrap();
        assert_eq!(token, Token::Number(BigDecimal::from_f64(1.2).unwrap()))
    }

    #[test]
    fn test_parse_cell_reference() {
        let tokenizer = Tokenizer::new(&[' ', 'a', 'a', '1', '2']);
        let tokens = tokenizer.collect::<Vec<TableResult<Token>>>();
        assert_eq!(tokens.len(), 1);
        let token = tokens[0].clone().unwrap();
        assert_eq!(token, Token::CellRef((26, 11)));
    }

    #[test]
    fn test_parse_cell_op() {
        use Token::*;
        for (op, expected_op_token) in vec![('+', Plus), ('-', Minus), ('/', Slash), ('*', Star)] {
            let input = &[' ', ' ', 'a', '1', ' ', op, ' ', 'b', '3'];
            let tokenizer = Tokenizer::new(input);
            let tokens = tokenizer.collect::<Vec<TableResult<Token>>>();
            assert_eq!(tokens.len(), 3);
            let expected_tokens = vec![CellRef((0, 0)), expected_op_token, CellRef((1, 2))];
            for (token, expected_token) in tokens.iter().zip(expected_tokens) {
                assert!(token.is_ok());
                assert_eq!(token.clone().unwrap(), expected_token);
            }
        }
    }
}
