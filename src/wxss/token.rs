use compact_str::CompactString;

use super::{CSSParse, Location};

#[derive(Debug, Clone)]
pub(crate) enum TokenTree {
    Ident(Ident),
    AtKeyword(AtKeyword),
    Hash(Hash),
    IDHash(IDHash),
    QuotedString(QuotedString),
    UnquotedUrl(UnquotedUrl),
    Number(Number),
    Percentage(Percentage),
    Dimension(Dimension),
    Colon(Colon),
    Semicolon(Semicolon),
    Comma(Comma),
    Operator(Operator),
    Function(Function<Vec<TokenTree>>),
    Paren(Paren<Vec<TokenTree>>),
    Bracket(Bracket<Vec<TokenTree>>),
    Brace(Brace<Vec<TokenTree>>),
    BadUrl(BadUrl),
    BadString(BadString),
    BadOperator(Operator),
}

impl TokenTree {
    pub(crate) fn children(&self) -> Option<&Vec<TokenTree>> {
        match self {
            Self::Ident(..)
            | Self::AtKeyword(..)
            | Self::Hash(..)
            | Self::IDHash(..)
            | Self::QuotedString(..)
            | Self::UnquotedUrl(..)
            | Self::Number(..)
            | Self::Percentage(..)
            | Self::Dimension(..)
            | Self::Colon(..)
            | Self::Semicolon(..)
            | Self::Comma(..)
            | Self::Operator(..)
            | Self::BadUrl(..)
            | Self::BadString(..)
            | Self::BadOperator(..) => None,
            Self::Function(x) => Some(&x.children),
            Self::Paren(x) => Some(&x.children),
            Self::Bracket(x) => Some(&x.children),
            Self::Brace(x) => Some(&x.children),
        }
    }

    pub(crate) fn is_keyword(&self, is: &str) -> bool {
        if let Self::Ident(x) = self {
            x.content == is
        } else {
            false
        }
    }

    pub(crate) fn is_ident(&self) -> bool {
        if let Self::Ident(_) = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_function(&self) -> bool {
        if let Self::Function(_) = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_ident_or_function(&self) -> bool {
        self.is_ident() || self.is_function()
    }
}

impl TokenExt for TokenTree {}

impl CSSParse for TokenTree {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        ps.next()
    }

    fn location(&self) -> Location {
        match self {
            Self::Ident(x) => x.location(),
            Self::AtKeyword(x) => x.location(),
            Self::Hash(x) => x.location(),
            Self::IDHash(x) => x.location(),
            Self::QuotedString(x) => x.location(),
            Self::UnquotedUrl(x) => x.location(),
            Self::Number(x) => x.location(),
            Self::Percentage(x) => x.location(),
            Self::Dimension(x) => x.location(),
            Self::Colon(x) => x.location(),
            Self::Semicolon(x) => x.location(),
            Self::Comma(x) => x.location(),
            Self::Operator(x) => x.location(),
            Self::Function(x) => x.location(),
            Self::Paren(x) => x.location(),
            Self::Bracket(x) => x.location(),
            Self::Brace(x) => x.location(),
            Self::BadUrl(x) => x.location(),
            Self::BadString(x) => x.location(),
            Self::BadOperator(x) => x.location(),
        }
    }
}

pub(crate) trait TokenExt: CSSParse {}

macro_rules! basic_token {
    ($t:ident) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $t {
            pub(crate) content: CompactString,
            pub(crate) location: Location,
        }

        impl TokenExt for $t {}

        impl CSSParse for $t {
            fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
                if let Some(TokenTree::$t(..)) = ps.peek() {
                    let Some(TokenTree::$t(x)) = ps.next() else { unreachable!() };
                    Some(x)
                } else {
                    None
                }
            }

            fn location(&self) -> Location {
                self.location.clone()
            }
        }
    };
}

basic_token!(Ident);
basic_token!(AtKeyword);
basic_token!(Hash);
basic_token!(IDHash);
basic_token!(QuotedString);
basic_token!(UnquotedUrl);
basic_token!(BadUrl);
basic_token!(BadString);

macro_rules! core_delim_token {
    ($t:ident) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $t {
            pub(crate) location: Location,
        }

        impl TokenExt for $t {}

        impl CSSParse for $t {
            fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
                if let Some(TokenTree::$t(..)) = ps.peek() {
                    let Some(TokenTree::$t(x)) = ps.next() else { unreachable!() };
                    Some(x)
                } else {
                    None
                }
            }

            fn location(&self) -> Location {
                self.location.clone()
            }
        }
    };
}

core_delim_token!(Colon);
core_delim_token!(Semicolon);
core_delim_token!(Comma);

#[derive(Debug, Clone)]
pub(crate) struct Operator {
    pub(crate) name: [u8; 4],
    pub(crate) location: Location,
}

impl Operator {
    pub(crate) fn new<S: ?Sized + AsRef<str>>(s: &S, location: Location) -> Self {
        let b = s.as_ref().as_bytes();
        assert!(b.len() <= 4);
        let mut name = [0u8; 4];
        for i in 0..4 {
            name[i] = b.get(i).cloned().unwrap_or(0);
        }
        Self { name, location }
    }

    pub(crate) fn is(&self, s: &str) -> bool {
        let b = s.as_bytes();
        if b.len() > 4 {
            return false;
        }
        for i in 0..4 {
            if b.get(i).cloned().unwrap_or(0) != self.name[i] {
                return false;
            }
        }
        true
    }
}

impl TokenExt for Operator {}

