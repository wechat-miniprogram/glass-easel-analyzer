use super::*;

pub(crate) struct Property {
    name: Ident,
    colon: Colon,
    value: Vec<TokenTree>,
    semicolon: Option<Semicolon>,
}

impl CSSParse for Property {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let name = CSSParse::css_parse(ps)?;
        let colon = CSSParse::css_parse(ps)?;
        let value = ps.skip_until_before_semicolon();
        let mut semicolon = CSSParse::css_parse(ps);
        Some(Self { name, colon, value, semicolon })
    }
}
