use super::error::TableResult;
use super::table::Table;
use bigdecimal::BigDecimal;
use std::cell::{RefCell, RefMut};
use std::collections::HashSet;

pub trait Evaluate: Clone + std::fmt::Debug {
    fn evaluate<P>(&self, evaluate_other: P) -> TableResult<BigDecimal>
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>;
}