impl CSSParse for Operator {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        if let Some(TokenTree::Operator(..)) = ps.peek() {
            let Some(TokenTree::Operator(x)) = ps.next() else { unreachable!() };
            Some(x)
        } else {
            None
        }
    }

    fn location(&self) -> Location {
        self.location.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Number {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) location: Location,
}

impl TokenExt for Number {}

impl CSSParse for Number {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        if let Some(TokenTree::Number(..)) = ps.peek() {
            let Some(TokenTree::Number(x)) = ps.next() else { unreachable!() };
            Some(x)
        } else {
            None
        }
    }

    fn location(&self) -> Location {
        self.location.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Percentage {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) location: Location,
}

impl TokenExt for Percentage {}

impl CSSParse for Percentage {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        if let Some(TokenTree::Percentage(..)) = ps.peek() {
            let Some(TokenTree::Percentage(x)) = ps.next() else { unreachable!() };
            Some(x)
        } else {
            None
        }
    }

    fn location(&self) -> Location {
        self.location.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Dimension {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) unit: CompactString,
    pub(crate) location: Location,
}

impl TokenExt for Dimension {}

impl CSSParse for Dimension {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        if let Some(TokenTree::Dimension(..)) = ps.peek() {
            let Some(TokenTree::Dimension(x)) = ps.next() else { unreachable!() };
            Some(x)
        } else {
            None
        }
    }

    fn location(&self) -> Location {
        self.location.clone()
    }
}

pub(crate) trait TokenGroupExt<T> {
    fn left(&self) -> Location;
    fn right(&self) -> Location;
    fn children(&self) -> &T;
    fn trailing(&self) -> &[TokenTree];
}

#[derive(Debug, Clone)]
pub(crate) struct Function<T> {
    pub(crate) name: CompactString,
    pub(crate) children: T,
    pub(crate) left: Location,
    pub(crate) right: Location,
    pub(crate) trailing: Vec<TokenTree>,
}

impl<T> Function<T> {
    pub(crate) fn location(&self) -> Location {
        self.left.start..self.right.end
    }
}

impl<T: CSSParse> TokenExt for Function<T> {}

impl<T> TokenGroupExt<T> for Function<T> {
    fn left(&self) -> Location {
        self.left.clone()
    }

    fn right(&self) -> Location {
        self.right.clone()
    }

    fn children(&self) -> &T {
        &self.children
    }

    fn trailing(&self) -> &[TokenTree] {
        &self.trailing
    }
}

impl<T: CSSParse> CSSParse for Function<T> {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        if let Some(TokenTree::Function(..)) = ps.peek() {
            ps.parse_function(|mut ps| {
                CSSParse::css_parse(&mut ps)
            })
        } else {
            None
        }
    }

    fn location(&self) -> Location {
        self.left.start..self.right.end
    }
}

macro_rules! group_token {
    ($t:ident, $p:ident) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $t<T> {
            pub(crate) children: T,
            pub(crate) left: Location,
            pub(crate) right: Location,
            pub(crate) trailing: Vec<TokenTree>,
        }

        impl<T> $t<T> {
            pub(crate) fn location(&self) -> Location {
                self.left.start..self.right.end
            }
        }

        impl<T: Default> $t<T> {
            pub(crate) fn new_empty(left: Location) -> Self {
                let right = left.clone();
                Self {
                    children: T::default(),
                    left,
                    right,
                    trailing: vec![],
                }
            }
        }

        impl<T: CSSParse> TokenExt for $t<T> {}

        impl<T> TokenGroupExt<T> for $t<T> {
            fn left(&self) -> Location {
                self.left.clone()
            }
        
            fn right(&self) -> Location {
                self.right.clone()
            }
        
            fn children(&self) -> &T {
                &self.children
            }

            fn trailing(&self) -> &[TokenTree] {
                &self.trailing
            }
        }

        impl<T: CSSParse> CSSParse for $t<T> {
            fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
                if let Some(TokenTree::$t(..)) = ps.peek() {
                    ps.$p(|mut ps| {
                        CSSParse::css_parse(&mut ps)
                    })
                } else {
                    None
                }
            }

            fn location(&self) -> Location {
                self.left.start..self.right.end
            }
        }
    };
}

group_token!(Paren, parse_paren);
group_token!(Bracket, parse_bracket);
group_token!(Brace, parse_brace);

#[derive(Debug, Clone)]
pub(crate) struct Comment {
    pub(crate) content: CompactString,
    pub(crate) location: Location,
}

#[derive(Debug, Clone)]
pub(crate) enum BraceOrSemicolon<T> {
    Brace(Brace<T>),
    UnknownBrace(Brace<()>),
    Semicolon(Semicolon),
}

impl<T: CSSParse> CSSParse for BraceOrSemicolon<T> {
    fn css_parse(ps: &mut super::state::ParseState) -> Option<Self> {
        let ret = match ps.peek()? {
            TokenTree::Brace(_) => {
                match Brace::<T>::css_parse(ps) {
                    None => Self::UnknownBrace(ps.parse_brace(|_| Some(())).unwrap()),
                    Some(x) => Self::Brace(x),
                }
            },
            TokenTree::Semicolon(_) => Self::Semicolon(CSSParse::css_parse(ps)?),
            _ => return None
        };
        Some(ret)
    }

    fn location(&self) -> Location {
        match self {
            Self::Brace(x) => x.location(),
            Self::UnknownBrace(x) => x.location(),
            Self::Semicolon(x) => x.location(),
        }
    }
}
