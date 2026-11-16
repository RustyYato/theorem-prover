mod lexer;

use lexer::{Kind::*, Lexer, Token};

pub struct Parser<'text> {
    lexer: lexer::Lexer<'text>,
}

impl<'text> Parser<'text> {
    pub fn new(text: &'text (impl ?Sized + AsRef<[u8]>)) -> Self {
        Self::new_(text.as_ref())
    }

    fn new_(text: &'text [u8]) -> Self {
        Self {
            lexer: Lexer::new(text),
        }
    }
}
