use std::ops::Range;

use glass_easel_template_compiler::parse::{ParseErrorLevel, Position};
use state::{ParseState, ParseStateOwned};
use token::*;

pub(crate) type Location = Range<Position>;

pub(crate) mod font_face;
pub(crate) mod import;
pub(crate) mod key_frame;
pub(crate) mod media;
pub(crate) mod property;
pub(crate) mod rule;
pub(crate) mod token;

trait CSSParse: Sized {
    fn css_parse(ps: &mut ParseState) -> Option<Self>;
}

pub(crate) struct StyleSheet {
    pub(crate) items: Vec<Item>,
    pub(crate) comments: Vec<Comment>,
}

impl StyleSheet {
    fn parse_str(src: &str) -> (Self, Vec<ParseError>) {
        let mut pso = ParseStateOwned::new(src.to_string());
        let mut items: Vec<Item> = vec![];
        pso.run(|mut ps| {
            while let Some(item) = Item::css_parse(&mut ps) {
                items.push(item);
            }
        });
        let ret = Self {
            items,
            comments: pso.extract_comments(),
        };
        let warnings = pso.extract_warnings();
        (ret, warnings)
    }
}

pub(crate) enum Item {
    Unknown(Vec<TokenTree>),
    Property(property::Property),
    Style(rule::StyleRule),
    Import(import::ImportRule),
    Media(media::MediaRule),
    FontFace(font_face::FontFaceRule),
    KeyFrames(key_frame::KeyFramesRule),
    UnknownAtRule(AtKeyword, Vec<TokenTree>),
}

impl CSSParse for Item {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!() // TODO
    }
}

mod state {
    use std::mem::ManuallyDrop;

    use compact_str::CompactString;
    use cssparser::{BasicParseErrorKind, Token as CSSToken};

    use super::*;

    pub(super) struct ParseStateOwned {
        src: ManuallyDrop<String>,
        input: ManuallyDrop<cssparser::ParserInput<'static>>,
        parser: ManuallyDrop<cssparser::Parser<'static, 'static>>,
        warnings: Vec<ParseError>,
        comments: Vec<Comment>,
    }

    impl Drop for ParseStateOwned {
        fn drop(&mut self) {
            unsafe {
                ManuallyDrop::drop(&mut self.parser);
                ManuallyDrop::drop(&mut self.input);
                ManuallyDrop::drop(&mut self.src);
            }
        }
    }

    impl ParseStateOwned {
        pub(super) fn new(src: String) -> Self {
            let src = ManuallyDrop::new(src);
            let mut input = ManuallyDrop::new({
                let src: &str = src.as_str();
                let src_static: &'static str = unsafe { &*(src as *const _) };
                cssparser::ParserInput::new(src_static)
            });
            let parser = ManuallyDrop::new({
                let input: &mut cssparser::ParserInput = &mut input;
                let input_static: &'static mut cssparser::ParserInput = unsafe { &mut *(input as *mut _) };
                cssparser::Parser::new(input_static)
            });
            Self {
                src,
                input,
                parser,
                warnings: vec![],
                comments: vec![],
            }
        }

        pub(super) fn run(&mut self, f: impl FnOnce(ParseState)) {
            let Self { parser, warnings, comments, .. } = self;
            let _ = parser.parse_entirely::<_, _, ()>(|parser| {
                let ps = ParseState {
                    parser,
                    warnings,
                    comments,
                };
                f(ps);
                if !parser.is_exhausted() {
                    let pos = parser_position(parser);
                    warnings.push(ParseError { kind: ParseErrorKind::UnexpectedToken, location: pos..pos })
                }
                while !parser.is_exhausted() {
                    let _ = parser.next();
                }
                Ok(())
            });
        }

        pub(super) fn extract_warnings(&mut self) -> Vec<ParseError> {
            std::mem::replace(&mut self.warnings, vec![])
        }

        pub(super) fn extract_comments(&mut self) -> Vec<Comment> {
            std::mem::replace(&mut self.comments, vec![])
        }
    }

    fn parser_position(parser: &cssparser::Parser<'static, '_>) -> Position {
        let p = parser.current_source_location();
        Position {
            line: p.line,
            utf16_col: p.column,
        }
    }

