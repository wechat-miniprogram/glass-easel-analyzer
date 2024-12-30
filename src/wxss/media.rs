use super::*;

pub(crate) struct MediaRule {
    at_media: AtKeyword,
    list: MediaQueryList,
    body: Brace<Vec<Rule>>,
}

impl CSSParse for MediaRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_media = CSSParse::css_parse(ps)?;
        let list = CSSParse::css_parse(ps).unwrap_or_else(|| MediaQueryList::Unknown(vec![]));
        let body = CSSParse::css_parse(ps)?;
        Some(Self {
            at_media,
            list,
            body,
        })
    }
}

pub(crate) enum MediaQueryList {
    Paren(Paren<Box<MediaQueryList>>),
    And(Vec<(MediaQueryList, MediaAndKeyword)>),
    Or(Vec<(MediaQueryList, MediaOrKeyword)>),
    Not(Box<MediaQueryList>),
    Only(Box<MediaQueryList>),
    MediaType(MediaType),
    MediaFeature(MediaFeature),
    Unknown(Vec<TokenTree>),
}

impl CSSParse for MediaQueryList {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!()
    }
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

pub(crate) enum MediaType {
    Unknown(Ident),
    Screen(Ident),
    Print(Ident),
}

pub(crate) enum MediaFeature {
    Unknown(Vec<TokenTree>),
    Condition(Ident, Colon, Vec<TokenTree>),
}
