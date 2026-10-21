use super::{number::consume_number, Error, ExpectedToken};
use crate::front::wgsl::error::NumberError;
use crate::front::wgsl::parse::{conv, Number};
use crate::front::wgsl::Scalar;
use crate::Span;

type TokenSpan<'a> = (Token<'a>, Span);

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
    Separator(char),
    Paren(char),
    Attribute,
    Number(Result<Number, NumberError>),
    Word(&'a str),
    Operation(char),
    LogicalOperation(char),
    ShiftOperation(char),
    AssignmentOperation(char),
    IncrementOperation,
    DecrementOperation,
    Arrow,
    Unknown(char),
    Trivia,
    End,
}

fn consume_any(input: &str, what: impl Fn(char) -> bool) -> (&str, &str) {
    let pos = input.find(|c| !what(c)).unwrap_or(input.len());
    input.split_at(pos)
}

fn consume_token(input: &str, generic: bool) -> (Token<'_>, &str) {
    let mut chars = input.chars();
    let cur = match chars.next() {
        Some(c) => c,
        None => return (Token::End, ""),
    };
    match cur {
        ':' | ';' | ',' => (Token::Separator(cur), chars.as_str()),
        '.' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('0'..='9') => consume_number(input),
                _ => (Token::Separator(cur), og_chars),
            }
        }
        '@' => (Token::Attribute, chars.as_str()),
        '(' | ')' | '{' | '}' | '[' | ']' => (Token::Paren(cur), chars.as_str()),
        '<' | '>' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('=') if !generic => (Token::LogicalOperation(cur), chars.as_str()),
                Some(c) if c == cur && !generic => {
                    let og_chars = chars.as_str();
                    match chars.next() {
                        Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                        _ => (Token::ShiftOperation(cur), og_chars),
                    }
                }
                _ => (Token::Paren(cur), og_chars),
            }
        }
        '0'..='9' => consume_number(input),
        '/' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('/') => {
                    let _ = chars.position(is_comment_end);
                    (Token::Trivia, chars.as_str())
                }
                Some('*') => {
                    let mut depth = 1;
                    let mut prev = None;

                    for c in &mut chars {
                        match (prev, c) {
                            (Some('*'), '/') => {
                                prev = None;
                                depth -= 1;
                                if depth == 0 {
                                    return (Token::Trivia, chars.as_str());
                                }
                            }
                            (Some('/'), '*') => {
                                prev = None;
                                depth += 1;
                            }
                            _ => {
                                prev = Some(c);
                            }
                        }
                    }

                    (Token::End, "")
                }
                Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        '-' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('>') => (Token::Arrow, chars.as_str()),
                Some('-') => (Token::DecrementOperation, chars.as_str()),
                Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        '+' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('+') => (Token::IncrementOperation, chars.as_str()),
                Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        '*' | '%' | '^' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        '~' => (Token::Operation(cur), chars.as_str()),
        '=' | '!' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some('=') => (Token::LogicalOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        '&' | '|' => {
            let og_chars = chars.as_str();
            match chars.next() {
                Some(c) if c == cur => (Token::LogicalOperation(cur), chars.as_str()),
                Some('=') => (Token::AssignmentOperation(cur), chars.as_str()),
                _ => (Token::Operation(cur), og_chars),
            }
        }
        _ if is_blankspace(cur) => {
            let (_, rest) = consume_any(input, is_blankspace);
            (Token::Trivia, rest)
        }
        _ if is_word_start(cur) => {
            let (word, rest) = consume_any(input, is_word_part);
            (Token::Word(word), rest)
        }
        _ => (Token::Unknown(cur), chars.as_str()),
    }
}

