use crate::parser::{tokenize, Token};

pub struct Report {
    pub total: usize,
    pub tokens: Vec<Token>,
}

pub fn run(input: &str) -> Report {
    let tokens = tokenize(input);
    Report {
        total: tokens.len(),
        tokens,
    }
}
