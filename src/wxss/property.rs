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
        let mut value = vec![];
        let mut semicolon = None;
        while let Some(tt) = ps.next() {
            if let TokenTree::Semicolon(x) = tt {
                semicolon = Some(x);
                break;
            }
            value.push(tt);
        }
        Some(Self { name, colon, value, semicolon })
    }
}
