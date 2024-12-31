use super::*;

pub(crate) struct FontFaceRule {
    at_font_face: AtKeyword,
    body: Option<BraceOrSemicolon<Vec<RuleOrProperty>>>,
}

impl CSSParse for FontFaceRule {
    fn css_parse(ps: &mut ParseState) -> Option<Self> {
        let at_font_face = CSSParse::css_parse(ps)?;
        let body = CSSParse::css_parse(ps);
        Some(Self { at_font_face, body })
    }
}
