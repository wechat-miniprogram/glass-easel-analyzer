use super::token::{Colon, Ident, Semicolon, TokenTree};

pub(crate) struct Property {
    name: Ident,
    colon: Colon,
    value: Vec<TokenTree>,
    semicolon: Semicolon,
}
