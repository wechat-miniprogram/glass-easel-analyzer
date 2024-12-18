use super::{media::MediaQueryList, token::{AtKeyword, QuotedString, TokenTree}};

pub(crate) struct ImportRule {
    at_import: AtKeyword,
    url: QuotedString,
    cond: ImportCondition,
}

pub(crate) enum ImportCondition {
    Media(MediaQueryList),
    Unknown(Vec<TokenTree>),
}
