pub use anyhow::{format_err, Error, Result};
pub use itertools::Itertools;

pub enum Either<L, R> {
    Left(L),
    Right(R),
}
pub use Either::{Left, Right};

impl<L, R> Either<L, R> {
    pub fn is_left(&self) -> bool {
        match self {
            Left(_) => true,
            _ => false,
        }
    }

    pub fn is_right(&self) -> bool {
        match self {
            Right(_) => true,
            _ => false,
        }
    }
}
