use super::error::TableResult;
use bigdecimal::BigDecimal;

pub trait Evaluate: Clone + std::fmt::Debug {
    fn evaluate<P>(&self, get_cell_value: &mut P) -> Vec<TableResult<BigDecimal>>
    where
        P: FnMut(usize, usize) -> TableResult<BigDecimal>;
}
