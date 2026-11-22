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
            "expected {kind:?} but found {:?} {:?}",
            token.kind(),
            token.span(),
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

    pub fn parse_file(mut self) -> (a::Document, lexer::Newlines) {
        let mut globals = Vec::new();
        loop {
            globals.push(match self.lexer.peek() {
                Eof => break,
                Def => a::Global::Definition(self.parse_def()),
                Axiom => a::Global::Axiom(self.parse_axiom()),
                Opaque => a::Global::Opaque(self.parse_def()),
                _ => self.unreachable(),
            })
        }
        (a::Document { globals }, self.lexer.finish())
    }

    fn parse_universe_params<T>(
        &mut self,
        f: impl Copy + core::ops::Fn(&mut Self) -> T,
    ) -> (Vec<T>, Vec<T>) {
        self.parse_universe_params2(f, f)
    }

    fn parse_universe_params2<T, U>(
        &mut self,
        s: impl core::ops::Fn(&mut Self) -> T,
        l: impl core::ops::Fn(&mut Self) -> U,
    ) -> (Vec<T>, Vec<U>) {
        let mut sorts = Vec::new();
        let mut levels = Vec::new();
        self.parse(LCurly);
        let mut is_sorts = true;
        let mut needs_comma = false;
        loop {
            match self.lexer.peek() {
                RCurly => break,
                Slash if is_sorts => {
                    self.parse(Slash);
                    is_sorts = false;
                    needs_comma = false;
                }
                Comma if needs_comma => {
                    self.parse(Comma);
                    needs_comma = false;
                }
                _ => {
                    if needs_comma {
                        break;
                    }
                    if is_sorts {
                        sorts.push(s(self))
                    } else {
                        levels.push(l(self))
                    }
                    needs_comma = true;
                }
            }
        }
        self.parse(RCurly);

        (sorts, levels)
    }

    fn parse_axiom_prefix(&mut self) -> a::Axiom {
        let name = self.parse_ident();
        let (sorts, levels) = if self.try_parse(Period) {
            self.parse_universe_params(|this| this.parse_ident().val)
        } else {
            Default::default()
        };

        self.parse(Colon);
        let ty = self.parse_term();
        a::Axiom {
            name,
            sorts,
            levels,
            ty,
        }
    }

    fn parse_def(&mut self) -> a::Definition {
        self.parse(Def);
        let prefix = self.parse_axiom_prefix();
        self.parse(ColonEq);
        let expr = self.parse_term();
        self.parse(Semicolon);
        a::Definition {
            name: prefix.name,
            sorts: prefix.sorts,
            levels: prefix.levels,
            ty: prefix.ty,
            expr,
        }
    }

    fn parse_axiom(&mut self) -> a::Axiom {
        self.parse(Axiom);
        let axiom = self.parse_axiom_prefix();
        self.parse(Semicolon);
        axiom
    }

    fn parse_args(&mut self) -> Vec<a::Argument> {
        let mut args = Vec::new();
        loop {
            let name = self.parse_ident();
            self.parse(Colon);
            let ty = self.parse_term();

            args.push(a::Argument { name, ty });

            if !self.try_parse(Comma) || !is_ident!(check self.lexer.peek()) {
                break args;
            }
        }
    }

    fn has_term(&self) -> bool {
        matches!(
            self.lexer.peek(),
            LParen | Forall | Fn | Univ | Type | Prop | SProp | Natural
        ) || is_ident!(check self.lexer.peek())
    }

    fn parse_basic_term(&mut self) -> a::Term {
        match self.lexer.peek() {
            LParen => {
                self.parse(LParen);
                let t = self.parse_term();
                self.parse(RParen);
                t
            }
            Forall => {
                self.parse(Forall);
                let args = self.parse_args();
                self.parse(BoldArrow);
                let body = self.parse_term();
                a::Term::Forall(a::Thin::from(a::SliceWithHeader::new::<Box<_>, _>(
                    body, args,
                )))
            }
            Fn => {
                self.parse(Fn);
                let args = self.parse_args();
                self.parse(BoldArrow);
                let body = self.parse_term();
                a::Term::Lambda(a::Thin::from(a::SliceWithHeader::new::<Box<_>, _>(
                    body, args,
                )))
            }
            Univ => {
                self.parse(Univ);
                let u = self.parse_universe();
                a::Term::Univ(u)
            }
            Type => {
                self.parse(Type);
                let level = if is_ident!(check self.lexer.peek())
                    || matches!(self.lexer.peek(), Natural | LParen)
                {
                    self.parse_univ_level(UnivPrec::All)
                } else {
                    a::Level::nat(0)
                };
                a::Term::Univ(a::Universe {
                    sort: a::Sort::type_(),
                    level,
                })
            }
            Prop => {
                self.parse(Prop);
                a::Term::Univ(a::Universe {
                    sort: a::Sort::prop(),
                    level: a::Level::nat(0),
                })
            }
            SProp => {
                self.parse(SProp);
                a::Term::Univ(a::Universe {
                    sort: a::Sort::sprop(),
                    level: a::Level::nat(0),
                })
            }
            Natural => {
                self.parse(Natural);
                unimplemented!()
                // a::Term::Natural(0)
            }
            kind if is_ident!(check kind) => {
                let name = self.parse_ident();
                if self.try_parse(Period) {
                    let (sorts, levels) = self.parse_universe_params2(
                        |this| this.parse_univ_sort(UnivPrec::All),
                        |this| this.parse_univ_level(UnivPrec::All),
                    );
                    a::Term::GlobalRef(Box::new(a::GlobalRef {
                        ident: name,
                        sorts: sorts.into(),
                        levels: levels.into(),
                    }))
                } else {
                    a::Term::Variable(name)
                }
            }
            _ => self.unreachable(),
        }
    }

    fn parse_term(&mut self) -> a::Term {
        let term = self.parse_basic_term();

        let mut args = Vec::new();

        while self.has_term() {
            args.push(self.parse_basic_term());
        }

        if args.is_empty() {
            term
        } else {
            a::Term::Application(a::Thin::from(a::SliceWithHeader::new::<Box<_>, _>(
                term, args,
            )))
        }
    }

    fn parse_universe(&mut self) -> a::Universe {
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
            Natural => a::Level::nat(self.parse_level_offset()),
            Max => {
                if matches!(univ_prec, UnivPrec::Bound) {
                    panic!()
                }

                self.parse(Max);
                let mut levels = Vec::new();
                while is_ident!(check self.lexer.peek())
                    || matches!(self.lexer.peek(), LParen | Natural)
                {
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
                    self.parse_level_offset()
                } else {
                    0
                };
                return a::Level::param(name, offset);
            }
            _ => self.unreachable(),
        };

        if matches!(univ_prec, UnivPrec::All) && self.try_parse(Add) {
            l.add(self.parse_level_offset())
        } else {
            l
        }
    }

    fn parse_level_offset(&mut self) -> u16 {
        let tok = self.parse(Natural);
        tok.lexeme().parse().unwrap()
    }
}

enum UnivPrec {
    All,
    Bound,
}
