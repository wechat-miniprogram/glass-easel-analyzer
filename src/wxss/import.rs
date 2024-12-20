use super::{*, media::MediaQueryList};

pub(crate) struct ImportRule {
    at_import: AtKeyword,
    url: QuotedString,
    cond: ImportCondition,
}

pub(crate) enum ImportCondition {
    Media(MediaQueryList),
    Unknown(Vec<TokenTree>),
}

impl CSSParse for ImportRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        todo!() // TODO
    }
}
