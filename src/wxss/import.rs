use super::{*, media::MediaQueryList};

#[derive(Debug, Clone)]
pub(crate) struct ImportRule {
    pub(crate) at_import: AtKeyword,
    pub(crate) url: MaybeUnknown<QuotedString>,
    pub(crate) condition: Vec<TokenTree>,
    pub(crate) semicolon: Option<Semicolon>,
}

impl CSSParse for ImportRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_import = CSSParse::css_parse(ps)?;
        let url = match ps.peek() {
            Some(TokenTree::QuotedString(_)) => {
                MaybeUnknown::Normal(CSSParse::css_parse(ps)?, vec![])
            }
            _ => {
                MaybeUnknown::Unknown(ps.skip_until_before_semicolon())
            }
        };
        let condition = ps.skip_until_before_semicolon();
        let semicolon = CSSParse::css_parse(ps);
        Some(Self { at_import, url, condition, semicolon })
    }
}
