use compact_str::CompactString;
use enum_dispatch::enum_dispatch;

use super::Location;

#[enum_dispatch(TokenExt)]
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
    Comment(Comment),
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
}

pub(crate) trait TokenExt {
    fn location(&self) -> Location;
}

macro_rules! basic_token {
    ($t:ident) => {
        pub(crate) struct $t {
            pub(crate) content: CompactString,
            pub(crate) location: Location,
        }

        impl TokenExt for $t {
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
basic_token!(Comment);
basic_token!(BadUrl);
basic_token!(BadString);

macro_rules! core_delim_token {
    ($t:ident) => {
        pub(crate) struct $t {
            pub(crate) location: Location,
        }

        impl TokenExt for $t {
            fn location(&self) -> Location {
                self.location.clone()
            }
        }
    };
}

core_delim_token!(Colon);
core_delim_token!(Semicolon);
core_delim_token!(Comma);

pub(crate) struct Operator {
    pub(crate) name: [u8; 4],
    pub(crate) location: Location,
}

impl Operator {
    pub(crate) fn new(s: &impl AsRef<str>, location: Location) -> Self {
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

pub(crate) struct Number {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) location: Location,
}

impl TokenExt for Number {
    fn location(&self) -> Location {
        self.location.clone()
    }
}

pub(crate) struct Percentage {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) location: Location,
}

impl TokenExt for Percentage {
    fn location(&self) -> Location {
        self.location.clone()
    }
}

pub(crate) struct Dimension {
    pub(crate) has_sign: bool,
    pub(crate) value: f32,
    pub(crate) int_value: Option<i32>,
    pub(crate) unit: CompactString,
    pub(crate) location: Location,
}

impl TokenExt for Dimension {
    fn location(&self) -> Location {
        self.location.clone()
    }
}

pub(crate) trait TokenGroupExt<T> {
    fn children(&self) -> &T;
}

pub(crate) struct Function<T> {
    pub(crate) name: Ident,
    pub(crate) paren: Paren<T>,
}

impl<T> TokenExt for Function<T> {
    fn location(&self) -> Location {
        self.name.location.start..self.paren.location.end
    }
}

impl<T> TokenGroupExt<T> for Function<T> {
    fn children(&self) -> &T {
        &self.paren.children
    }
}

macro_rules! group_token {
    ($t:ident) => {
        pub(crate) struct $t<T> {
            pub(crate) children: T,
            pub(crate) location: Location,
        }

        impl<T> TokenExt for $t<T> {
            fn location(&self) -> Location {
                self.location.clone()
            }
        }

        impl<T> TokenGroupExt<T> for $t<T> {
            fn children(&self) -> &T {
                &self.children
            }
        }
    };
}

group_token!(Paren);
group_token!(Bracket);
group_token!(Brace);
