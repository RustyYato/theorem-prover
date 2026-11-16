mod lexer;

use std::assert_matches;

use lexer::{Kind::*, Lexer, Token};

use crate::lexer::Kind;
pub use lexer::Newlines;

pub struct Parser<'text> {
    lexer: lexer::Lexer<'text>,
}

macro_rules! is_ident {
    (check $val:expr) => {
        is_ident!(@internal inject check $val)
    };
    (assert $val:expr) => {
        is_ident!(@internal inject assert $val)
    };
    (@internal inject $i:tt $val:expr) => {
        is_ident!(@internal $i ($val) (Ident | Max | When | Trunc))
    };
    (@internal assert ($val:expr) ($pat:pat)) => {
        assert_matches!($val, $pat)
    };
    (@internal check ($val:expr) ($pat:pat)) => {
        matches!($val, $pat)
    };
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

    fn parse(&mut self, kind: Kind) -> Token<'text> {
        let token = self.lexer.lex();

        assert_eq!(
            token.kind(),
            kind,
            "expected {kind:?} but found {:?}",
            token.kind()
        );

        token
    }

    fn try_parse(&mut self, kind: Kind) -> bool {
        if self.lexer.peek() == kind {
            self.lexer.lex();
            true
        } else {
            false
        }
    }

    fn unreachable(&self) -> ! {
        unreachable!("{:?}", self.lexer.peek_token().span())
    }

    fn parse_ident(&mut self) {
        let token = self.lexer.lex();

        is_ident!(assert token.kind());
    }

    pub fn parse_file(mut self) -> lexer::Newlines {
        loop {
            match self.lexer.peek() {
                Eof => break,
                Def => self.parse_def(),
                _ => self.unreachable(),
            }
        }
        self.lexer.finish()
    }

    fn parse_def(&mut self) {
        self.parse(Def);
        self.parse_ident();
        self.parse(Colon);
        self.parse_term();
        self.parse(ColonEq);
        self.parse_term();
    }

    fn parse_term_list(&mut self) {
        loop {
            self.parse_ident();
            self.parse(Colon);
            self.parse_term();

            if !self.try_parse(Comma) || !is_ident!(check self.lexer.peek()) {
                break;
            }
        }
    }

    fn parse_term(&mut self) {
        match self.lexer.peek() {
            Forall => {
                self.parse(Forall);
                self.parse_term_list();
                self.parse(BoldArrow);
                self.parse_term();
            }
            Fn => {
                self.parse(Fn);
                self.parse_term_list();
                self.parse(BoldArrow);
                self.parse_term();
            }
            Univ => {
                self.parse(Univ);
                self.parse_universe();
            }
            Type => {
                self.parse(Type);
                self.parse_univ_level();
            }
            Prop => {
                self.parse(Prop);
            }
            SProp => {
                self.parse(SProp);
            }
            Natural => {
                self.parse(Natural);
            }
            kind if is_ident!(check kind) => {
                self.parse_ident();
            }
            _ => self.unreachable(),
        }
    }

    fn parse_universe(&mut self) {
        if self.try_parse(LParen) {
            self.parse_universe();
            self.parse(RParen);
            return;
        }

        self.parse_univ_kind();
        self.parse(Semicolon);
        self.parse_univ_level();
    }

    fn parse_univ_kind(&mut self) {
        match self.lexer.peek() {
            LParen => {
                self.parse(LParen);
                self.parse_univ_kind();
                self.parse(RParen);
            }
            Ident => {
                self.parse_ident();
            }
            Max => {
                self.parse(Max);
                self.parse_univ_kind();
                self.parse_univ_kind();
            }
            Trunc => {
                self.parse(Trunc);
                self.parse_univ_kind();
            }
            Type => {
                self.parse(Type);
            }
            Prop => {
                self.parse(Prop);
            }
            SProp => {
                self.parse(SProp);
            }
            _ => self.unreachable(),
        }
    }

    fn parse_univ_level(&mut self) {
        match self.lexer.peek() {
            LParen => {
                self.parse(LParen);
                self.parse_univ_level();
                self.parse(RParen);
            }
            Natural => {
                self.parse(Natural);
            }
            Max => {
                self.parse(Max);
                self.parse_univ_level();
                self.parse_univ_level();
            }
            When => {
                self.parse(When);
                self.parse_univ_kind();
                self.parse_univ_level();
            }
            ident if is_ident!(check ident) => {
                self.parse_ident();
                if self.try_parse(Add) {
                    self.parse(Natural);
                }
            }
            _ => self.unreachable(),
        }
    }
}
