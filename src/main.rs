#[allow(unused_variables)]
#[allow(dead_code)]
#[allow(unused)]
use std::fs::File;
use std::io::{BufReader, Read};

mod cell;
mod error;
mod precedence;
use error::{TableError, TableResult};
mod ast;
mod eval;
mod table;
mod tokenizer;
use ast::Parser;
use tokenizer::Tokenizer;

fn main() -> TableResult<()> {
    let f = File::open("./input.rxl").map_err(|_| TableError::ErrorReadingFile)?;
    let mut reader = BufReader::new(f);
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|_| TableError::ErrorReadingFile)?;

    let chars = buf.chars().collect::<Vec<_>>();

    //println!("{:?}", table);
    Ok(())
}
