#![allow(unreachable_code)]

use std::range::Range;

use bstr::BStr;

#[derive(logos::Logos, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(utf8 = false, error = TokenError)]
#[repr(u8)]
pub enum Kind {
    Eof,
    Error,

    #[regex(b".", priority = 0)]
    InvalidUtf8,
    #[regex(".", priority = 1)]
    Utf8,

    #[regex(r"[^\n\P{White_Space}]+")]
    WhiteSpace,
    #[default]
    #[regex("\n", priority = 100)]
    Newline,

    #[regex(r"[_\p{XID_Start}]\p{XID_Continue}*")]
    Ident,
    #[regex(r"[0-9]+")]
    Natural,

    #[token("forall")]
    Forall,
    #[token("fn")]
    Fn,
    #[token("def")]
    Def,
    #[token("axiom")]
    Axiom,
    #[token("opaque")]
    Opaque,
    #[token("Univ")]
    Univ,
    #[token("Type")]
    Type,
    #[token("Prop")]
    Prop,
    #[token("SProp")]
    SProp,
    #[token("max")]
    Max,
    #[token("when")]
    When,
    #[token("trunc")]
    Trunc,
    #[token(":=")]
    ColonEq,
    #[token("=")]
    Eq,
    #[token("=>")]
    BoldArrow,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Period,
    #[token("+")]
    Add,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LCurly,
    #[token("}")]
    RCurly,
    #[token("/")]
    Slash,
    #[token("/-")]
    CommentStart,
    #[token("-/")]
    CommentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenError {}

impl Default for TokenError {
    #[cold]
    #[inline(never)]
    fn default() -> Self {
        panic!("logos::Lexer::next can never error")
    }
}

trait SafeUnwrap {
    type Output;

    fn safe_unwrap(self) -> Self::Output;
}

impl<T> SafeUnwrap for Result<T, TokenError> {
    type Output = T;

    #[inline]
    fn safe_unwrap(self) -> Self::Output {
        let Ok(val) = self;
        val
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Token<'text> {
    kind: Kind,
    bytes: &'text bstr::BStr,
    span: Range<usize>,
}

#[derive(Clone)]
pub struct Lexer<'text> {
    peek: Token<'text>,
    lexer: logos::Lexer<'text, Kind>,
    newlines: Vec<usize>,
}

pub struct TextPos {
    pub line: usize,
    pub col: usize,
}

impl core::fmt::Debug for TextPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

pub struct Newlines(Vec<usize>);

impl<'text> Token<'text> {
    #[inline]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    #[inline]
    pub fn lexeme(&self) -> &'text str {
        assert_ne!(self.kind, Kind::InvalidUtf8);
        unsafe { core::str::from_utf8_unchecked(self.bytes) }
    }

    #[inline]
    pub fn as_bytes(&self) -> &'text BStr {
        self.bytes
    }

    #[inline]
    pub fn span(&self) -> Range<usize> {
        self.span
    }
}

impl Newlines {
    pub fn translate(&self, span: Range<usize>) -> Range<TextPos> {
        let line_start = self.0.partition_point(|&i| i <= span.start);

        let remaining = &self.0.as_slice()[line_start..];
        let (head, tail) = remaining.split_at(remaining.len().min(16));
        let line_end = line_start
            + head
                .iter()
                .position(|&i| span.end <= i)
                .unwrap_or_else(|| 16 + tail.partition_point(|&i| i < span.end));

        Range {
            start: TextPos {
                line: line_start,
                col: span.start - self.0[line_start - 1],
            },
            end: TextPos {
                line: line_end,
                col: span.end - self.0[line_end - 1],
            },
        }
    }
}

impl<'text> Lexer<'text> {
    pub fn new(text: &'text (impl ?Sized + AsRef<[u8]>)) -> Self {
        Self::new_(text.as_ref())
    }

    fn new_(text: &'text [u8]) -> Self {
        let mut lexer = Self {
            peek: Token::default(),
            lexer: logos::Lexer::new(text),
            newlines: Vec::new(),
        };

        lexer.lex();

        lexer
    }

    #[inline]
    pub fn finish(mut self) -> Newlines {
        assert_eq!(self.peek.kind, Kind::Eof);
        self.newlines.push(self.lexer.span().end);
        Newlines(self.newlines)
    }

    #[inline]
    pub fn peek_token(&self) -> &Token<'text> {
        &self.peek
    }

    #[inline]
    pub fn peek(&self) -> Kind {
        self.peek.kind
    }

    pub fn lex(&mut self) -> Token<'text> {
        let token = self.peek;

        let mut comment_depth = 0u64;

        loop {
            self.peek = Token {
                kind: (self.lexer.next().transpose())
                    .safe_unwrap()
                    .unwrap_or(Kind::Eof),
                bytes: bstr::BStr::new(self.lexer.slice()),
                span: self.lexer.span().into(),
            };

            if let Kind::Newline = self.peek.kind {
                self.newlines.push(token.span.start);
            }

            match self.peek.kind {
                Kind::Newline | Kind::WhiteSpace => continue,
                Kind::CommentStart => comment_depth = comment_depth.saturating_add(1),
                Kind::CommentEnd if comment_depth > 0 => {
                    comment_depth -= 1;
                    continue;
                }
                Kind::CommentEnd => self.peek.kind = Kind::Error,
                Kind::Eof if comment_depth > 0 => {
                    self.peek.kind = Kind::Error;
                    comment_depth = 0;
                }
                _ => (),
            }

            if comment_depth > 0 {
                continue;
            }

            return token;
        }
    }
}
