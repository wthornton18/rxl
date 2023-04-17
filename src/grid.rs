use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub struct Grid<T>
where
    T: Debug + Clone,
{
    internal: Vec<T>,
    pub rows: usize,
    pub cols: usize,
}

impl<T: Debug + Clone> Grid<T> {
    pub fn new(rows: usize, cols: usize, internal: Vec<T>) -> Self {
        Self {
            rows,
            cols,
            internal: internal,
        }
    }
}

impl<T: Debug + Clone> Index<(usize, usize)> for Grid<T> {
    type Output = T;
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.internal[(self.cols * row) + col]
    }
}

impl<T: Debug + Clone> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        &mut self.internal[(self.cols * row) + col]
    }
}
