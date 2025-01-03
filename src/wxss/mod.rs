use std::ops::Range;

use glass_easel_template_compiler::parse::{ParseErrorLevel, Position as _Position};
use state::{ParseState, ParseStateOwned};
use token::*;

pub(crate) type Position = _Position;
pub(crate) type Location = Range<Position>;

pub(crate) mod font_face;
pub(crate) mod import;
pub(crate) mod keyframe;
pub(crate) mod media;
pub(crate) mod property;
pub(crate) mod rule;
pub(crate) mod token;

trait CSSParse: Sized {
    /// Do real parsing.
    /// 
    /// Returns `None` if it cannot be parsed at all.
    /// Otherwise, try parse as many tokens as possible,
    /// and generates warnings and collect comments if any.
    fn css_parse(ps: &mut ParseState) -> Option<Self>;
}

impl<C: CSSParse> CSSParse for Vec<C> {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let mut items = vec![];
        while let Some(c) = C::css_parse(ps) {
            items.push(c);
        }
        Some(items)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StyleSheet {
    pub(crate) items: Vec<Rule>,
    pub(crate) comments: Vec<Comment>,
}

impl StyleSheet {
    pub(crate) fn parse_str(src: &str) -> (Self, Vec<ParseError>) {
        let mut pso = ParseStateOwned::new(src.to_string());
        let mut items: Vec<Rule> = vec![];
        pso.run(|mut ps| {
            while let Some(item) = Rule::css_parse(&mut ps) {
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

#[derive(Debug, Clone)]
pub(crate) enum RuleOrProperty {
    Rule(Rule),
    Property(property::Property),
}

impl CSSParse for RuleOrProperty {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        if let Some((TokenTree::Ident(..), TokenTree::Colon(..))) = ps.peek2() {
            if let Some(p) = property::Property::css_parse(ps) {
                return Some(Self::Property(p))
            }
        }
        CSSParse::css_parse(ps).map(|x| Self::Rule(x))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Rule {
    Unknown(Vec<TokenTree>),
    Style(rule::StyleRule),
    Import(import::ImportRule),
    Media(media::MediaRule),
    FontFace(font_face::FontFaceRule),
    Keyframes(keyframe::KeyframesRule),
    UnknownAtRule(AtKeyword, Vec<TokenTree>),
}

impl CSSParse for Rule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let ret = match ps.peek()? {
            TokenTree::AtKeyword(at_keyword) => {
                match at_keyword.content.as_str() {
                    "import" => Self::Import(CSSParse::css_parse(ps)?),
                    "media" => Self::Media(CSSParse::css_parse(ps)?),
                    "font-face" => Self::FontFace(CSSParse::css_parse(ps)?),
                    "keyframes" => Self::Keyframes(CSSParse::css_parse(ps)?),
                    _ => {
                        let Some(TokenTree::AtKeyword(at_keyword)) = ps.next() else { unreachable!() };
                        let mut tt = vec![];
                        while let Some(next) = ps.next() {
                            let ended = match &next {
                                TokenTree::Semicolon(..) | TokenTree::Brace(..) => true,
                                _ => false,
                            };
                            tt.push(next);
                            if ended { break };
                        }
                        Self::UnknownAtRule(at_keyword, tt)
                    }
                }
            }
            TokenTree::Ident(..)
            | TokenTree::IDHash(..)
            | TokenTree::Colon(..)
            | TokenTree::Bracket(..) => {
                Self::Style(CSSParse::css_parse(ps)?)
            }
            TokenTree::Operator(op) if op.is(".") => {
                Self::Style(CSSParse::css_parse(ps)?)
            }
            _ => {
                let mut tt = vec![];
                while let Some(next) = ps.next() {
                    let ended = match &next {
                        TokenTree::Semicolon(..) | TokenTree::Brace(..) => true,
                        _ => false,
                    };
                    tt.push(next);
                    if ended { break };
                }
                Self::Unknown(tt)
            }
        };
        Some(ret)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Repeat<C, S> {
    pub(crate) items: Vec<(C, Option<S>)>,
}

impl<C: CSSParse, S: TokenExt> CSSParse for Repeat<C, S> {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let mut items = vec![];
        while let Some(c) = C::css_parse(ps) {
            let s = S::css_parse(ps);
            let ended = s.is_none();
            items.push((c, s));
            if ended { break; }
        }
        Some(Self { items })
    }
}

impl<C, S> Repeat<C, S> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &C> {
        self.items.iter().map(|(c, _)| c)
    }

    pub(crate) fn iter_items(&self) -> impl Iterator<Item = (&C, Option<&S>)> {
        self.items.iter().map(|(c, s)| (c, s.as_ref()))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MaybeUnknown<T> {
    Unknown(Vec<TokenTree>),
    Normal(T, Vec<TokenTree>),
}

impl<T: CSSParse> MaybeUnknown<T> {
    fn parse_with_trailing(
        ps: &mut ParseState,
        trailing_f: impl for<'a, 'b, 'c, 'd> FnOnce(&'a mut ParseState<'b, 'c, 'd>) -> Vec<TokenTree>,
    ) -> Self {
        if let Some(t) = T::css_parse(ps) {
            let trailing = trailing_f(ps);
            Self::Normal(t, trailing)
        } else {
            Self::Unknown(trailing_f(ps))
        }
    }
}

mod state {
    use compact_str::CompactString;
    use cssparser::{BasicParseErrorKind, Token as CSSToken};

    use super::*;

    pub(super) struct ParseStateOwned {
        src: String,
        warnings: Vec<ParseError>,
        comments: Vec<Comment>,
    }

    impl ParseStateOwned {
        pub(super) fn new(src: String) -> Self {
            Self {
                src,
                warnings: vec![],
                comments: vec![],
            }
        }

        pub(super) fn run<'s>(&'s mut self, f: impl for<'a, 'b, 'c> FnOnce(&mut ParseState<'a, 'b, 'c>)) {
            let Self { src, warnings, comments, .. } = self;
            let mut input = cssparser::ParserInput::new(src);
            let mut parser = cssparser::Parser::<'s, '_>::new(&mut input);
            let _ = parser.parse_entirely::<_, _, ()>(|parser| {
                let mut ps = ParseState {
                    parser,
                    warnings,
                    comments,
                };
                f(&mut ps);
                while ps.next().is_some() {
                    // empty
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

    fn parser_position(parser: &cssparser::Parser<'_, '_>) -> Position {
        let p = parser.current_source_location();
        Position {
            line: p.line,
            utf16_col: p.column - 1,
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
                let left = location.end..location.end;
                let right = location.end..location.end;
                let name = CompactString::new(name);
                TokenTree::Function(Function { name, children: vec![], left, right, trailing: vec![] })
            }
            CSSToken::ParenthesisBlock => {
                TokenTree::Paren(Paren::new_empty(location))
            }
            CSSToken::SquareBracketBlock => {
                TokenTree::Bracket(Bracket::new_empty(location))
            }
            CSSToken::CurlyBracketBlock => {
                TokenTree::Brace(Brace::new_empty(location))
            }
            CSSToken::CloseParenthesis => {
                TokenTree::BadOperator(Operator::new(")", location))
            }
            CSSToken::CloseSquareBracket => {
                TokenTree::BadOperator(Operator::new("]", location))
            }
            CSSToken::CloseCurlyBracket => {
                TokenTree::BadOperator(Operator::new("}", location))
            }
            CSSToken::WhiteSpace(..)
            | CSSToken::Comment(..) => unreachable!(),
        }
    }

    pub(super) struct ParseState<'a, 'b, 'c> {
        parser: &'c mut cssparser::Parser<'a, 'b>,
        warnings: &'c mut Vec<ParseError>,
        comments: &'c mut Vec<Comment>,
    }

    impl<'a, 'b, 'c> ParseState<'a, 'b, 'c> {
        pub(super) fn add_warning(&mut self, kind: ParseErrorKind, location: Location) {
            self.warnings.push(ParseError {
                kind,
                location,
            })
        }

        #[allow(dead_code)]
        pub(super) fn add_warning_at_current_position(&mut self, kind: ParseErrorKind) {
            let pos = self.position();
            self.add_warning(kind, pos..pos);
        }

        pub(super) fn position(&self) -> Position {
            parser_position(&self.parser)
        }

        /// Get the next non-comment token and advance the cursor.
        /// 
        /// It will collect comments and return the next `TokenTree` if not ended.
        pub(super) fn next(&mut self) -> Option<TokenTree> {
            fn rec<'a, 'b>(
                parser: &mut cssparser::Parser<'a, 'b>,
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
                                x => {
                                    let ret = convert_css_token(&x, location);
                                    match &ret {
                                        TokenTree::BadString(x) => {
                                            let location = x.location();
                                            warnings.push(ParseError { kind: ParseErrorKind::BadString, location });
                                        }
                                        TokenTree::BadUrl(x) => {
                                            let location = x.location();
                                            warnings.push(ParseError { kind: ParseErrorKind::BadUrl, location });
                                        }
                                        TokenTree::BadOperator(x) => {
                                            let location = x.location();
                                            warnings.push(ParseError { kind: ParseErrorKind::UnexpectedToken, location });
                                        }
                                        _ => {}
                                    }
                                    ret
                                },
                            };
                            if token.children().is_some() {
                                let mut children = vec![];
                                let _ = parser.parse_nested_block::<_, _, ()>(|parser| {
                                    while let Some(token) = rec(parser, warnings, comments) {
                                        children.push(token);
                                    }
                                    Ok(())
                                });
                                let right_pos = parser_position(&parser);
                                let location = right_pos..end_pos;
                                match &mut token {
                                    TokenTree::Function(x) => {
                                        x.children = children;
                                        x.right = location;
                                    }
                                    TokenTree::Paren(x) => {
                                        x.children = children;
                                        x.right = location;
                                    }
                                    TokenTree::Bracket(x) => {
                                        x.children = children;
                                        x.right = location;
                                    }
                                    TokenTree::Brace(x) => {
                                        x.children = children;
                                        x.right = location;
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

        /// Get the next non-comment token.
        /// 
        /// It will not collect any comment.
        /// Note that it will not parse any child inside paren, brackets, brace, and function.
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

        /// Get the next token, returning `None` if it is whitespace or comment.
        /// 
        /// Note that it will not parse any child inside paren, brackets, brace, and function.
        pub(super) fn peek_with_whitespace(&mut self) -> Option<TokenTree> {
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

        pub(super) fn peek2(&mut self) -> Option<(TokenTree, TokenTree)> {
            let state = self.parser.state();
            let start_pos = parser_position(&self.parser);
            let next = self.parser.next();
            let ret = match next {
                Err(_) => None,
                Ok(next) => {
                    let ret1 = convert_css_token(next, start_pos..start_pos);
                    if ret1.children().is_some() {
                        None
                    } else {
                        let next = self.parser.next();
                        match next {
                            Err(_) => None,
                            Ok(next) => Some((ret1, convert_css_token(next, start_pos..start_pos))),
                        }
                    }
                }
            };
            self.parser.reset(&state);
            ret
        }

        pub(super) fn skip_until_before(&mut self, mut f: impl FnMut(&TokenTree) -> bool) -> Vec<TokenTree> {
            let mut ret = vec![];
            while let Some(peek) = self.peek() {
                if !f(&peek) {
                    break
                }
                ret.push(self.next().unwrap());
            }
            ret
        }

        pub(super) fn skip_until_before_semicolon(&mut self) -> Vec<TokenTree> {
            self.skip_until_before(|peek| {
                match peek {
                    TokenTree::Semicolon(_) => false,
                    _ => true,
                }
            })
        }

        pub(super) fn skip_until_before_brace_or_semicolon(&mut self) -> Vec<TokenTree> {
            self.skip_until_before(|peek| {
                match peek {
                    TokenTree::Semicolon(_) | TokenTree::Brace(_) => false,
                    _ => true,
                }
            })
        }

        fn parse_nested<R>(
            &mut self,
            reset_state: cssparser::ParserState,
            f: impl FnOnce(&mut ParseState) -> Option<R>,
        ) -> Option<(R, Vec<TokenTree>, Location)> {
            let Self { parser, warnings, comments } = self;
            let ret = parser.parse_nested_block::<_, _, ()>(|parser| {
                let mut ps = ParseState {
                    parser,
                    warnings,
                    comments,
                };
                let Some(r) = f(&mut ps) else {
                    return Err(ps.parser.new_error_for_next_token());
                };
                let mut trailing = vec![];
                while let Some(next) = ps.next() {
                    trailing.push(next);
                }
                Ok((r, trailing, ps.position()))
            });
            let Ok((r, trailing, pos)) = ret else {
                parser.reset(&reset_state);
                return None;
            };
            Some((r, trailing, pos..self.position()))
        }

        pub(super) fn parse_function<R>(&mut self, f: impl FnOnce(&mut ParseState) -> Option<R>) -> Option<Function<R>> {
            match self.peek() {
                Some(TokenTree::Function(..)) => {}
                _ => return None,
            }
            let state = self.parser.state();
            let start_pos = self.position();
            let Ok(CSSToken::Function(name_str)) = self.parser.next().cloned() else { unreachable!() };
            let name_end_pos = self.position();
            let left = start_pos..name_end_pos;
            let name = CompactString::new(name_str);
            let (children, trailing, right) = self.parse_nested(state, f)?;
            if right.is_empty() {
                self.add_warning(ParseErrorKind::UnmatchedParenthesis, left.clone());
            }
            Some(Function { name, children, left, right, trailing })
        }

        pub(super) fn parse_paren<R>(&mut self, f: impl FnOnce(&mut ParseState) -> Option<R>) -> Option<Paren<R>> {
            match self.peek() {
                Some(TokenTree::Paren(..)) => {}
                _ => return None,
            }
            let state = self.parser.state();
            let start_pos = self.position();
            let _ = self.parser.next();
            let name_end_pos = self.position();
            let left = start_pos..name_end_pos;
            let (children, trailing, right) = self.parse_nested(state, f)?;
            if right.is_empty() {
                self.add_warning(ParseErrorKind::UnmatchedParenthesis, left.clone());
            }
            Some(Paren { children, left, right, trailing })
        }

        pub(super) fn parse_bracket<R>(&mut self, f: impl FnOnce(&mut ParseState) -> Option<R>) -> Option<Bracket<R>> {
            match self.peek() {
                Some(TokenTree::Bracket(..)) => {}
                _ => return None,
            }
            let state = self.parser.state();
            let start_pos = self.position();
            let _ = self.parser.next();
            let name_end_pos = self.position();
            let left = start_pos..name_end_pos;
            let (children, trailing, right) = self.parse_nested(state, f)?;
            if right.is_empty() {
                self.add_warning(ParseErrorKind::UnmatchedBracket, left.clone());
            }
            Some(Bracket { children, left, right, trailing })
        }

        pub(super) fn parse_brace<R>(&mut self, f: impl FnOnce(&mut ParseState) -> Option<R>) -> Option<Brace<R>> {
            match self.peek() {
                Some(TokenTree::Brace(..)) => {}
                _ => return None,
            }
            let state = self.parser.state();
            let start_pos = self.position();
            let _ = self.parser.next();
            let name_end_pos = self.position();
            let left = start_pos..name_end_pos;
            let (children, trailing, right) = self.parse_nested(state, f)?;
            if right.is_empty() {
                self.add_warning(ParseErrorKind::UnmatchedBrace, left.clone());
            }
            Some(Brace { children, left, right, trailing })
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
}

#[repr(u32)]
#[derive(Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken = 0x10001,
    BadString,
    BadUrl,
    UnmatchedBrace,
    UnmatchedBracket,
    UnmatchedParenthesis,
}

impl ParseErrorKind {
    fn static_message(&self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected token",
            Self::BadString => "invalid string",
            Self::BadUrl => "invalid URL",
            Self::UnmatchedBrace => "unmatched curly bracket",
            Self::UnmatchedBracket => "unmatched square bracket",
            Self::UnmatchedParenthesis => "unmatched parenthesis",
        }
    }

    pub fn level(&self) -> ParseErrorLevel {
        match self {
            Self::UnexpectedToken => ParseErrorLevel::Fatal,
            Self::BadString => ParseErrorLevel::Error,
            Self::BadUrl => ParseErrorLevel::Error,
            Self::UnmatchedBrace => ParseErrorLevel::Error,
            Self::UnmatchedBracket => ParseErrorLevel::Error,
            Self::UnmatchedParenthesis => ParseErrorLevel::Error,
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