    fn convert_css_token(css_token: &CSSToken, location: Location) -> TokenTree {
        match css_token {
            CSSToken::Ident(s) => {
                TokenTree::Ident(Ident { content: CompactString::new(s), location })
            }
            CSSToken::AtKeyword(s) => {
                TokenTree::AtKeyword(AtKeyword { content: CompactString::new(s), location })
            }
            CSSToken::Hash(s) => {
                TokenTree::Hash(Hash { content: CompactString::new(s), location })
            }
            CSSToken::IDHash(s) => {
                TokenTree::IDHash(IDHash { content: CompactString::new(s), location })
            }
            CSSToken::QuotedString(s) => {
                TokenTree::QuotedString(QuotedString { content: CompactString::new(s), location })
            }
            CSSToken::UnquotedUrl(s) => {
                TokenTree::UnquotedUrl(UnquotedUrl { content: CompactString::new(s), location })
            }
            CSSToken::Number { has_sign, value, int_value } => {
                TokenTree::Number(Number { has_sign: *has_sign, value: *value, int_value: *int_value, location })
            }
            CSSToken::Percentage { has_sign, unit_value, int_value } => {
                TokenTree::Percentage(Percentage { has_sign: *has_sign, value: *unit_value, int_value: *int_value, location })
            }
            CSSToken::Dimension { has_sign, value, int_value, unit } => {
                TokenTree::Dimension(Dimension { has_sign: *has_sign, value: *value, int_value: *int_value, unit: CompactString::new(unit), location })
            }
            CSSToken::Colon => {
                TokenTree::Colon(Colon { location })
            }
            CSSToken::Semicolon => {
                TokenTree::Semicolon(Semicolon { location })
            }
            CSSToken::Comma => {
                TokenTree::Comma(Comma { location })
            }
            CSSToken::Delim(ch) => {
                TokenTree::Operator(Operator::new(&ch.to_string(), location))
            }
            CSSToken::IncludeMatch => {
                TokenTree::Operator(Operator::new("~=", location))
            }
            CSSToken::DashMatch => {
                TokenTree::Operator(Operator::new("|=", location))
            }
            CSSToken::PrefixMatch => {
                TokenTree::Operator(Operator::new("^=", location))
            }
            CSSToken::SuffixMatch => {
                TokenTree::Operator(Operator::new("$=", location))
            }
            CSSToken::SubstringMatch => {
                TokenTree::Operator(Operator::new("*=", location))
            }
            CSSToken::CDO => {
                TokenTree::Operator(Operator::new("<!--", location))
            }
            CSSToken::CDC => {
                TokenTree::Operator(Operator::new("-->", location))
            }
            CSSToken::BadUrl(s) => {
                TokenTree::BadUrl(BadUrl { content: CompactString::new(s), location })
            }
            CSSToken::BadString(s) => {
                TokenTree::BadString(BadString { content: CompactString::new(s), location })
            }
            CSSToken::Function(name) => {
                let paren = Paren::new(vec![], location.end..location.end);
                let name = Ident { content: CompactString::new(name), location };
                TokenTree::Function(Function { name, paren })
            }
            CSSToken::ParenthesisBlock => {
                TokenTree::Paren(Paren::new(vec![], location))
            }
            CSSToken::SquareBracketBlock => {
                TokenTree::Bracket(Bracket::new(vec![], location))
            }
            CSSToken::CurlyBracketBlock => {
                TokenTree::Brace(Brace::new(vec![], location))
            }
            CSSToken::WhiteSpace(..)
            | CSSToken::Comment(..)
            | CSSToken::CloseParenthesis
            | CSSToken::CloseSquareBracket
            | CSSToken::CloseCurlyBracket => unreachable!(),
        }
    }

    pub(super) struct ParseState<'a, 'b> {
        parser: &'a mut cssparser::Parser<'static, 'b>,
        warnings: &'a mut Vec<ParseError>,
        comments: &'a mut Vec<Comment>,
    }

    impl<'a, 'b> ParseState<'a, 'b> {
        pub(super) fn add_warning(&mut self, kind: ParseErrorKind, location: Location) {
            self.warnings.push(ParseError {
                kind,
                location,
            })
        }

        pub(super) fn add_warning_at_current_position(&mut self, kind: ParseErrorKind) {
            let pos = self.position();
            self.add_warning(kind, pos..pos);
        }

        pub(super) fn position(&self) -> Position {
            parser_position(&self.parser)
        }