const fn is_comment_end(c: char) -> bool {
    match c {
        '\u{000a}'..='\u{000d}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

const fn is_blankspace(c: char) -> bool {
    match c {
        '\u{0020}'
        | '\u{0009}'..='\u{000d}'
        | '\u{0085}'
        | '\u{200e}'
        | '\u{200f}'
        | '\u{2028}'
        | '\u{2029}' => true,
        _ => false,
    }
}

fn is_word_start(c: char) -> bool {
    c == '_' || unicode_xid::UnicodeXID::is_xid_start(c)
}

fn is_word_part(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
}

#[derive(Clone)]
pub(in crate::front::wgsl) struct Lexer<'a> {
    input: &'a str,
    pub(in crate::front::wgsl) source: &'a str,
    last_end_offset: usize,
}

impl<'a> Lexer<'a> {
    pub(in crate::front::wgsl) const fn new(input: &'a str) -> Self {
        Lexer {
            input,
            source: input,
            last_end_offset: 0,
        }
    }

    #[inline]
    pub fn capture_span<T, E>(
        &mut self,
        inner: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<(T, Span), E> {
        let start = self.current_byte_offset();
        let res = inner(self)?;
        let end = self.current_byte_offset();
        Ok((res, Span::from(start..end)))
    }

    pub(in crate::front::wgsl) fn start_byte_offset(&mut self) -> usize {
        loop {
            let (token, rest) = consume_token(self.input, false);
            if let Token::Trivia = token {
                self.input = rest;
            } else {
                return self.current_byte_offset();
            }
        }
    }

    fn peek_token_and_rest(&mut self) -> (TokenSpan<'a>, &'a str) {
        let mut cloned = self.clone();
        let token = cloned.next();
        let rest = cloned.input;
        (token, rest)
    }

    const fn current_byte_offset(&self) -> usize {
        self.source.len() - self.input.len()
    }

    pub(in crate::front::wgsl) fn span_from(&self, offset: usize) -> Span {
        Span::from(offset..self.last_end_offset)
    }

    #[must_use]
    pub(in crate::front::wgsl) fn next(&mut self) -> TokenSpan<'a> {
        self.next_impl(false)
    }

    #[must_use]
    pub(in crate::front::wgsl) fn next_generic(&mut self) -> TokenSpan<'a> {
        self.next_impl(true)
    }

    fn next_impl(&mut self, generic: bool) -> TokenSpan<'a> {
        let mut start_byte_offset = self.current_byte_offset();
        loop {
            let (token, rest) = consume_token(self.input, generic);
            self.input = rest;
            match token {
                Token::Trivia => start_byte_offset = self.current_byte_offset(),
                _ => {
                    self.last_end_offset = self.current_byte_offset();
                    return (token, self.span_from(start_byte_offset));
                }
            }
        }
    }

    #[must_use]
    pub(in crate::front::wgsl) fn peek(&mut self) -> TokenSpan<'a> {
        let (token, _) = self.peek_token_and_rest();
        token
    }

    pub(in crate::front::wgsl) fn expect_span(
        &mut self,
        expected: Token<'a>,
    ) -> Result<Span, Error<'a>> {
        let next = self.next();
        if next.0 == expected {
            Ok(next.1)
        } else {
            Err(Error::Unexpected(next.1, ExpectedToken::Token(expected)))
        }
    }

    pub(in crate::front::wgsl) fn expect(&mut self, expected: Token<'a>) -> Result<(), Error<'a>> {
        self.expect_span(expected)?;
        Ok(())
    }

    pub(in crate::front::wgsl) fn expect_generic_paren(
        &mut self,
        expected: char,
    ) -> Result<(), Error<'a>> {
        let next = self.next_generic();
        if next.0 == Token::Paren(expected) {
            Ok(())
        } else {
            Err(Error::Unexpected(
                next.1,
                ExpectedToken::Token(Token::Paren(expected)),
            ))
        }
    }

    pub(in crate::front::wgsl) fn skip(&mut self, what: Token<'_>) -> bool {
        let (peeked_token, rest) = self.peek_token_and_rest();
        if peeked_token.0 == what {
            self.input = rest;
            true
        } else {
            false
        }
    }

    pub(in crate::front::wgsl) fn next_ident_with_span(
        &mut self,
    ) -> Result<(&'a str, Span), Error<'a>> {
        match self.next() {
            (Token::Word("_"), span) => Err(Error::InvalidIdentifierUnderscore(span)),
            (Token::Word(word), span) if word.starts_with("__") => {
                Err(Error::ReservedIdentifierPrefix(span))
            }
            (Token::Word(word), span) => Ok((word, span)),
            other => Err(Error::Unexpected(other.1, ExpectedToken::Identifier)),
        }
    }

    pub(in crate::front::wgsl) fn next_ident(
        &mut self,
    ) -> Result<super::ast::Ident<'a>, Error<'a>> {
        let ident = self
            .next_ident_with_span()
            .map(|(name, span)| super::ast::Ident { name, span })?;

        if crate::keywords::wgsl::RESERVED.contains(&ident.name) {
            return Err(Error::ReservedKeyword(ident.span));
        }

        Ok(ident)
    }

    pub(in crate::front::wgsl) fn next_scalar_generic(&mut self) -> Result<Scalar, Error<'a>> {
        self.expect_generic_paren('<')?;
        let pair = match self.next() {
            (Token::Word(word), span) => {
                conv::get_scalar_type(word).ok_or(Error::UnknownScalarType(span))
            }
            (_, span) => Err(Error::UnknownScalarType(span)),
        }?;
        self.expect_generic_paren('>')?;
        Ok(pair)
    }

    pub(in crate::front::wgsl) fn next_scalar_generic_with_span(
        &mut self,
    ) -> Result<(Scalar, Span), Error<'a>> {
        self.expect_generic_paren('<')?;
        let pair = match self.next() {
            (Token::Word(word), span) => conv::get_scalar_type(word)
                .map(|scalar| (scalar, span))
                .ok_or(Error::UnknownScalarType(span)),
            (_, span) => Err(Error::UnknownScalarType(span)),
        }?;
        self.expect_generic_paren('>')?;
        Ok(pair)
    }

    pub(in crate::front::wgsl) fn next_storage_access(
        &mut self,
    ) -> Result<crate::StorageAccess, Error<'a>> {
        let (ident, span) = self.next_ident_with_span()?;
        match ident {
            "read" => Ok(crate::StorageAccess::LOAD),
            "write" => Ok(crate::StorageAccess::STORE),
            "read_write" => Ok(crate::StorageAccess::LOAD | crate::StorageAccess::STORE),
            _ => Err(Error::UnknownAccess(span)),
        }
    }

    pub(in crate::front::wgsl) fn next_format_generic(
        &mut self,
    ) -> Result<(crate::StorageFormat, crate::StorageAccess), Error<'a>> {
        self.expect(Token::Paren('<'))?;
        let (ident, ident_span) = self.next_ident_with_span()?;
        let format = conv::map_storage_format(ident, ident_span)?;
        self.expect(Token::Separator(','))?;
        let access = self.next_storage_access()?;
        self.expect(Token::Paren('>'))?;
        Ok((format, access))
    }

    pub(in crate::front::wgsl) fn open_arguments(&mut self) -> Result<(), Error<'a>> {
        self.expect(Token::Paren('('))
    }

    pub(in crate::front::wgsl) fn close_arguments(&mut self) -> Result<(), Error<'a>> {
        let _ = self.skip(Token::Separator(','));
        self.expect(Token::Paren(')'))
    }

    pub(in crate::front::wgsl) fn next_argument(&mut self) -> Result<bool, Error<'a>> {
        let paren = Token::Paren(')');
        if self.skip(Token::Separator(',')) {
            Ok(!self.skip(paren))
        } else {
            self.expect(paren).map(|()| false)
        }
    }
}










