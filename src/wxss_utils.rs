use std::ops::Range;

use crate::wxss::{token::{AtKeyword, BadString, BadUrl, Hash, IDHash, Ident, QuotedString, UnquotedUrl}, Position, StyleSheet};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Token<'a> {
    None,
    Ident(&'a Ident),
    AtKeyword(&'a AtKeyword),
    Hash(&'a Hash),
    IDHash(&'a IDHash),
    QuotedString(&'a QuotedString),
    UnquotedUrl(&'a UnquotedUrl),
    BadUrl(&'a BadUrl),
    BadString(&'a BadString),
}

pub(crate) fn find_token_in_position(sheet: &StyleSheet, pos: Position) {
    todo!()
}