        pub(super) fn next(&mut self) -> Option<TokenTree> {
            fn rec(
                parser: &mut cssparser::Parser<'static, '_>,
                warnings: &mut Vec<ParseError>,
                comments: &mut Vec<Comment>,
            ) -> Option<TokenTree> {
                loop {
                    let start_pos = parser_position(&parser);
                    let next = parser.next_including_whitespace_and_comments().cloned();
                    let end_pos = parser_position(&parser);
                    let location = start_pos..end_pos;
                    match next {
                        Err(err) => {
                            match err.kind {
                                BasicParseErrorKind::EndOfInput => {
                                    break None;
                                }
                                _ => {
                                    warnings.push(ParseError { kind: ParseErrorKind::UnexpectedToken, location });
                                    break None;
                                }
                            }
                        }
                        Ok(css_token) => {
                            let mut token = match css_token {
                                CSSToken::Comment(s) => {
                                    comments.push(Comment { content: CompactString::new(s), location });
                                    continue;
                                }
                                CSSToken::WhiteSpace(_) => {
                                    continue;
                                }
                                x => convert_css_token(&x, location),
                            };
                            if token.children().is_some() {
                                let mut children = vec![];
                                let _ = parser.parse_nested_block::<_, _, ()>(|parser| {
                                    while let Some(token) = rec(parser, warnings, comments) {
                                        children.push(token);
                                    }
                                    Ok(())
                                });
                                let end_pos = parser_position(&parser);
                                let location = start_pos..end_pos;
                                match &mut token {
                                    TokenTree::Function(x) => {
                                        x.paren.children = children;
                                        x.paren.location = location;
                                    }
                                    TokenTree::Paren(x) => {
                                        x.children = children;
                                        x.location = location;
                                    }
                                    TokenTree::Bracket(x) => {
                                        x.children = children;
                                        x.location = location;
                                    }
                                    TokenTree::Brace(x) => {
                                        x.children = children;
                                        x.location = location;
                                    }
                                    _ => unreachable!(),
                                }
                            }
                            break Some(token);
                        }
                    }
                }
            }
            rec(&mut self.parser, &mut self.warnings, &mut self.comments)
        }

        pub(super) fn peek(&mut self) -> Option<TokenTree> {
            let state = self.parser.state();
            let start_pos = parser_position(&self.parser);
            let next = self.parser.next();
            let ret = match next {
                Err(_) => None,
                Ok(next) => Some(convert_css_token(next, start_pos..start_pos)),
            };
            self.parser.reset(&state);
            ret
        }

        pub(super) fn peek_ident(&mut self) -> bool {
            match self.peek() {
                Some(TokenTree::Ident(..)) => true,
                _ => false,
            }
        }

        fn parse_nested<R>(&mut self, f: impl FnOnce(ParseState) -> R) -> Option<R> {
            let Self { parser, warnings, comments } = self;
            parser.parse_nested_block::<_, _, ()>(|parser| {
                let ps = ParseState {
                    parser,
                    warnings,
                    comments,
                };
                let r = f(ps);
                if !parser.is_exhausted() {
                    let pos = parser_position(parser);
                    warnings.push(ParseError { kind: ParseErrorKind::UnexpectedToken, location: pos..pos })
                }
                while !parser.is_exhausted() {
                    let _ = parser.next();
                }
                Ok(Some(r))
            }).unwrap()
        }

        pub(super) fn parse_paren<R>(&mut self, f: impl FnOnce(ParseState) -> R) -> Option<Paren<R>> {
            match self.peek() {
                Some(TokenTree::Paren(..)) => {}
                _ => return None,
            }
            let start_pos = self.position();
            let r = self.parse_nested(f)?;
            let end_pos = self.position();
            Some(Paren::new(r, start_pos..end_pos))
        }

        pub(super) fn parse_bracket<R>(&mut self, f: impl FnOnce(ParseState) -> R) -> Option<Bracket<R>> {
            match self.peek() {
                Some(TokenTree::Bracket(..)) => {}
                _ => return None,
            }
            let start_pos = self.position();
            let r = self.parse_nested(f)?;
            let end_pos = self.position();
            Some(Bracket::new(r, start_pos..end_pos))
        }

        pub(super) fn parse_brace<R>(&mut self, f: impl FnOnce(ParseState) -> R) -> Option<Brace<R>> {
            match self.peek() {
                Some(TokenTree::Brace(..)) => {}
                _ => return None,
            }
            let start_pos = self.position();
            let r = self.parse_nested(f)?;
            let end_pos = self.position();
            Some(Brace::new(r, start_pos..end_pos))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParseError {
    pub(crate) kind: ParseErrorKind,
    pub(crate) location: Range<Position>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "style sheet parsing error at {}:{}-{}:{}: {}",
            self.location.start.line + 1,
            self.location.start.utf16_col + 1,
            self.location.end.line + 1,
            self.location.end.utf16_col + 1,
            self.kind,
        )
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub(crate) fn level(&self) -> ParseErrorLevel {
        self.kind.level()
    }

    pub(crate) fn code(&self) -> u32 {
        self.kind.clone() as u32
    }

    pub(crate) fn prevent_success(&self) -> bool {
        self.level() >= ParseErrorLevel::Error
    }
}

#[repr(u32)]
#[derive(Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken = 0x10001,
}

impl ParseErrorKind {
    fn static_message(&self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected token",
        }
    }

    pub fn level(&self) -> ParseErrorLevel {
        match self {
            Self::UnexpectedToken => ParseErrorLevel::Fatal,
        }
    }
}

impl std::fmt::Debug for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.static_message())
    }
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.static_message())
    }
}
