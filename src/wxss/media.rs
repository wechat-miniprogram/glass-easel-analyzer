use super::{token::{AtKeyword, Brace, Ident, Paren, TokenTree}, Item};

pub(crate) struct MediaRule {
    at_media: AtKeyword,
    list: MediaQueryList,
    body: Brace<Vec<Item>>,
}

pub(crate) enum MediaQueryList {
    Paren(Paren<Box<MediaQueryList>>),
    And(Vec<(MediaQueryList, MediaAndKeyword)>),
    Or(Vec<(MediaQueryList, MediaOrKeyword)>),
    Not(Box<MediaQueryList>),
    Only(Box<MediaQueryList>),
    Unknown(Vec<TokenTree>),
}

pub(crate) enum MediaAndKeyword {
    None,
    And(Ident),
}

pub(crate) enum MediaOrKeyword {
    None,
    Or(Ident),
    Comma(Ident),
}
