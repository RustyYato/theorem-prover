mod lexer;

use std::{assert_matches, num::NonZero};

use lexer::{Kind::*, Lexer, Token};
use tp_ast as a;
use ustr::Ustr;

use crate::lexer::Kind;
pub use lexer::Newlines;

pub struct Parser<'text> {
    lexer: lexer::Lexer<'text>,
    ident_id: u32,
    invalid_ident: Ustr,
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
            ident_id: 0,
            invalid_ident: Ustr::from("<invalid-ident>"),
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

    fn parse_ident(&mut self) -> a::Ident {
        let token = self.lexer.lex();

        is_ident!(assert token.kind());
        self.ident_id = self.ident_id.strict_add(1);

        a::Ident {
            id: NonZero::new(self.ident_id).unwrap(),
            val: if is_ident!(check token.kind()) {
                Ustr::from(token.lexeme())
            } else {
                self.invalid_ident
            },
        }
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

    fn parse_args(&mut self) {
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
                self.parse_args();
                self.parse(BoldArrow);
                self.parse_term();
            }
            Fn => {
                self.parse(Fn);
                self.parse_args();
                self.parse(BoldArrow);
                self.parse_term();
            }
            Univ => {
                self.parse(Univ);
                let u = self.parse_universe();
                dbg!(u);
            }
            Type => {
                self.parse(Type);
                let l = self.parse_univ_level(UnivPrec::All);
                dbg!(l);
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

    fn parse_universe(&mut self) -> a::Universe {
        if self.try_parse(LParen) {
            let u = self.parse_universe();
            self.parse(RParen);
            return u;
        }

        let sort = self.parse_univ_sort(UnivPrec::Bound);
        let level = self.parse_univ_level(UnivPrec::Bound);

        a::Universe { sort, level }
    }

    fn parse_univ_sort(&mut self, univ_prec: UnivPrec) -> a::Sort {
        match self.lexer.peek() {
            LParen => {
                self.parse(LParen);
                let s = self.parse_univ_sort(UnivPrec::All);
                self.parse(RParen);
                s
            }
            Ident => {
                let name = self.parse_ident();
                a::Sort::param(name)
            }
            Max => {
                if matches!(univ_prec, UnivPrec::Bound) {
                    panic!()
                }

                self.parse(Max);
                let mut sorts = Vec::new();
                while is_ident!(check self.lexer.peek()) {
                    sorts.push(self.parse_univ_sort(UnivPrec::Bound));
                }
                a::Sort::max(sorts)
            }
            Trunc => {
                if matches!(univ_prec, UnivPrec::Bound) {
                    panic!()
                }

                self.parse(Trunc);
                self.parse_univ_sort(UnivPrec::Bound).trunc()
            }
            Type => {
                self.parse(Type);
                a::Sort::type_()
            }
            Prop => {
                self.parse(Prop);
                a::Sort::prop()
            }
            SProp => {
                self.parse(SProp);
                a::Sort::sprop()
            }
            _ => self.unreachable(),
        }
    }

    fn parse_univ_level(&mut self, univ_prec: UnivPrec) -> a::Level {
        let l = match self.lexer.peek() {
            LParen => {
                self.parse(LParen);
                let l = self.parse_univ_level(UnivPrec::All);
                self.parse(RParen);
                l
            }
            Natural => {
                self.parse(Natural);
                a::Level::nat(0)
            }
            Max => {
                if matches!(univ_prec, UnivPrec::Bound) {
                    panic!()
                }

                self.parse(Max);
                let mut levels = Vec::new();
                while is_ident!(check self.lexer.peek()) {
                    levels.push(self.parse_univ_level(UnivPrec::Bound));
                }
                a::Level::max(levels)
            }
            When => {
                if matches!(univ_prec, UnivPrec::Bound) {
                    panic!()
                }

                self.parse(When);
                let s = self.parse_univ_sort(UnivPrec::Bound);
                let l = self.parse_univ_level(UnivPrec::Bound);
                l.when(s)
            }
            ident if is_ident!(check ident) => {
                let name = self.parse_ident();
                let offset = if matches!(univ_prec, UnivPrec::All) && self.try_parse(Add) {
                    let n = self.parse(Natural);
                    0
                } else {
                    0
                };
                return a::Level::param(name, offset);
            }
            _ => self.unreachable(),
        };

        if matches!(univ_prec, UnivPrec::All) && self.try_parse(Add) {
            let n = self.parse(Natural);
            l.add(0)
        } else {
            l
        }
    }
}

enum UnivPrec {
    All,
    Bound,
}
