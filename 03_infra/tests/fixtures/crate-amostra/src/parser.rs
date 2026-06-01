pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

pub enum TokenKind {
    Word,
    Number,
    Punct,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    input
        .split_whitespace()
        .map(|w| Token {
            kind: classify(w),
            text: w.to_string(),
        })
        .collect()
}

fn classify(word: &str) -> TokenKind {
    if word.chars().all(|c| c.is_ascii_digit()) {
        TokenKind::Number
    } else if word.chars().all(|c| c.is_ascii_alphabetic()) {
        TokenKind::Word
    } else {
        TokenKind::Punct
    }
}
