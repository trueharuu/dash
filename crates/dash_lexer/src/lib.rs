use std::borrow::Cow;
use std::ops::Range;

use dash_middle::lexer::error::{Error, ErrorKind};
use dash_middle::lexer::token::{Location, Token, TokenType};
use dash_middle::util;

/// A JavaScript source code lexer
#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a [u8],

    tokens: Vec<Token<'a>>,
    errors: Vec<Error<'a>>,

    idx: usize,
    line: usize,
    start: usize,
    line_idx: usize,
    template_literal_depths_stack: Vec<usize>,
}

/// Represents a comment
#[derive(Debug)]
pub enum CommentKind {
    /// A multiline comment: /* */
    Multiline,
    /// A singleline comment: //
    Singleline,
}

/// A lexer node (either a token or an error)
#[derive(Debug)]
pub enum Node<'a> {
    /// A valid token
    Token(Token<'a>),
    /// An error
    Error(Error<'a>),
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer
    pub fn new(source: &'a str) -> Self {
        Self {
            input: source.as_bytes(),
            idx: 0,
            line: 1,
            start: 0,
            line_idx: 0,
            template_literal_depths_stack: Vec::new(),
            errors: Vec::new(),
            tokens: Vec::new(),
        }
    }

    /// This lexer is exhausted and has reached the end of the string
    fn is_eof(&self) -> bool {
        self.idx >= self.input.len()
    }

    /// Returns the next character
    fn next_char(&mut self) -> Option<u8> {
        let cur = self.current()?;
        self.advance();
        Some(cur)
    }

    /// Returns the current byte
    fn current(&self) -> Option<u8> {
        self.input.get(self.idx).copied()
    }

    /// Looks ahead by one and returns the next byte
    fn peek(&self) -> Option<u8> {
        self.input.get(self.idx + 1).copied()
    }

    /// Returns the current byte, without returning an Option
    fn current_real(&self) -> u8 {
        self.input[self.idx]
    }

    /// Creates a token based on the current location
    fn create_contextified_token(&mut self, ty: TokenType) {
        let tok = Token {
            ty,
            loc: Location {
                line: self.line,
                offset: self.start,
                line_offset: self.line_idx,
            },
            full: Cow::Borrowed(self.get_lexeme()),
        };
        self.tokens.push(tok);
    }

    /// Creates a token based on the current location and a given predicate
    ///
    /// A token may be multiple bytes wide, in which case this function can be used.
    /// This function can be seen as a helper function to create a token based on the next bytes.
    fn create_contextified_conditional_token(&mut self, default: Option<TokenType>, tokens: &[(&[u8], TokenType)]) {
        for (expect, token) in tokens {
            let from = self.idx;
            let slice = self.safe_subslice(from, from + expect.len());

            if slice.eq(*expect) {
                self.create_contextified_token(*token);
                self.idx += expect.len();
                return;
            }
        }

        if let Some(tt) = default {
            self.create_contextified_token(tt);
        } else {
            // TODO: can we actually reach this branch?
            unreachable!()
        }
    }

    /// Creates a new error token
    fn create_error(&mut self, kind: ErrorKind) {
        let err = Error {
            loc: Location {
                line: self.line,
                offset: self.start,
                line_offset: self.line_idx,
            },
            kind,
            source: self.input,
        };
        self.errors.push(err);
    }

    /// Creates a token based on the current location and a given lexeme
    fn create_contextified_token_with_lexeme(&mut self, ty: TokenType, lexeme: Cow<'a, str>) {
        let tok = Token {
            ty,
            loc: Location {
                line: self.line,
                offset: match lexeme {
                    Cow::Borrowed(lexeme) => lexeme.as_ptr() as usize - self.input.as_ptr() as usize,
                    // TODO: Handle the Cow::Owned case properly, somehow
                    _ => 0,
                },
                line_offset: self.line_idx,
            },
            full: lexeme,
        };
        self.tokens.push(tok);
    }

    /// Returns the current lexeme
    fn get_lexeme(&self) -> &'a str {
        util::force_utf8_borrowed(&self.input[self.start..self.idx])
    }

    /// Slices into the source string
    fn subslice(&self, r: Range<usize>) -> &'a [u8] {
        &self.input[r]
    }

    /// Slices into the source string, but makes sure no panic occurs
    fn safe_subslice(&self, from: usize, to: usize) -> &'a [u8] {
        let from = from.max(0);
        let to = to.min(self.input.len());
        &self.input[from..to]
    }

    /// Advances the cursor
    fn advance(&mut self) {
        self.idx += 1;
    }

    /// Advances the cursor by n
    fn advance_n(&mut self, n: usize) {
        self.idx += n;
    }

    /// Expects the current byte to be `expected` and advances the stream if matched
    fn expect_and_skip(&mut self, expected: u8) -> bool {
        let cur = match self.current() {
            Some(c) => c,
            None => return false,
        };

        if !cur.eq(&expected) {
            return false;
        }

        self.advance();

        true
    }

    /// Reads a string literal
    ///
    /// This function expects to be one byte ahead of a quote
    fn read_string_literal(&mut self) {
        let quote = self.input[self.idx - 1];
        let mut found_quote = false;

        let mut lexeme: Option<Cow<'a, str>> = None;
        let mut lexeme_starting_idx = self.idx;

        while !self.is_eof() {
            let cur = self.current_real();
            if cur == quote {
                self.advance();
                found_quote = true;
                break;
            }

            if cur == b'\n' {
                self.line += 1;
                self.line_idx = self.idx;
            }

            if cur == b'\\' {
                // Append borrowed segment since last escape sequence
                let segment = util::force_utf8_borrowed(self.subslice(lexeme_starting_idx..self.idx));
                match &mut lexeme {
                    Some(lexeme) => lexeme.to_mut().push_str(segment),
                    None => lexeme = Some(Cow::Borrowed(segment)),
                };

                self.advance();
                let escape = self.current_real();
                match escape {
                    b'n' | b't' | b'r' | b'b' | b'f' | b'v' => {
                        lexeme.as_mut().unwrap().to_mut().push(match escape {
                            b'n' => '\n',
                            b't' => '\t',
                            b'r' => '\r',
                            b'b' => '\x08',
                            b'f' => '\x0C',
                            b'v' => '\x0B',
                            _ => unreachable!(),
                        });
                        self.advance();
                    }
                    other if !other.is_ascii() => {
                        // if the escaped character is non-ascii, decode UTF-8
                        let (c, len) = util::next_char_in_bytes(&self.input[self.idx..]);
                        lexeme.as_mut().unwrap().to_mut().push(c);
                        self.advance_n(len);
                    }
                    // TODO: handle \u, \x
                    other => {
                        lexeme.as_mut().unwrap().to_mut().push(other as char);
                        self.advance();
                    }
                }
                lexeme_starting_idx = self.idx;

                continue;
            }

            self.advance();
        }

        let lexeme = match lexeme {
            None => Cow::Borrowed(util::force_utf8_borrowed(self.subslice(self.start + 1..self.idx - 1))),
            Some(Cow::Owned(mut lexeme)) => {
                lexeme.push_str(util::force_utf8_borrowed(
                    self.subslice(lexeme_starting_idx..self.idx - 1),
                ));
                Cow::Owned(lexeme)
            }
            Some(Cow::Borrowed(..)) => unreachable!("Lexeme cannot be borrowed at this point"),
        };

        if !found_quote && self.is_eof() {
            return self.create_error(ErrorKind::UnexpectedEof);
        }

        self.create_contextified_token_with_lexeme(TokenType::String, lexeme);
    }

    /// Reads a prefixed number literal (0x, 0b, 0o)
    fn read_prefixed_literal<P>(&mut self, ty: TokenType, predicate: P)
    where
        P: Fn(u8) -> bool,
    {
        // Skip prefix (0x)
        self.advance();

        while !self.is_eof() {
            let cur = self.current_real();

            if cur == b'_' || predicate(cur) {
                self.advance();
            } else {
                break;
            }
        }

        self.create_contextified_token(ty);
    }

    /// Reads a number literal
    fn read_number_literal(&mut self) {
        let mut is_float = false;
        let mut is_exp = false;

        while !self.is_eof() {
            let cur = self.current_real();

            match cur {
                b'.' => {
                    if is_float {
                        break;
                    }

                    is_float = true;
                }
                b'e' => {
                    if is_exp {
                        break;
                    }

                    // Handle minus after e, like 1e-5
                    if matches!(self.peek(), Some(b'-')) {
                        self.advance();
                    }

                    is_exp = true;
                }
                _ => {
                    if !util::is_digit(cur) {
                        break;
                    }
                }
            }

            self.advance();
        }

        self.create_contextified_token(TokenType::NumberDec)
    }

    fn read_template_literal_segment(&mut self) {
        let mut found_end = false;
        let mut is_interpolated = false;

        while !self.is_eof() {
            let cur = self.current_real();
            if cur == b'`' {
                self.advance();
                found_end = true;
                break;
            }

            if cur == b'$' {
                if let Some(b'{') = self.peek() {
                    // String interpolation
                    found_end = true;
                    is_interpolated = true;
                    self.template_literal_depths_stack.push(0);
                    break;
                }
            }

            if cur == b'\n' {
                self.line += 1;
                self.line_idx = self.idx;
            }

            self.advance();
        }

        if !found_end && self.is_eof() {
            return self.create_error(ErrorKind::UnexpectedEof);
        }

        let range = match is_interpolated {
            true => self.start + 1..self.idx,
            false => self.start + 1..self.idx - 1,
        };

        let lexeme = util::force_utf8_borrowed(self.subslice(range));
        self.create_contextified_token_with_lexeme(TokenType::TemplateLiteral, Cow::Borrowed(lexeme));
    }

    /// Reads an identifier and returns it as a node
    fn read_identifier(&mut self) {
        while !self.is_eof() {
            let cur = self.current_real();

            if !util::is_alpha(cur) {
                break;
            }

            self.advance();
        }

        let lexeme = self.get_lexeme();
        self.create_contextified_token(lexeme.into());
    }

    /// Reads a regex literal, assuming the current cursor is one byte ahead of the `/`
    fn read_regex_literal(&mut self) {
        // No real regex parsing here, we only skip to the end of the regex literal here.
        while !self.is_eof() {
            let c = self.next_char().unwrap();
            if c == b'/' {
                // End of regex literal
                break;
            } else if c == b'\\' {
                // Skip escaped character
                self.advance();
            }
        }

        let lexeme = self.get_lexeme();
        self.create_contextified_token_with_lexeme(TokenType::RegexLiteral, Cow::Borrowed(lexeme));
    }

    /// Iterates through the input string and yields the next node
    pub fn scan_next(&mut self) -> Option<()> {
        self.skip_whitespaces();
        while self.current() == Some(b'/') {
            let index_before_skip = self.idx;
            self.skip_comments();

            // We need to manually break out of the loop if the index didn't change
            // This is the case when visiting a single slash
            if self.idx == index_before_skip {
                break;
            }

            self.skip_whitespaces();
        }
        self.skip_whitespaces();
        self.start = self.idx;

        let cur = self.next_char()?;

        match cur {
            b'$' => {
                if util::is_alpha(self.current()?) {
                    self.read_identifier();
                } else {
                    self.create_contextified_token(TokenType::Dollar);
                }
            }
            b'(' => self.create_contextified_token(TokenType::LeftParen),
            b')' => self.create_contextified_token(TokenType::RightParen),
            b'{' => {
                if let Some(depth) = self.template_literal_depths_stack.last_mut() {
                    *depth += 1;
                }

                self.create_contextified_token(TokenType::LeftBrace)
            }
            b'}' => {
                self.create_contextified_token(TokenType::RightBrace);

                if let Some(depth) = self.template_literal_depths_stack.last_mut() {
                    *depth -= 1;
                    if *depth == 0 {
                        self.template_literal_depths_stack.pop();
                        self.read_template_literal_segment();
                    }
                }
                // if self.template_literal_depth > 0 {
                //     self.template_literal_depth -= 1;
                //     self.read_template_literal_segment();
                // }
            }
            b'[' => self.create_contextified_conditional_token(
                Some(TokenType::LeftSquareBrace),
                &[(b"]", TokenType::EmptySquareBrace)],
            ),
            b']' => self.create_contextified_token(TokenType::RightSquareBrace),
            b',' => self.create_contextified_token(TokenType::Comma),
            b'.' => self.create_contextified_token(TokenType::Dot),
            b'-' => self.create_contextified_conditional_token(
                Some(TokenType::Minus),
                &[(b"-", TokenType::Decrement), (b"=", TokenType::SubtractionAssignment)],
            ),
            b'+' => self.create_contextified_conditional_token(
                Some(TokenType::Plus),
                &[(b"+", TokenType::Increment), (b"=", TokenType::AdditionAssignment)],
            ),
            b'*' => self.create_contextified_conditional_token(
                Some(TokenType::Star),
                &[
                    (b"*=", TokenType::ExponentiationAssignment),
                    (b"*", TokenType::Exponentiation),
                    (b"=", TokenType::MultiplicationAssignment),
                ],
            ),
            b'|' => self.create_contextified_conditional_token(
                Some(TokenType::BitwiseOr),
                &[
                    (b"|=", TokenType::LogicalOrAssignment),
                    (b"=", TokenType::BitwiseOrAssignment),
                    (b"|", TokenType::LogicalOr),
                ],
            ),
            b'^' => self.create_contextified_conditional_token(
                Some(TokenType::BitwiseXor),
                &[(b"=", TokenType::BitwiseXorAssignment)],
            ),
            b'&' => self.create_contextified_conditional_token(
                Some(TokenType::BitwiseAnd),
                &[
                    (b"&=", TokenType::LogicalAndAssignment),
                    (b"=", TokenType::BitwiseAndAssignment),
                    (b"&", TokenType::LogicalAnd),
                ],
            ),
            b'>' => self.create_contextified_conditional_token(
                Some(TokenType::Greater),
                &[
                    (b">>=", TokenType::UnsignedRightShiftAssignment),
                    (b">=", TokenType::RightShiftAssignment),
                    (b">>", TokenType::UnsignedRightShift),
                    (b"=", TokenType::GreaterEqual),
                    (b">", TokenType::RightShift),
                ],
            ),
            b'<' => self.create_contextified_conditional_token(
                Some(TokenType::Less),
                &[
                    (b"<=", TokenType::LeftShiftAssignment),
                    (b"=", TokenType::LessEqual),
                    (b"<", TokenType::LeftShift),
                ],
            ),
            b'%' => self.create_contextified_conditional_token(
                Some(TokenType::Remainder),
                &[(b"=", TokenType::RemainderAssignment)],
            ),
            b'/' => {
                // '/' is very ambiguous, probably the most ambiguous character in the grammar
                // Comments (both single line and multi line) have already been checked for,
                // so the only ambiguity left is the division operator and the start of a regex literal.
                // It is impossible (as far as I'm aware) to fully distuingish these at the lexer level,
                // as the lexer does not understand grammar (i.e. where a certain token is valid).
                // But we also HAVE to special case regex literals here in the lexer as they can contain any character,
                // and should not be parsed as JS source tokens (whitespaces are not preserved at the parser level),
                // much like how string literals work.

                // One way that "works" for most cases is to look at the previous token:
                // If the previous token was a token that syntactically requires an expression to follow (not an operator),
                // then the '/' MUST be the start of a regex literal.
                // For example: `let x = /b/` is a regex literal, because `=` requires an expression.
                // `a /b/ c` is not a regex literal, because `a` must NOT be followed by another expression.
                // Unfortunately, even the previous token can be ambiguous, for example:
                // `+{}  /a/g` : /a/ is NOT a regex literal
                // `{}   /a/g` : /a/ IS a regex literal
                // The previous token is the same in both cases `}`, but is parsed differently depending on whether
                // `}` ends a code block or an object literal.

                const PRECEDING_TOKENS: &[TokenType] = &[
                    // Symbols
                    TokenType::Dot,
                    TokenType::LeftParen,
                    TokenType::LeftBrace,
                    TokenType::LeftSquareBrace,
                    TokenType::Semicolon,
                    TokenType::Comma,
                    TokenType::Less,
                    TokenType::Greater,
                    TokenType::LessEqual,
                    TokenType::GreaterEqual,
                    TokenType::Equality,
                    TokenType::Inequality,
                    TokenType::StrictEquality,
                    TokenType::StrictInequality,
                    TokenType::Plus,
                    TokenType::Minus,
                    TokenType::Star,
                    TokenType::Remainder,
                    TokenType::Increment,
                    TokenType::Decrement,
                    TokenType::LeftShift,
                    TokenType::RightShift,
                    TokenType::UnsignedRightShift,
                    TokenType::BitwiseAnd,
                    TokenType::BitwiseOr,
                    TokenType::BitwiseXor,
                    TokenType::LogicalNot,
                    TokenType::BitwiseNot,
                    TokenType::LogicalAnd,
                    TokenType::LogicalOr,
                    TokenType::Conditional,
                    TokenType::Colon,
                    TokenType::Assignment,
                    TokenType::AdditionAssignment,
                    TokenType::SubtractionAssignment,
                    TokenType::MultiplicationAssignment,
                    TokenType::RemainderAssignment,
                    TokenType::LeftShiftAssignment,
                    TokenType::RightShiftAssignment,
                    TokenType::UnsignedRightShiftAssignment,
                    TokenType::BitwiseAndAssignment,
                    TokenType::BitwiseOrAssignment,
                    TokenType::BitwiseXorAssignment,
                    TokenType::Slash,
                    TokenType::DivisionAssignment,
                    // Keywords
                    TokenType::New,
                    TokenType::Delete,
                    TokenType::Void,
                    TokenType::Typeof,
                    TokenType::Instanceof,
                    TokenType::In,
                    // TokenType::Do,
                    TokenType::Return,
                    TokenType::Case,
                    TokenType::Throw,
                    TokenType::Else,
                    TokenType::Await,
                    TokenType::Yield,
                ];

                match self.tokens.last() {
                    Some(token) if PRECEDING_TOKENS.contains(&token.ty) => self.read_regex_literal(),
                    None => self.read_regex_literal(),
                    _ => self.create_contextified_conditional_token(
                        Some(TokenType::Slash),
                        &[(b"=", TokenType::DivisionAssignment)],
                    ),
                }
            }
            b'!' => self.create_contextified_conditional_token(
                Some(TokenType::LogicalNot),
                &[(b"==", TokenType::StrictInequality), (b"=", TokenType::Inequality)],
            ),
            b'~' => self.create_contextified_token(TokenType::BitwiseNot),
            b'?' => self.create_contextified_conditional_token(
                Some(TokenType::Conditional),
                &[
                    (b"?=", TokenType::LogicalNullishAssignment),
                    (b"?", TokenType::NullishCoalescing),
                    (b".", TokenType::OptionalChaining),
                ],
            ),
            b'#' => self.create_contextified_token(TokenType::Hash),
            b':' => self.create_contextified_token(TokenType::Colon),
            b';' => self.create_contextified_token(TokenType::Semicolon),
            b'=' => self.create_contextified_conditional_token(
                Some(TokenType::Assignment),
                &[
                    (b"==", TokenType::StrictEquality),
                    (b"=", TokenType::Equality),
                    (b">", TokenType::FatArrow),
                ],
            ),
            b'"' | b'\'' => self.read_string_literal(),
            b'`' => self.read_template_literal_segment(),
            _ => {
                if util::is_digit(cur) {
                    let is_prefixed = cur == b'0';

                    match (is_prefixed, self.current()) {
                        (true, Some(b'x' | b'X')) => {
                            self.read_prefixed_literal(TokenType::NumberHex, util::is_hex_digit)
                        }
                        (true, Some(b'b' | b'B')) => {
                            self.read_prefixed_literal(TokenType::NumberBin, util::is_binary_digit)
                        }
                        (true, Some(b'o' | b'O')) => {
                            self.read_prefixed_literal(TokenType::NumberOct, util::is_octal_digit)
                        }
                        _ => self.read_number_literal(),
                    }
                } else if util::is_identifier_start(cur) {
                    self.read_identifier()
                } else {
                    self.create_error(ErrorKind::UnknownCharacter(cur));
                }
            }
        };
        Some(())
    }

    /// Skips any meaningless whitespaces
    fn skip_whitespaces(&mut self) {
        while !self.is_eof() {
            let ch = match self.current() {
                Some(c) => c,
                None => return,
            };

            match ch {
                b'\n' => {
                    self.line += 1;
                    self.line_idx = self.idx;
                }
                b'\r' | b'\t' | b' ' => {}
                _ => return,
            };

            self.advance();
        }
    }

    /// Skips any comments
    fn skip_comments(&mut self) {
        let cur = match self.current() {
            Some(c) => c,
            None => return,
        };

        if cur == b'/' {
            match self.peek() {
                Some(b'/') => self.skip_single_line_comment(),
                Some(b'*') => self.skip_multi_line_comment(),
                _ => {}
            };
        }
    }

    /// Skips a single line comment
    fn skip_single_line_comment(&mut self) {
        while !self.is_eof() {
            let ch = match self.current() {
                Some(c) => c,
                None => return,
            };

            if ch == b'\n' {
                self.line += 1;
                self.line_idx = self.idx;
                return;
            }

            self.advance();
        }
    }

    /// Skips a multi line comment
    fn skip_multi_line_comment(&mut self) {
        self.expect_and_skip(b'/');
        self.expect_and_skip(b'*');
        while !self.is_eof() {
            let ch = match self.current() {
                Some(c) => c,
                None => return,
            };

            if ch == b'\n' {
                self.line += 1;
                self.line_idx = self.idx;
            } else if ch == b'*' && self.peek() == Some(b'/') {
                self.advance_n(2);
                return;
            }

            self.advance();
        }
    }

    /// Drives this lexer to completion
    ///
    /// Calling this function will exhaust the lexer and return all nodes
    pub fn scan_all(mut self) -> Result<Vec<Token<'a>>, Vec<Error<'a>>> {
        while !self.is_eof() {
            self.scan_next();
        }
        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }
}
