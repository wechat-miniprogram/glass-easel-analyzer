use super::*;

pub(crate) struct Property {
    pub(crate) name: Ident,
    pub(crate) colon: Colon,
    pub(crate) value: Vec<TokenTree>,
    pub(crate) semicolon: Option<Semicolon>,
}

impl CSSParse for Property {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let name = CSSParse::css_parse(ps)?;
        let colon = CSSParse::css_parse(ps)?;
        let value = ps.skip_until_before_semicolon();
        let semicolon = CSSParse::css_parse(ps);
        Some(Self { name, colon, value, semicolon })
    }
}
